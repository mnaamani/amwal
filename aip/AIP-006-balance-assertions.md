# AIP-006 — Balance Assertions

**Status:** Draft  
**Area:** Ledger integrity

---

## Problem

The ledger currently has no mechanism to assert that an account's computed balance matches a known-good figure at a specific point in time. Errors introduced by a bug, a manual correction, or a data migration can silently accumulate without detection. There is no programmatic way to confirm "as of 31 December, account X had balance Y" and have the system raise an error if the books disagree.

This matters acutely for:
- Period-end close: a finance controller needs to attest that computed balances match external statements.
- Data migration verification: after moving historical data, asserting spot balances confirms the migration was lossless.
- Continuous integrity monitoring: assertions stored in the ledger act as a regression test for the historical record.

## Proposed Enhancement

Introduce a **BalanceAssertion** record:

```rust
pub struct BalanceAssertion {
    pub id:          i32,
    pub client_id:   String,         // idempotency key
    pub account_id:  AccountId,
    pub assert_date: DateTime<Utc>,  // the point in time being asserted
    pub currency:    String,
    pub expected:    Decimal,        // the asserted balance
    pub actual:      Decimal,        // computed at insertion time
    pub passed:      bool,
    pub diff:        Decimal,        // actual - expected
    pub created_at:  DateTime<Utc>,
}
```

`insert_balance_assertion` computes the actual balance as of `assert_date` by summing all ledger lines posted on or before that timestamp, compares it to `expected`, records the result, and returns `Err(BalanceAssertionFailed { expected, actual, diff })` if the assertion does not pass.

A **verification sweep** — `verify_all_assertions()` — re-runs every stored assertion against current data and returns a list of any that now disagree with the computed balance. This is the tool used after a migration or a manual correction.

## Key Design Points

- Assertions are immutable once inserted. A failed assertion is a permanent record that something was wrong; it is not updated when the books are corrected. A corrective assertion with a new `client_id` is inserted after the fix.
- `assert_date` is the exclusive upper bound: balance = sum of all postings with `created_at <= assert_date`.
- Assertions are multi-currency aware (one assertion per currency). When AIP-002 is implemented, `currency` is mandatory; until then it defaults to `"AED"`.
- Insertion is idempotent on `client_id`: submitting the same assertion twice returns the existing record.
- A separate scheduled job (or CLI command) runs `verify_all_assertions()` daily and emits an `AssertionDriftDetected` event to the outbox if any discrepancy is found.
- Assertions do not block posting. They are a detection mechanism, not a write lock.

## Acceptance Criteria

- `insert_balance_assertion` with a correct expected value persists a passing assertion.
- `insert_balance_assertion` with an incorrect expected value persists a failing assertion and returns `BalanceAssertionFailed`.
- `verify_all_assertions()` returns an empty list when all assertions still hold.
- `verify_all_assertions()` flags any assertion whose expected balance no longer matches the computed balance.
- Submitting the same `client_id` twice returns the original assertion without re-computing.
- The `assert_date` boundary is respected: postings after that date do not affect the computed actual.
