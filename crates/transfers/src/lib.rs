use std::num::NonZeroU64;

use ledger_api::{AccountId, JournalLeg, JournalPosting, LedgerClient, LedgerClientError};

mod postgres;

pub use postgres::TransferStore;

pub type TransferInternalId = i32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransferStatus {
    Pending,
    Cancelled,
    Completed,
    Failed,
}

pub struct Transfer {
    pub id: TransferInternalId,
    pub client_id: String,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
    pub status: TransferStatus,
}

pub struct NewTransferInput {
    pub client_id: String,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
}

#[derive(Debug)]
pub enum TransferError {
    Ledger(LedgerClientError),
    Storage(String),
    InsufficientFunds { available: i64, requested: i64 },
    TransferNotFound(TransferInternalId),
    InvalidState(String),
}

/// Create a pending transfer and block funds on the sender's account.
///
/// The balance check uses the ledger's posted balance and does not account for
/// other in-flight blocks — callers should be aware of this race window until
/// a dedicated `get_available_balance` is added to `LedgerClient`.
pub fn initiate_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    input: &NewTransferInput,
) -> Result<TransferInternalId, TransferError> {
    if input.amount <= 0 {
        return Err(TransferError::InvalidState(
            "amount must be positive".into(),
        ));
    }
    if input.from_account_id == input.to_account_id {
        return Err(TransferError::InvalidState(
            "from and to accounts must differ".into(),
        ));
    }

    // Validate both accounts exist and are active.
    for account_id in [input.from_account_id, input.to_account_id] {
        let account = ledger
            .get_account(account_id)
            .map_err(TransferError::Ledger)?
            .ok_or(TransferError::Ledger(LedgerClientError::AccountNotFound(
                account_id,
            )))?;
        if !account.active {
            return Err(TransferError::Ledger(LedgerClientError::AccountNotActive(
                account_id,
            )));
        }
    }

    // Check the sender's posted balance.
    let balance = ledger
        .get_account_balance(input.from_account_id)
        .map_err(TransferError::Ledger)?;
    if balance < input.amount {
        return Err(TransferError::InsufficientFunds {
            available: balance,
            requested: input.amount,
        });
    }

    let transfer = store.insert_transfer(
        &input.client_id,
        input.from_account_id,
        input.to_account_id,
        input.amount,
    )?;

    ledger
        .block_funds(&input.client_id, input.from_account_id, input.amount)
        .map_err(TransferError::Ledger)?;

    Ok(transfer.id)
}

/// Post the journal entry, release the block, and mark the transfer Completed.
///
/// The journal entry Debits the receiver (increases their balance) and Credits
/// the sender (decreases their balance) — correct for asset accounts.
pub fn complete_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    transfer_id: TransferInternalId,
) -> Result<(), TransferError> {
    let transfer = store.find_transfer(transfer_id)?;
    if transfer.status != TransferStatus::Pending {
        return Err(TransferError::InvalidState(format!(
            "transfer {transfer_id} is not pending"
        )));
    }

    // Both accounts must still be active at settlement time.
    for account_id in [transfer.from_account_id, transfer.to_account_id] {
        let account = ledger
            .get_account(account_id)
            .map_err(TransferError::Ledger)?
            .ok_or(TransferError::Ledger(LedgerClientError::AccountNotFound(
                account_id,
            )))?;
        if !account.active {
            return Err(TransferError::Ledger(LedgerClientError::AccountNotActive(
                account_id,
            )));
        }
    }

    let amount = NonZeroU64::new(transfer.amount as u64)
        .ok_or_else(|| TransferError::InvalidState("transfer amount must be positive".into()))?;

    ledger
        .post_journal_entry(
            &transfer.client_id,
            vec![
                JournalLeg {
                    account_id: transfer.to_account_id,
                    posting: JournalPosting::Debit(amount),
                },
                JournalLeg {
                    account_id: transfer.from_account_id,
                    posting: JournalPosting::Credit(amount),
                },
            ],
        )
        .map_err(TransferError::Ledger)?;

    ledger
        .release_funds(&transfer.client_id)
        .map_err(TransferError::Ledger)?;

    store.set_transfer_status(transfer_id, TransferStatus::Completed)
}

/// Release the funds block and mark the transfer Cancelled.
pub fn cancel_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    transfer_id: TransferInternalId,
) -> Result<(), TransferError> {
    let transfer = store.find_transfer(transfer_id)?;
    if transfer.status != TransferStatus::Pending {
        return Err(TransferError::InvalidState(format!(
            "transfer {transfer_id} is not pending"
        )));
    }

    ledger
        .release_funds(&transfer.client_id)
        .map_err(TransferError::Ledger)?;

    store.set_transfer_status(transfer_id, TransferStatus::Cancelled)
}
