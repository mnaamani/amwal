use std::collections::HashMap;
use std::num::NonZeroU64;

use ledger_api::{
    AccountId as ApiAccountId, AccountSummary, AccountType as ApiAccountType,
    JournalEntryId as ApiJournalEntryId, JournalLeg, JournalPosting, LedgerClient,
    LedgerClientError,
};

use crate::domain::{
    Account, AccountId, AccountType, Balance, JournalEntry, NewLedgerLineInput, Posting,
    TrialBalanceReport,
};
use crate::errors::LedgerError;
use crate::postgres::PostgresLedgerStore;
use crate::store::LedgerStore;

/// Business logic layer. Wraps any [`LedgerStore`] and adds:
/// - input validation (structural rules, double-entry invariant)
/// - balance delta computation (account-type-aware)
/// - the external [`LedgerClient`] interface consumed by other services
///
/// Use `Arc<LedgerService<S>>` to share a single instance across callers.
/// Events are published via the transactional outbox — wire up an
/// [`OutboxRelay`](crate::OutboxRelay) at startup rather than passing a bus here.
pub struct LedgerService<S: LedgerStore> {
    store: S,
}

impl<S: LedgerStore + Clone> Clone for LedgerService<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}

impl<S: LedgerStore> LedgerService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn create_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<Account, LedgerError> {
        if name.trim().is_empty() {
            return Err(LedgerError::InvalidInput(
                "account name cannot be empty".into(),
            ));
        }
        self.store.insert_account(client_id, name, account_type)
    }

    pub fn activate_account(&self, id: AccountId) -> Result<Account, LedgerError> {
        if self.store.find_account(id)?.is_none() {
            return Err(LedgerError::AccountNotFound(id));
        }
        self.store.set_account_active(id)
    }

    pub fn get_account(&self, id: AccountId) -> Result<Option<Account>, LedgerError> {
        self.store.find_account(id)
    }

    pub fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<NewLedgerLineInput>,
    ) -> Result<JournalEntry, LedgerError> {
        // ── Structural validation ──────────────────────────────────────────
        if legs.len() < 2 {
            return Err(LedgerError::InvalidJournalEntry(
                "a journal entry requires at least two legs".into(),
            ));
        }
        let total_debits: i64 = legs.iter().map(|l| l.posting.debit()).sum();
        let total_credits: i64 = legs.iter().map(|l| l.posting.credit()).sum();
        if total_debits != total_credits {
            return Err(LedgerError::ImbalancedEntry {
                total_debits,
                total_credits,
            });
        }

        // ── Account validation ─────────────────────────────────────────────
        let distinct_ids: Vec<AccountId> = {
            let mut ids: Vec<AccountId> = legs.iter().map(|l| l.account_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };
        let found_accounts = self.store.find_accounts_by_ids(&distinct_ids)?;
        if found_accounts.len() != distinct_ids.len() {
            return Err(LedgerError::Storage(
                "one or more accounts not found".into(),
            ));
        }
        for account in &found_accounts {
            if !account.active {
                return Err(LedgerError::AccountNotActive(account.id));
            }
        }

        // ── Balance delta computation ──────────────────────────────────────
        // Debit-nature (Asset, Expense): delta = debit − credit
        // Credit-nature (Liability, Equity, Revenue): delta = credit − debit
        let type_map: HashMap<AccountId, AccountType> = found_accounts
            .iter()
            .map(|a| (a.id, a.account_type))
            .collect();
        let mut deltas: HashMap<AccountId, i64> = HashMap::new();
        for leg in &legs {
            let account_type = type_map[&leg.account_id];
            let delta = match (&leg.posting, account_type) {
                (Posting::Debit(v), AccountType::Asset | AccountType::Expense) => v.get() as i64,
                (Posting::Credit(v), AccountType::Asset | AccountType::Expense) => {
                    -(v.get() as i64)
                }
                (Posting::Credit(v), _) => v.get() as i64,
                (Posting::Debit(v), _) => -(v.get() as i64),
            };
            *deltas.entry(leg.account_id).or_insert(0) += delta;
        }

        let entry = self.store.persist_journal_entry(client_id, &legs, deltas)?;
        Ok(entry)
    }

    pub fn post_transfer(
        &self,
        client_id: &str,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerError> {
        let nonzero_amount = u64::try_from(amount)
            .ok()
            .and_then(NonZeroU64::new)
            .ok_or_else(|| {
                LedgerError::InvalidInput(format!("transfer amount must be positive, got {amount}"))
            })?;

        let from = self
            .store
            .find_account(from_account_id)?
            .ok_or(LedgerError::AccountNotFound(from_account_id))?;
        if !from.active {
            return Err(LedgerError::AccountNotActive(from_account_id));
        }

        let to = self
            .store
            .find_account(to_account_id)?
            .ok_or(LedgerError::AccountNotFound(to_account_id))?;
        if !to.active {
            return Err(LedgerError::AccountNotActive(to_account_id));
        }

        if !ledger_api::accounts_compatible(from.account_type, to.account_type) {
            return Err(LedgerError::AccountsIncompatible);
        }

        let (from_posting, to_posting) = transfer_postings(from.account_type, nonzero_amount);
        self.post_journal_entry(
            client_id,
            vec![
                NewLedgerLineInput {
                    account_id: from_account_id,
                    posting: from_posting,
                },
                NewLedgerLineInput {
                    account_id: to_account_id,
                    posting: to_posting,
                },
            ],
        )?;
        Ok(())
    }

    pub fn block_funds(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerError> {
        self.store
            .apply_account_block(client_id, account_id, amount)
            .map(|_| ())
    }

    pub fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerError> {
        self.store
            .release_account_block(block_client_id)
            .map(|_| ())
    }

    pub fn get_account_balance(&self, account_id: AccountId) -> Result<Balance, LedgerError> {
        self.store.find_balance(account_id)
    }

    pub fn get_available_balance(&self, account_id: AccountId) -> Result<i64, LedgerError> {
        let balance = self.store.find_balance(account_id)?.balance;
        let blocked = self.store.sum_unreleased_blocks(account_id)?;
        Ok(balance - blocked)
    }

    pub fn trial_balance(&self) -> Result<TrialBalanceReport, LedgerError> {
        let rows = self.store.aggregate_balances_by_type()?;
        let mut report = TrialBalanceReport {
            asset: 0,
            expense: 0,
            liability: 0,
            equity: 0,
            revenue: 0,
            is_balanced: false,
        };
        for (account_type, balance) in rows {
            match account_type {
                AccountType::Asset => report.asset += balance,
                AccountType::Expense => report.expense += balance,
                AccountType::Liability => report.liability += balance,
                AccountType::Equity => report.equity += balance,
                AccountType::Revenue => report.revenue += balance,
            }
        }
        report.is_balanced =
            (report.asset + report.expense) == (report.liability + report.equity + report.revenue);
        Ok(report)
    }
}

/// Convenience constructor for the standard Postgres-backed service.
impl LedgerService<PostgresLedgerStore> {
    pub fn from_env() -> Self {
        Self::new(PostgresLedgerStore::from_env())
    }
}

// ── External interface ────────────────────────────────────────────────────────

impl<S: LedgerStore> LedgerClient for LedgerService<S> {
    fn create_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: ApiAccountType,
    ) -> Result<AccountSummary, LedgerClientError> {
        LedgerService::create_account(self, client_id, name, account_type)
            .map(Into::into)
            .map_err(Into::into)
    }

    fn activate_account(&self, id: ApiAccountId) -> Result<AccountSummary, LedgerClientError> {
        LedgerService::activate_account(self, id)
            .map(Into::into)
            .map_err(Into::into)
    }

    fn get_account(&self, id: ApiAccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        self.store
            .find_account(id)
            .map(|opt| opt.map(Into::into))
            .map_err(Into::into)
    }

    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        self.store
            .list_active_accounts()
            .map(|v| v.into_iter().map(Into::into).collect())
            .map_err(Into::into)
    }

    fn get_account_balance(&self, id: ApiAccountId) -> Result<i64, LedgerClientError> {
        self.store
            .find_balance(id)
            .map(|b| b.balance)
            .map_err(Into::into)
    }

    fn get_balance_history(
        &self,
        account_id: ApiAccountId,
    ) -> Result<Vec<ledger_api::BalanceSnapshot>, LedgerClientError> {
        let account = self
            .store
            .find_account(account_id)?
            .ok_or(LedgerClientError::AccountNotFound(account_id))?;

        let lines = self.store.find_ledger_lines(account_id)?;

        let mut running: i64 = 0;
        let snapshots = lines
            .into_iter()
            .map(|line| {
                let delta = match account.account_type {
                    AccountType::Asset | AccountType::Expense => line.debit - line.credit,
                    _ => line.credit - line.debit,
                };
                running += delta;
                ledger_api::BalanceSnapshot {
                    timestamp: line.created_at,
                    balance: running,
                }
            })
            .collect();

        Ok(snapshots)
    }

    fn get_available_balance(&self, id: ApiAccountId) -> Result<i64, LedgerClientError> {
        LedgerService::get_available_balance(self, id).map_err(Into::into)
    }

    fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<JournalLeg>,
    ) -> Result<ApiJournalEntryId, LedgerClientError> {
        LedgerService::post_journal_entry(
            self,
            client_id,
            legs.into_iter().map(Into::into).collect(),
        )
        .map(|e| e.id)
        .map_err(Into::into)
    }

    fn block_funds(
        &self,
        client_id: &str,
        account_id: ApiAccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError> {
        LedgerService::block_funds(self, client_id, account_id, amount).map_err(Into::into)
    }

    fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerClientError> {
        LedgerService::release_funds(self, block_client_id).map_err(Into::into)
    }

    fn post_transfer(
        &self,
        client_id: &str,
        from_account_id: ApiAccountId,
        to_account_id: ApiAccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError> {
        LedgerService::post_transfer(self, client_id, from_account_id, to_account_id, amount)
            .map_err(Into::into)
    }
}

// ── Conversions ───────────────────────────────────────────────────────────────

impl From<Account> for AccountSummary {
    fn from(a: Account) -> Self {
        AccountSummary {
            id: a.id,
            active: a.active,
            name: a.name,
            account_type: a.account_type,
        }
    }
}

impl From<JournalPosting> for Posting {
    fn from(p: JournalPosting) -> Self {
        match p {
            JournalPosting::Debit(v) => Posting::Debit(v),
            JournalPosting::Credit(v) => Posting::Credit(v),
        }
    }
}

impl From<JournalLeg> for NewLedgerLineInput {
    fn from(leg: JournalLeg) -> Self {
        NewLedgerLineInput {
            account_id: leg.account_id,
            posting: leg.posting.into(),
        }
    }
}

impl From<LedgerError> for LedgerClientError {
    fn from(e: LedgerError) -> Self {
        match e {
            LedgerError::AccountNotFound(id) => LedgerClientError::AccountNotFound(id),
            LedgerError::AccountNotActive(id) => LedgerClientError::AccountNotActive(id),
            LedgerError::ImbalancedEntry {
                total_debits,
                total_credits,
            } => LedgerClientError::ImbalancedEntry {
                total_debits,
                total_credits,
            },
            LedgerError::InvalidJournalEntry(s)
            | LedgerError::InvalidLedgerLine(s)
            | LedgerError::InvalidInput(s) => LedgerClientError::InvalidRequest(s),
            LedgerError::Storage(s) => LedgerClientError::Unavailable(s),
            LedgerError::InsufficientFunds {
                available,
                requested,
            } => LedgerClientError::InsufficientFunds {
                available,
                requested,
            },
            LedgerError::AccountsIncompatible => LedgerClientError::AccountsIncompatible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Account, AccountBlock, AccountId, Balance, JournalEntry, LedgerLine, NewLedgerLineInput,
        Posting,
    };
    use crate::errors::LedgerError;
    use crate::store::LedgerStore;
    use std::collections::HashMap;
    use std::num::NonZeroU64;
    use std::time::SystemTime;

    // ── MockStore ─────────────────────────────────────────────────────────────

    struct MockStore {
        // (id, account_type, active)
        accounts: Vec<(AccountId, AccountType, bool)>,
    }

    impl MockStore {
        fn new(accounts: Vec<(AccountId, AccountType, bool)>) -> Self {
            Self { accounts }
        }

        fn build_account(id: AccountId, account_type: AccountType, active: bool) -> Account {
            Account {
                id,
                client_id: format!("client-{id}"),
                account_type,
                active,
                name: format!("Account {id}"),
                created_at: SystemTime::UNIX_EPOCH,
            }
        }
    }

    impl LedgerStore for MockStore {
        fn insert_account(
            &self,
            _client_id: &str,
            _name: &str,
            account_type: AccountType,
        ) -> Result<Account, LedgerError> {
            Ok(Self::build_account(1, account_type, false))
        }

        fn set_account_active(&self, _: AccountId) -> Result<Account, LedgerError> {
            unimplemented!()
        }

        fn find_account(&self, id: AccountId) -> Result<Option<Account>, LedgerError> {
            Ok(self
                .accounts
                .iter()
                .find(|(aid, _, _)| *aid == id)
                .map(|&(id, at, active)| Self::build_account(id, at, active)))
        }

        fn find_accounts_by_ids(&self, ids: &[AccountId]) -> Result<Vec<Account>, LedgerError> {
            Ok(self
                .accounts
                .iter()
                .filter(|(id, _, _)| ids.contains(id))
                .map(|&(id, at, active)| Self::build_account(id, at, active))
                .collect())
        }

        fn list_active_accounts(&self) -> Result<Vec<Account>, LedgerError> {
            unimplemented!()
        }

        fn persist_journal_entry(
            &self,
            client_id: &str,
            _legs: &[NewLedgerLineInput],
            _deltas: HashMap<AccountId, i64>,
        ) -> Result<JournalEntry, LedgerError> {
            Ok(JournalEntry {
                id: 1,
                client_id: client_id.to_string(),
                created_at: SystemTime::UNIX_EPOCH,
                updated_at: None,
            })
        }

        fn find_balance(&self, _: AccountId) -> Result<Balance, LedgerError> {
            unimplemented!()
        }

        fn find_ledger_lines(&self, _: AccountId) -> Result<Vec<LedgerLine>, LedgerError> {
            unimplemented!()
        }

        fn aggregate_balances_by_type(&self) -> Result<Vec<(AccountType, i64)>, LedgerError> {
            unimplemented!()
        }

        fn sum_unreleased_blocks(&self, _: AccountId) -> Result<i64, LedgerError> {
            unimplemented!()
        }

        fn apply_account_block(
            &self,
            _: &str,
            _: AccountId,
            _: i64,
        ) -> Result<AccountBlock, LedgerError> {
            unimplemented!()
        }

        fn release_account_block(&self, _: &str) -> Result<AccountBlock, LedgerError> {
            unimplemented!()
        }
    }

    fn svc(accounts: Vec<(AccountId, AccountType, bool)>) -> LedgerService<MockStore> {
        LedgerService::new(MockStore::new(accounts))
    }

    fn nz(n: u64) -> NonZeroU64 {
        NonZeroU64::new(n).unwrap()
    }

    // ── create_account ────────────────────────────────────────────────────────

    #[test]
    fn create_account_rejects_empty_name() {
        let result = svc(vec![]).create_account("c1", "", AccountType::Asset);
        assert!(matches!(result, Err(LedgerError::InvalidInput(_))));
    }

    #[test]
    fn create_account_rejects_whitespace_name() {
        let result = svc(vec![]).create_account("c1", "   ", AccountType::Asset);
        assert!(matches!(result, Err(LedgerError::InvalidInput(_))));
    }

    // ── post_journal_entry ────────────────────────────────────────────────────

    #[test]
    fn post_journal_entry_requires_two_legs() {
        let result = svc(vec![]).post_journal_entry(
            "je-1",
            vec![NewLedgerLineInput {
                account_id: 1,
                posting: Posting::Debit(nz(100)),
            }],
        );
        assert!(matches!(result, Err(LedgerError::InvalidJournalEntry(_))));
    }

    #[test]
    fn post_journal_entry_rejects_imbalanced() {
        let result = svc(vec![]).post_journal_entry(
            "je-1",
            vec![
                NewLedgerLineInput {
                    account_id: 1,
                    posting: Posting::Debit(nz(100)),
                },
                NewLedgerLineInput {
                    account_id: 2,
                    posting: Posting::Credit(nz(200)),
                },
            ],
        );
        assert!(matches!(result, Err(LedgerError::ImbalancedEntry { .. })));
    }

    #[test]
    fn post_journal_entry_rejects_inactive_account() {
        let result = svc(vec![
            (1, AccountType::Asset, true),
            (2, AccountType::Asset, false),
        ])
        .post_journal_entry(
            "je-1",
            vec![
                NewLedgerLineInput {
                    account_id: 1,
                    posting: Posting::Debit(nz(100)),
                },
                NewLedgerLineInput {
                    account_id: 2,
                    posting: Posting::Credit(nz(100)),
                },
            ],
        );
        assert!(matches!(result, Err(LedgerError::AccountNotActive(2))));
    }

    #[test]
    fn post_journal_entry_missing_account_returns_error() {
        // Account 2 is not in the store.
        let result = svc(vec![(1, AccountType::Asset, true)]).post_journal_entry(
            "je-1",
            vec![
                NewLedgerLineInput {
                    account_id: 1,
                    posting: Posting::Debit(nz(100)),
                },
                NewLedgerLineInput {
                    account_id: 2,
                    posting: Posting::Credit(nz(100)),
                },
            ],
        );
        assert!(matches!(result, Err(LedgerError::Storage(_))));
    }

    #[test]
    fn post_journal_entry_balanced_active_accounts_succeeds() {
        let result = svc(vec![
            (1, AccountType::Asset, true),
            (2, AccountType::Liability, true),
        ])
        .post_journal_entry(
            "je-1",
            vec![
                NewLedgerLineInput {
                    account_id: 1,
                    posting: Posting::Debit(nz(500)),
                },
                NewLedgerLineInput {
                    account_id: 2,
                    posting: Posting::Credit(nz(500)),
                },
            ],
        );
        assert!(result.is_ok());
    }
}

/// Returns the (from_posting, to_posting) pair that decreases the sender's
/// balance and increases the receiver's, given the sender's account type.
fn transfer_postings(account_type: AccountType, amount: NonZeroU64) -> (Posting, Posting) {
    if matches!(account_type, AccountType::Asset | AccountType::Expense) {
        // Debit-normal: Credit decreases balance, Debit increases balance.
        (Posting::Credit(amount), Posting::Debit(amount))
    } else {
        // Credit-normal: Debit decreases balance, Credit increases balance.
        (Posting::Debit(amount), Posting::Credit(amount))
    }
}
