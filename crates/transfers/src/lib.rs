use std::num::NonZeroU64;

use ledger_api::{AccountId, AccountSummary, LedgerClient, LedgerClientError, accounts_compatible};

mod postgres;

pub use postgres::TransferStore;

pub type TransferInternalId = i32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransferStatus {
    Pending,
    /// Intent to cancel has been recorded; release_funds is in progress.
    Cancelling,
    Cancelled,
    /// Intent to complete has been recorded; journal posting is in progress.
    Completing,
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

pub struct TransferRequest {
    pub client_id: String,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    pub amount: i64,
}

#[derive(Debug)]
pub enum TransferError {
    Ledger(LedgerClientError),
    Storage(String),
    InsufficientFunds {
        available: i64,
        requested: i64,
    },
    TransferNotFound,
    TransferNotPending,
    /// complete_transfer was called on a transfer already being cancelled.
    TransferBeingCancelled,
    /// cancel_transfer was called on a transfer already being completed.
    TransferBeingCompleted,
    AmountNotPositive(i64),
    AccountsNotDistinct,
    AccountsNotSameNature,
}

fn validate_transfer_request<L: LedgerClient>(
    ledger: &L,
    request: &TransferRequest,
) -> Result<(), TransferError> {
    let _ = u64::try_from(request.amount)
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or(TransferError::AmountNotPositive(request.amount))?;

    if request.from_account_id == request.to_account_id {
        return Err(TransferError::AccountsNotDistinct);
    }

    let from_account = require_active(ledger, request.from_account_id)?;
    let to_account = require_active(ledger, request.to_account_id)?;

    // Validate both accounts share the same accounting nature upfront so we
    // don't create a transfer that will fail at settlement.
    if !accounts_compatible(from_account.account_type, to_account.account_type) {
        return Err(TransferError::AccountsNotSameNature);
    }

    // Other validation checks..(limits, account restrictions, fraud,
    // security checks (recently added beneficiery, recently changed account password)
    // compliance, unusual activity detection..
    Ok(())
}

fn ensure_available_funds<L: LedgerClient>(
    ledger: &L,
    request: &TransferRequest,
) -> Result<(), TransferError> {
    // If we are retrying the initiate transfer we may have already blocked funds and perhaps available balance
    // no longer sufficient. So we must check if there are funds blocked related to the client_id
    let available = ledger
        .get_available_balance(request.from_account_id)
        .map_err(TransferError::Ledger)?;
    if available < request.amount {
        return Err(TransferError::InsufficientFunds {
            available,
            requested: request.amount,
        });
    }
    Ok(())
}

/// Create a pending transfer and block funds on the sender's account.
pub fn initiate_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    request: &TransferRequest,
) -> Result<Transfer, TransferError> {
    let transfer = match store.find_transfer_by_client_id(&request.client_id) {
        Ok(t) => {
            if TransferStatus::Pending != t.status {
                return Err(TransferError::TransferNotPending);
            }
            Ok(t)
        }
        Err(TransferError::TransferNotFound) => {
            validate_transfer_request(ledger, request)?;
            ensure_available_funds(ledger, request)?;
            store.insert_transfer(
                &request.client_id,
                request.from_account_id,
                request.to_account_id,
                request.amount,
            )
        }
        e => return e,
    }?;

    if let Err(e) = ledger.block_funds(
        &transfer.client_id,
        transfer.from_account_id,
        transfer.amount,
    ) {
        // Set the status failed if we are unable to block funds
        let _ = store.set_transfer_status(transfer.id, TransferStatus::Failed);
        return Err(TransferError::Ledger(e));
    }

    Ok(transfer)
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
///
/// Intent is captured atomically before any ledger write: the transfer is moved
/// to `Completing` first, so a concurrent `cancel_transfer` sees a conflict and
/// returns `TransferBeingCompleted` rather than racing ahead.
pub fn complete_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    client_id: String,
) -> Result<(), TransferError> {
    let transfer = store.find_transfer_by_client_id(&client_id)?;

    // Claim the intent atomically. Returns the effective status after the attempt.
    match store.claim_pending(transfer.id, TransferStatus::Completing)? {
        TransferStatus::Completing => {} // claimed now, or retry from a prior attempt
        TransferStatus::Cancelling => return Err(TransferError::TransferBeingCancelled),
        _ => return Err(TransferError::TransferNotPending),
    }

    // amount was validated positive at initiation time; a non-positive value
    // here means the DB record was corrupted by something outside this service.
    let _ = u64::try_from(transfer.amount)
        .ok()
        .and_then(NonZeroU64::new)
        .expect("transfer amount in DB must be positive — data integrity violation");

    ledger
        .post_transfer(
            &transfer.client_id,
            transfer.from_account_id,
            transfer.to_account_id,
            transfer.amount,
        )
        .map_err(TransferError::Ledger)?;

    ledger
        .release_funds(&transfer.client_id)
        .map_err(TransferError::Ledger)?;

    store.set_transfer_status(transfer.id, TransferStatus::Completed)
}

/// Release the funds block and mark the transfer Cancelled.
///
/// Intent is captured atomically before the ledger write: the transfer is moved
/// to `Cancelling` first, so a concurrent `complete_transfer` sees a conflict
/// and returns `TransferBeingCompleted` rather than racing ahead. Safe to retry.
pub fn cancel_transfer<L: LedgerClient>(
    ledger: &L,
    store: &TransferStore,
    client_id: String,
) -> Result<(), TransferError> {
    let transfer = store.find_transfer_by_client_id(&client_id)?;

    match store.claim_pending(transfer.id, TransferStatus::Cancelling)? {
        TransferStatus::Cancelling => {} // claimed now, or retry from a prior attempt
        TransferStatus::Completing => return Err(TransferError::TransferBeingCompleted),
        _ => return Err(TransferError::TransferNotPending),
    }

    ledger
        .release_funds(&transfer.client_id)
        .map_err(TransferError::Ledger)?;

    store.set_transfer_status(transfer.id, TransferStatus::Cancelled)
}

// ── Reconciliation ────────────────────────────────────────────────────────────

/// Returns all transfers stuck in `Completing` state.
///
/// These had their completion intent recorded but the process crashed before
/// the journal entry, block release, or final status update completed.
/// Each entry can be driven to completion by calling `complete_transfer` again —
/// all ledger operations are idempotent so retrying is always safe.
pub fn find_stuck_completing_transfers(
    store: &TransferStore,
) -> Result<Vec<Transfer>, TransferError> {
    store.find_transfers_by_status(TransferStatus::Completing)
}

/// Returns all transfers stuck in `Cancelling` state.
///
/// These had their cancellation intent recorded but the process crashed before
/// the block release or final status update completed.
/// Each entry can be driven to completion by calling `cancel_transfer` again —
/// `release_funds` is idempotent so retrying is always safe.
pub fn find_stuck_cancelling_transfers(
    store: &TransferStore,
) -> Result<Vec<Transfer>, TransferError> {
    store.find_transfers_by_status(TransferStatus::Cancelling)
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
