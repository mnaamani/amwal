use ledger::{
    LedgerError, LedgerStore,
    domain::{NewTransferInternal, TransferInternalId},
};

pub fn initiate_transfer_internal_<S: LedgerStore>(
    store: &mut S,
    new_transfer: &NewTransferInternal,
) -> Result<TransferInternalId, LedgerError> {
    let _from_account = match store.get_account(new_transfer.from_account_id)? {
        Some(account) if account.active => account,
        Some(account) => return Err(LedgerError::AccountNotActive(account.id)),
        None => return Err(LedgerError::AccountNotFound(new_transfer.from_account_id)),
    };
    let _to_account = match store.get_account(new_transfer.to_account_id)? {
        Some(account) if account.active => account,
        Some(account) => return Err(LedgerError::AccountNotActive(account.id)),
        None => return Err(LedgerError::AccountNotFound(new_transfer.to_account_id)),
    };
    // db transaction:
    //  |- check source account available balance (balance - sum of blocks)
    //  |- create transfer record in pending state
    //  |- create account block for from_account (needs transfer_id ref)
    Err(LedgerError::InvalidTransferState(
        "initiate_transfer not yet implemented".into(),
    ))
}

pub fn complete_transfer_internal<S: LedgerStore>(
    _store: &mut S,
    _transfer_id: TransferInternalId,
) -> Result<(), LedgerError> {
    // verify transfer exists and is in pending state
    // db transaction:
    //  |- verify accounts still exist and are active
    //  |- post journal entry
    //  |- remove account block
    //  |- update transfer: set journal_entry_id, status = completed, updated_at
    Ok(())
}

pub fn cancel_transfer_internal<S: LedgerStore>(
    _store: &mut S,
    _transfer_id: TransferInternalId,
) -> Result<(), LedgerError> {
    // verify transfer exists and is in pending state
    // remove account block
    // update transfer: set status = cancelled, updated_at
    Ok(())
}
