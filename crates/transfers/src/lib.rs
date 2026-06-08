use std::num::NonZeroU64;

use ledger_api::{AccountId, AccountSummary, LedgerClient, LedgerClientError, accounts_compatible};

mod postgres;

pub use postgres::TransferStore;

/// Internal identifier for a transfer record. Opaque to callers.
pub type TransferInternalId = i64;

/// The lifecycle state of a transfer.
///
/// State machine (happy path and cancellation path):
///
/// ```text
///  initiate_transfer
///        │
///        ▼
///     Pending ──── complete_transfer ──▶ Completing ──▶ Completed
///        │
///        ├────────── cancel_transfer ──▶ Cancelling ──▶ Cancelled
///        │
///        └────────── block_funds failure ──▶ Failed
/// ```
///
/// The intermediate states (`Completing`, `Cancelling`) capture intent
/// atomically before any ledger write, so concurrent calls to
/// `complete_transfer` and `cancel_transfer` cannot both proceed.
/// Stuck transfers in either intermediate state are safe to retry.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransferStatus {
    /// Funds are blocked; waiting for completion or cancellation.
    Pending,
    /// Intent to cancel recorded; `release_funds` in progress.
    Cancelling,
    /// Block released; transfer voided.
    Cancelled,
    /// Intent to complete recorded; journal posting in progress.
    Completing,
    /// Journal entry posted and block released.
    Completed,
    /// Non-retryable failure (e.g. could not block funds at initiation).
    Failed,
}

/// A transfer record as stored in the transfer service's own database.
pub struct Transfer {
    pub id: TransferInternalId,
    /// Caller-supplied idempotency key — identifies this transfer uniquely
    /// across retries.
    pub client_id: String,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    /// Amount in minor units.
    pub amount: i64,
    pub status: TransferStatus,
}

/// Input for initiating a new transfer.
pub struct TransferRequest {
    /// Idempotency key. A second call with the same `client_id` returns the
    /// existing transfer rather than creating a new one.
    pub client_id: String,
    pub from_account_id: AccountId,
    pub to_account_id: AccountId,
    /// Amount in minor units. Must be positive and non-zero.
    pub amount: i64,
}

impl From<LedgerClientError> for TransferError {
    fn from(e: LedgerClientError) -> Self {
        TransferError::Ledger(e)
    }
}

/// Errors returned by transfer operations.
#[derive(Debug)]
pub enum TransferError {
    /// A ledger operation failed — inspect the inner error for details.
    Ledger(LedgerClientError),
    /// A storage or database error — transient, safe to retry.
    Storage(String),
    /// The sender's available balance is insufficient.
    InsufficientFunds { available: i64, requested: i64 },
    /// No transfer exists with the given `client_id`.
    TransferNotFound,
    /// The transfer exists but is not in `Pending` state and cannot be
    /// driven forward by the requested operation.
    TransferNotPending,
    /// `complete_transfer` was called on a transfer already being cancelled.
    TransferBeingCancelled,
    /// `cancel_transfer` was called on a transfer already being completed.
    TransferBeingCompleted,
    /// The requested amount is zero or negative.
    AmountNotPositive(i64),
    /// The sender and receiver are the same account.
    AccountsNotDistinct,
    /// The two accounts have incompatible accounting natures (one debit-normal,
    /// one credit-normal) and cannot participate in a direct transfer.
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

    Ok(())
}

fn ensure_available_funds<L: LedgerClient>(
    ledger: &L,
    request: &TransferRequest,
) -> Result<(), TransferError> {
    let available = ledger.get_available_balance(request.from_account_id)?;
    if available < request.amount {
        return Err(TransferError::InsufficientFunds {
            available,
            requested: request.amount,
        });
    }
    Ok(())
}

/// Validate the request, record a `Pending` transfer, and block the funds.
///
/// Idempotent on `client_id`: if a `Pending` transfer with the same key
/// already exists (e.g. a prior call that timed out before responding), the
/// existing record is returned and `block_funds` is retried — which is itself
/// idempotent. Any other existing status returns `TransferNotPending`.
///
/// On success the sender's available balance is reduced by `amount` minor units for
/// the duration of the transfer. Call [`complete_transfer`] to post the journal
/// entry and release the block, or [`cancel_transfer`] to just release it.
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
        let _ = store.set_transfer_status(transfer.id, TransferStatus::Failed);
        return Err(e.into());
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

    ledger.post_transfer(
        &transfer.client_id,
        transfer.from_account_id,
        transfer.to_account_id,
        transfer.amount,
    )?;

    ledger.release_funds(&transfer.client_id)?;

    store.set_transfer_status(transfer.id, TransferStatus::Completed)
}

/// Release the funds block and mark the transfer Cancelled.
///
/// Intent is captured atomically before the ledger write: the transfer is moved
/// to `Cancelling` first, so a concurrent `complete_transfer` sees a conflict
/// and returns `TransferBeingCancelled` rather than racing ahead. Safe to retry.
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

    ledger.release_funds(&transfer.client_id)?;

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
        .get_account(account_id)?
        .ok_or(LedgerClientError::AccountNotFound(account_id))?;
    if !account.active {
        return Err(LedgerClientError::AccountNotActive(account_id).into());
    }
    Ok(account)
}
