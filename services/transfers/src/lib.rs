use diesel::prelude::*;
use ledger::{
    errors::LedgerError,
    get_account,
    models::{NewTransferInternal, TransferInternalId},
};

// Transfer lifecycle: initiate, complete, cancel

pub fn initiate_transfer_internal_(
    connection: &mut PgConnection,
    new_transfer: &NewTransferInternal,
) -> Result<TransferInternalId, LedgerError> {
    // - verify source and destinations exists and are active
    let from_account = match get_account(connection, new_transfer.from_account_id) {
        Ok(Some(account)) => {
            if account.active {
                account
            } else {
                return Err(LedgerError::AccountNotActive(account.id));
            }
        }
        Ok(None) => return Err(LedgerError::AccountNotFound(new_transfer.from_account_id)),
        Err(e) => return Err(e),
    };
    let to_account = match get_account(connection, new_transfer.to_account_id) {
        Ok(Some(account)) => {
            if account.active {
                account
            } else {
                return Err(LedgerError::AccountNotActive(account.id));
            }
        }
        Ok(None) => return Err(LedgerError::AccountNotFound(new_transfer.to_account_id)),
        Err(e) => return Err(e),
    };
    // db transaction
    //  |- check source account balance available (balance minus all block amounts)
    //  |- create transfer in pending state
    //  |- block funds of from_account (needs transfer_id ref)
    Err(LedgerError::GeneralError(
        "Unable to initiate transfer".into(),
    ))
}

pub fn complete_transfer_internal(
    connection: &mut PgConnection,
    transfer_id: TransferInternalId,
) -> Result<(), LedgerError> {
    // verify transfer id exists and is in pending state
    // db transaction
    //  |- verify accounts still exist and are active
    //  |- post journal_entry
    //  |- removed account block
    //  |- update transfer record with journal entry and set status to completed, updated_at
    Ok(())
}

pub fn cancel_transfer_internal(
    connection: &mut PgConnection,
    transfer_id: TransferInternalId,
) -> Result<(), LedgerError> {
    // verify transfer id exists and is in pending state
    // remove account block
    // update transfer record set status cancelled and updated_at
    Ok(())
}
