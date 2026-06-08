use ledger_api::{
    AccountId, AccountSummary, AccountType, BalanceSnapshot, JournalEntryId, JournalLeg,
    LedgerClient, LedgerClientError,
};

/// Synchronous HTTP client that implements [`LedgerClient`] by calling the
/// `ledger-http-server` REST API.
///
/// Drop-in replacement for `LedgerService` in any code that depends only on
/// the `LedgerClient` trait.
///
/// ```no_run
/// use ledger_http_client::LedgerHttpClient;
/// use ledger_api::LedgerClient;
///
/// let client = LedgerHttpClient::new("http://localhost:8080");
/// let accounts = client.list_active_accounts().unwrap();
/// ```
pub struct LedgerHttpClient {
    base_url: String,
    agent: ureq::Agent,
}

impl LedgerHttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            agent: ureq::AgentBuilder::new().build(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn map_error(e: ureq::Error) -> LedgerClientError {
        match e {
            ureq::Error::Status(_, resp) => {
                resp.into_json::<LedgerClientError>().unwrap_or_else(|_| {
                    LedgerClientError::Unavailable("unparseable error body".into())
                })
            }
            ureq::Error::Transport(t) => LedgerClientError::Unavailable(t.to_string()),
        }
    }

    fn parse<T: serde::de::DeserializeOwned>(resp: ureq::Response) -> Result<T, LedgerClientError> {
        resp.into_json::<T>()
            .map_err(|e| LedgerClientError::Unavailable(e.to_string()))
    }
}

impl LedgerClient for LedgerHttpClient {
    fn create_account(
        &self,
        client_id: &str,
        name: &str,
        account_type: AccountType,
    ) -> Result<AccountSummary, LedgerClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            client_id: &'a str,
            name: &'a str,
            account_type: AccountType,
        }
        let resp = self
            .agent
            .post(&self.url("/accounts"))
            .send_json(
                serde_json::to_value(Body {
                    client_id,
                    name,
                    account_type,
                })
                .unwrap(),
            )
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn activate_account(&self, id: AccountId) -> Result<AccountSummary, LedgerClientError> {
        let resp = self
            .agent
            .post(&self.url(&format!("/accounts/{id}/activate")))
            .send_bytes(&[])
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn get_account(&self, id: AccountId) -> Result<Option<AccountSummary>, LedgerClientError> {
        match self.agent.get(&self.url(&format!("/accounts/{id}"))).call() {
            Ok(resp) => Self::parse::<AccountSummary>(resp).map(Some),
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(e) => Err(Self::map_error(e)),
        }
    }

    fn list_active_accounts(&self) -> Result<Vec<AccountSummary>, LedgerClientError> {
        let resp = self
            .agent
            .get(&self.url("/accounts"))
            .call()
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn get_account_balance(&self, id: AccountId) -> Result<i64, LedgerClientError> {
        let resp = self
            .agent
            .get(&self.url(&format!("/accounts/{id}/balance")))
            .call()
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn get_available_balance(&self, id: AccountId) -> Result<i64, LedgerClientError> {
        let resp = self
            .agent
            .get(&self.url(&format!("/accounts/{id}/available-balance")))
            .call()
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn get_balance_history(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<BalanceSnapshot>, LedgerClientError> {
        let resp = self
            .agent
            .get(&self.url(&format!("/accounts/{account_id}/balance-history")))
            .call()
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn post_journal_entry(
        &self,
        client_id: &str,
        legs: Vec<JournalLeg>,
    ) -> Result<JournalEntryId, LedgerClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            client_id: &'a str,
            legs: Vec<JournalLeg>,
        }
        let resp = self
            .agent
            .post(&self.url("/journal/entries"))
            .send_json(serde_json::to_value(Body { client_id, legs }).unwrap())
            .map_err(Self::map_error)?;
        Self::parse(resp)
    }

    fn block_funds(
        &self,
        client_id: &str,
        account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            client_id: &'a str,
            account_id: i64,
            amount: i64,
        }
        self.agent
            .post(&self.url("/funds/blocks"))
            .send_json(
                serde_json::to_value(Body {
                    client_id,
                    account_id,
                    amount,
                })
                .unwrap(),
            )
            .map_err(Self::map_error)?;
        Ok(())
    }

    fn release_funds(&self, block_client_id: &str) -> Result<(), LedgerClientError> {
        self.agent
            .delete(&self.url(&format!("/funds/blocks/{block_client_id}")))
            .call()
            .map_err(Self::map_error)?;
        Ok(())
    }

    fn post_transfer(
        &self,
        client_id: &str,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
    ) -> Result<(), LedgerClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            client_id: &'a str,
            from_account_id: i64,
            to_account_id: i64,
            amount: i64,
        }
        self.agent
            .post(&self.url("/journal/transfers"))
            .send_json(
                serde_json::to_value(Body {
                    client_id,
                    from_account_id,
                    to_account_id,
                    amount,
                })
                .unwrap(),
            )
            .map_err(Self::map_error)?;
        Ok(())
    }
}
