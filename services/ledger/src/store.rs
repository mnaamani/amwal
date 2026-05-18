use crate::domain::{
    Account, AccountId, AccountType, Balance, JournalEntry, LedgerLine, NewLedgerLineInput,
    TrialBalanceReport,
};
use crate::errors::LedgerError;

pub trait LedgerStore {
    fn create_account(
        &mut self,
        name: &str,
        account_type: AccountType,
    ) -> Result<Account, LedgerError>;
    fn activate_account(&mut self, id: AccountId) -> Result<Account, LedgerError>;
    fn delete_account(&mut self, pattern: &str) -> Result<usize, LedgerError>;
    fn get_account(&mut self, id: AccountId) -> Result<Option<Account>, LedgerError>;
    fn get_active_accounts(&mut self) -> Result<Vec<Account>, LedgerError>;

    fn post_journal_entry(
        &mut self,
        legs: Vec<NewLedgerLineInput>,
    ) -> Result<JournalEntry, LedgerError>;
    fn get_account_balance(&mut self, account_id: AccountId) -> Result<Balance, LedgerError>;
    fn get_account_lines(&mut self, account_id: AccountId) -> Result<Vec<LedgerLine>, LedgerError>;
    fn trial_balance(&mut self) -> Result<TrialBalanceReport, LedgerError>;
}
