# AIP-007 — Period-Based Aggregated Reporting

**Status:** Draft  
**Area:** Ledger read path / reporting

---

## Problem

`get_balance_history` returns one data point per ledger line — every individual posting in chronological order with a running balance. This is a raw event stream, not a reporting primitive. There is no way to ask:

- "What was the net movement on this account each month for the last 12 months?"
- "Show me weekly cash inflows and outflows."
- "Produce a quarterly P&L by account category."

Every downstream reporting tool must implement period bucketing from scratch, creating duplicated logic across consuming services. Regulatory submissions (monthly financials, quarterly CBUAE reports) and customer-facing statements require period-granularity data as a first-class output.

## Proposed Enhancement

Introduce a `PeriodReport` API with configurable granularity:

```rust
pub enum Granularity {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

pub struct PeriodBucket {
    pub period_start: DateTime<Utc>,
    pub period_end:   DateTime<Utc>,
    pub opening:      Decimal,       // balance at start of period
    pub closing:      Decimal,       // balance at end of period
    pub net_change:   Decimal,       // closing - opening
    pub total_debits: Decimal,
    pub total_credits: Decimal,
    pub currency:     String,
}

fn get_period_balances(
    &self,
    account_id: AccountId,
    from: DateTime<Utc>,
    to:   DateTime<Utc>,
    granularity: Granularity,
    currency: String,
) -> Result<Vec<PeriodBucket>, LedgerClientError>;
```

A companion function operates over an account prefix (hierarchy node) to produce consolidated P&L and balance-sheet views:

```rust
fn get_period_balances_by_prefix(
    &self,
    account_prefix: &str,
    from: DateTime<Utc>,
    to:   DateTime<Utc>,
    granularity: Granularity,
    currency: String,
) -> Result<Vec<PeriodBucket>, LedgerClientError>;
```

## Key Design Points

- Period boundaries are calendar-aligned in UTC (months start on the 1st at 00:00:00 UTC, quarters on Jan/Apr/Jul/Oct). The first and last buckets may be partial if `from`/`to` do not fall on period boundaries.
- The SQL implementation uses `date_trunc` with a `generate_series` for the time axis, left-joined to ledger lines, so periods with zero activity appear as buckets with `net_change = 0` rather than being absent from the result.
- `opening` balance for the first bucket is the account balance as of `from` (computed from all prior postings). Each subsequent bucket's `opening` equals the prior bucket's `closing`.
- The implementation is a single SQL query (CTE + window function) to avoid N+1 problems.
- When AIP-003 (hierarchical accounts) is implemented, `get_period_balances_by_prefix` simply extends the `account_id IN (...)` clause with a prefix match; no separate code path.
- Timezone-aware reporting (e.g. for a customer whose fiscal year is in GST/UAE time) is handled by the caller converting `from`/`to` to UTC before calling; the service always stores and computes in UTC.

## Acceptance Criteria

- `get_period_balances` with `Monthly` granularity returns one `PeriodBucket` per calendar month within the range.
- Months with no transactions appear as buckets with `net_change = 0`, `opening == closing`.
- `opening` of each bucket matches the `closing` of the preceding bucket.
- `net_change == total_credits - total_debits` for credit-normal accounts, and the inverse for debit-normal accounts.
- `get_period_balances_by_prefix("Revenue")` aggregates all accounts under the Revenue hierarchy into consolidated period buckets.
- The query executes in a single round-trip to the database (no N+1).
