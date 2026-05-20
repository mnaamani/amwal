use ledger_api::{AccountId, LedgerClient, LedgerClientError};

mod postgres;

pub use postgres::TransferStore;

pub type TransferInternalId = i32;

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

pub fn initiate_transfer<L: LedgerClient>(
    ledger: &L,
    _store: &TransferStore,
    input: &NewTransferInput,
) -> Result<TransferInternalId, TransferError> {
    let from_account = ledger
        .get_account(input.from_account_id)
        .map_err(TransferError::Ledger)?
        .ok_or(TransferError::Ledger(LedgerClientError::AccountNotFound(input.from_account_id)))?;

    if !from_account.active {
        return Err(TransferError::Ledger(LedgerClientError::AccountNotActive(from_account.id)));
    }

    let to_account = ledger
        .get_account(input.to_account_id)
        .map_err(TransferError::Ledger)?
        .ok_or(TransferError::Ledger(LedgerClientError::AccountNotFound(input.to_account_id)))?;

    if !to_account.active {
        return Err(TransferError::Ledger(LedgerClientError::AccountNotActive(to_account.id)));
    }

    // TODO (store tx):
    //  |- get_account_balance from ledger + sum own account_blocks → check available >= amount
    //  |- insert transfer_internal (Pending) with input.client_id
    //  |- ledger.block_funds(input.client_id, from_account_id, amount)
    Err(TransferError::InvalidState("initiate_transfer not yet implemented".into()))
}

pub fn complete_transfer<L: LedgerClient>(
    _ledger: &L,
    _store: &TransferStore,
    _transfer_id: TransferInternalId,
) -> Result<(), TransferError> {
    // TODO:
    //  |- load transfer, assert Pending
    //  |- assert both accounts still active via ledger
    //  |- ledger.post_journal_entry(transfer.client_id, [Debit from, Credit to])
    //  |- ledger.release_funds(transfer.client_id)
    //  |- update transfer: status = Completed
    Ok(())
}

pub fn cancel_transfer<L: LedgerClient>(
    _ledger: &L,
    _store: &TransferStore,
    _transfer_id: TransferInternalId,
) -> Result<(), TransferError> {
    // TODO:
    //  |- load transfer, assert Pending
    //  |- ledger.release_funds(transfer.client_id)
    //  |- update transfer: status = Cancelled
    Ok(())
}
