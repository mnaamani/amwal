use std::collections::HashMap;

use ledger_api::{
    AccountId as ApiAccountId, AccountSummary, AccountType as ApiAccountType,
    JournalEntryId as ApiJournalEntryId, JournalLeg, JournalPosting, LedgerClient,
    LedgerClientError,
};

use crate::domain::{
    Account, AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLineInput,
    Posting, TrialBalanceReport,
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
pub struct LedgerService<S: LedgerStore> {
    store: S,
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

    pub fn get_active_accounts(&self) -> Result<Vec<AccountId>, LedgerError> {
        self.store.list_active_accounts()
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

        self.store.persist_journal_entry(client_id, &legs, deltas)
    }

    pub fn block_funds(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerError> {
        self.store
            .block_funds(client_id, account_id, amount)
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

    pub fn get_account_lines(&self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError> {
        self.store.find_ledger_lines(account_id)
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
            .map(account_to_summary)
            .map_err(to_client_err)
    }

    fn activate_account(&self, id: ApiAccountId) -> Result<AccountSummary, LedgerClientError> {
        LedgerService::activate_account(self, id)
            .map(account_to_summary)
            .map_err(to_client_err)
    }

    fn get_account(&self, id: ApiAccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        self.store
            .find_account(id)
            .map(|opt| opt.map(account_to_summary))
            .map_err(to_client_err)
    }

    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        let ids = self.store.list_active_accounts().map_err(to_client_err)?;
        let accounts = self
            .store
            .find_accounts_by_ids(&ids)
            .map_err(to_client_err)?;
        Ok(accounts.into_iter().map(account_to_summary).collect())
    }

    fn get_account_balance(&self, id: ApiAccountId) -> Result<i64, LedgerClientError> {
        self.store
            .find_balance(id)
            .map(|b| b.balance)
            .map_err(to_client_err)
    }

    fn get_available_balance(&self, id: ApiAccountId) -> Result<i64, LedgerClientError> {
        LedgerService::get_available_balance(self, id).map_err(to_client_err)
    }

    fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<JournalLeg>,
    ) -> Result<ApiJournalEntryId, LedgerClientError> {
        let domain_legs: Vec<NewLedgerLineInput> =
            legs.into_iter().map(journal_leg_to_input).collect();
        LedgerService::post_journal_entry(self, client_id, domain_legs)
            .map(|e| e.id)
            .map_err(to_client_err)
    }

    fn block_funds(
        &self,
        client_id: &str,
        account_id: ApiAccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError> {
        LedgerService::block_funds(self, client_id, account_id, amount).map_err(to_client_err)
    }

    fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerClientError> {
        LedgerService::release_funds(self, block_client_id).map_err(to_client_err)
    }
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn account_to_summary(a: Account) -> AccountSummary {
    AccountSummary {
        id: a.id,
        active: a.active,
        name: a.name,
        account_type: a.account_type,
    }
}

fn journal_leg_to_input(leg: JournalLeg) -> NewLedgerLineInput {
    NewLedgerLineInput {
        account_id: leg.account_id,
        posting: match leg.posting {
            JournalPosting::Debit(v) => Posting::Debit(v),
            JournalPosting::Credit(v) => Posting::Credit(v),
        },
    }
}

fn to_client_err(e: LedgerError) -> LedgerClientError {
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
    }
}
