# AIP-004 — Date-Range Querying and Transaction Filtering

**Status:** Draft  
**Area:** Ledger read path

---

## Problem

The current read API offers two operations: `get_balance_history` (all ledger lines for one account, unfiltered) and `get_account_balance` (current balance scalar). There is no way to:

- Retrieve transactions for an account within a date range.
- Search transactions by payee, narration, or reference number.
- Filter by amount bounds or currency.
- List all transactions that touched a particular account in a given month.
- Produce a paginated transaction feed for a customer statement.

Every consuming application — statement generation, dispute investigation, Shari'ah audit export — must fetch the entire history and filter in memory. This is not viable at scale.

## Proposed Enhancement

Introduce a `TransactionQuery` struct that expresses bounded, filtered reads:

```rust
pub struct TransactionQuery {
    pub account_id:   Option<AccountId>,        // filter to one account
    pub account_prefix: Option<String>,         // or all accounts under a hierarchy node
    pub from:         Option<DateTime<Utc>>,
    pub to:           Option<DateTime<Utc>>,
    pub payee:        Option<String>,           // substring match
    pub narration:    Option<String>,           // substring match
    pub reference:    Option<String>,           // exact match
    pub currency:     Option<String>,
    pub min_amount:   Option<Decimal>,
    pub max_amount:   Option<Decimal>,
    pub page:         u32,
    pub page_size:    u32,                      // capped at 1000
}
```

The corresponding `LedgerClient` method:

```rust
fn query_transactions(
    &self,
    query: &TransactionQuery,
) -> Result<Page<JournalEntry>, LedgerClientError>;
```

where `Page<T>` carries the result slice plus a total-count estimate for pagination UI.

## Key Design Points

- The underlying SQL is generated with `WHERE` clauses built from the non-None query fields. An empty query returns the first page of all transactions ordered by `created_at DESC`.
- `from`/`to` filter on `journal_entries.created_at`. A separate `effective_date` field (see AIP-001) would allow filtering by business date vs. system date.
- Payee and narration use `ILIKE '%term%'` for case-insensitive substring matching. Full-text search (`tsvector`) is a later optimisation.
- `account_prefix` translates to `full_name LIKE 'prefix:%'` and joins through `ledger_lines` to find all entries that touched any account under that node.
- Pagination is offset-based for V1. Cursor-based pagination (by `journal_entry_id`) is the recommended V2 when dealing with large result sets.
- A maximum `page_size` of 1000 is enforced server-side regardless of caller input.
- Adding a `created_at` index on `journal_entries` and `ledger_lines` is a prerequisite migration.

## Acceptance Criteria

- `query_transactions` with `from`/`to` bounds returns only entries whose `created_at` falls within the range, inclusive.
- `query_transactions` with an `account_prefix` returns entries that include at least one posting to a matching account.
- Payee and narration filters are case-insensitive and match partial strings.
- Results are paginated; requesting page 2 with page_size 50 returns the correct offset.
- An empty query (all fields None) returns the most recent page of all transactions.
- All filters compose correctly when combined (AND semantics).
