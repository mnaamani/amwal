use std::num::NonZeroU64;

use ledger_api::{
    AccountId, AccountSummary, AccountType, JournalLeg, JournalPosting, LedgerClient,
    LedgerClientError,
};

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
    TransferIsNotPending(TransferInternalId),
    InvalidTransferAmount(i64),
    InvalidToAccount(AccountId),
    InvalidInput(String),
    AccountsMustBeSameNature,
}

/// Create a pending transfer and block funds on the sender's account.
pub fn initiate_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    input: &NewTransferInput,
) -> Result<TransferInternalId, TransferError> {
    if input.amount <= 0 {
        return Err(TransferError::InvalidTransferAmount(input.amount));
    }
    if input.from_account_id == input.to_account_id {
        return Err(TransferError::InvalidToAccount(input.to_account_id));
    }

    let from_account = require_active(ledger, input.from_account_id)?;
    let to_account = require_active(ledger, input.to_account_id)?;

    // Validate both accounts share the same accounting nature up front so we
    // don't create a transfer that will fail at settlement.
    check_same_nature(from_account.account_type, to_account.account_type)?;

    let available = ledger
        .get_available_balance(input.from_account_id)
        .map_err(TransferError::Ledger)?;
    if available < input.amount {
        return Err(TransferError::InsufficientFunds {
            available,
            requested: input.amount,
        });
    }

    let transfer = store.insert_transfer(
        &input.client_id,
        input.from_account_id,
        input.to_account_id,
        input.amount,
    )?;

    if let Err(e) = ledger.block_funds(&input.client_id, input.from_account_id, input.amount) {
        // Compensate: mark the transfer Failed so it isn't left as a ghost
        // Pending record. Best-effort — if this also fails the transfer will
        // be cleaned up by reconciliation.
        let _ = store.set_transfer_status(transfer.id, TransferStatus::Failed);
        return Err(TransferError::Ledger(e));
    }

    Ok(transfer.id)
}

/// Post the journal entry, release the block, and mark the transfer Completed.
///
/// Posting direction is determined by account type:
/// - Debit-normal (Asset, Expense): sender gets Credit, receiver gets Debit.
/// - Credit-normal (Liability, Equity, Revenue): sender gets Debit, receiver gets Credit.
///
/// Example — two customer deposit accounts (Liability):
///   DR sender   (decreases liability — bank owes sender less)
///   CR receiver (increases liability — bank owes receiver more)
pub fn complete_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    transfer_id: TransferInternalId,
) -> Result<(), TransferError> {
    let transfer = store.find_transfer(transfer_id)?;
    if transfer.status != TransferStatus::Pending {
        return Err(TransferError::TransferIsNotPending(transfer.id));
    }

    // Re-validate both accounts are still active at settlement time and get their types.
    let from_account = require_active(ledger, transfer.from_account_id)?;
    let to_account = require_active(ledger, transfer.to_account_id)?;
    check_same_nature(from_account.account_type, to_account.account_type)?;

    // This is an invariant check really, amount should have been validated when it was initiated
    let amount = NonZeroU64::new(transfer.amount as u64)
        .ok_or_else(|| TransferError::InvalidTransferAmount(transfer.amount))?;

    let (from_posting, to_posting) = postings_for(from_account.account_type, amount);

    ledger
        .post_journal_entry(
            &transfer.client_id,
            vec![
                JournalLeg {
                    account_id: transfer.from_account_id,
                    posting: from_posting,
                },
                JournalLeg {
                    account_id: transfer.to_account_id,
                    posting: to_posting,
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
        return Err(TransferError::TransferIsNotPending(transfer.id));
    }

    ledger
        .release_funds(&transfer.client_id)
        .map_err(TransferError::Ledger)?;

    store.set_transfer_status(transfer_id, TransferStatus::Cancelled)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn require_active<L: LedgerClient>(
    ledger: &L,
    account_id: AccountId,
) -> Result<AccountSummary, TransferError> {
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
    Ok(account)
}

/// Returns true for accounts whose balance increases on a Debit (Asset, Expense).
fn is_debit_normal(t: AccountType) -> bool {
    matches!(t, AccountType::Asset | AccountType::Expense)
}

/// Transfers are only defined between accounts of the same nature. A transfer
/// between, say, a Liability and an Asset account would require a contra/
/// intermediate account and is out of scope here.
fn check_same_nature(from: AccountType, to: AccountType) -> Result<(), TransferError> {
    if is_debit_normal(from) != is_debit_normal(to) {
        return Err(TransferError::AccountsMustBeSameNature);
    }
    Ok(())
}

/// Returns `(from_posting, to_posting)` such that the sender's balance
/// decreases and the receiver's balance increases.
fn postings_for(account_type: AccountType, amount: NonZeroU64) -> (JournalPosting, JournalPosting) {
    if is_debit_normal(account_type) {
        // Asset/Expense: Credit decreases, Debit increases.
        (
            JournalPosting::Credit(amount),
            JournalPosting::Debit(amount),
        )
    } else {
        // Liability/Equity/Revenue: Debit decreases, Credit increases.
        (
            JournalPosting::Debit(amount),
            JournalPosting::Credit(amount),
        )
    }
}
