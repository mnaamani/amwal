# AIP-005 — Reconciliation and Clearing Status

**Status:** Draft  
**Area:** Ledger core model

---

## Problem

There is currently no mechanism to mark transactions as having been verified against an external source (a bank statement, a custodian report, a central bank feed). Operations teams must track reconciliation state in a spreadsheet or external tool, which creates a gap between the ledger and the audit record. Without reconciliation status in the ledger itself:

- Month-end close cannot be formally signed off.
- Uncleared transactions cannot be distinguished from cleared ones in balance reports.
- Auditors cannot confirm which entries have been matched to primary source documents.

## Proposed Enhancement

Add a **clearing status** field to each ledger line (posting) with three states:

| Status | Meaning |
|--------|---------|
| `Uncleared` | Newly posted; not yet verified against any external source |
| `Pending` | Tentatively matched to an external entry; awaiting final confirmation |
| `Cleared` | Confirmed against a primary source document (bank statement, custodian report) |

A **reconciliation session** groups a set of postings matched during one reconciliation run:

```
ReconciliationSession {
    id:               Uuid,
    account_id:       AccountId,
    statement_date:   Date,
    statement_balance: Amount,
    cleared_balance:   Amount,   // computed from cleared postings
    difference:        Amount,   // must reach zero to close the session
    closed_at:         Option<DateTime<Utc>>,
    closed_by:         Option<String>,
}
```

Closing a session (difference == 0) atomically marks all `Pending` postings in it as `Cleared`.

## Key Design Points

- New postings default to `Uncleared`. The posting API does not require callers to specify a status; it is an ops-facing field, not a business-logic field.
- `get_account_balance` gains a `cleared_only: bool` parameter. When true, only `Cleared` postings contribute to the returned balance. This is the "book balance" used for reconciliation sign-off.
- The `block_funds` and transfer machinery is unaffected: available balance uses all postings regardless of clearing status (uncleared postings are real obligations).
- A dedicated `ReconciliationService` (separate from `LedgerService`) manages session lifecycle so that the reconciliation concern does not pollute the core ledger API.
- Bulk-clearing an array of `ledger_line_id`s within a session is a single transactional update.
- Closing a session emits a `ReconciliationClosed` domain event via the outbox, enabling downstream audit log consumers.

## Acceptance Criteria

- New postings default to `Uncleared`.
- A reconciliation session can be opened for an account with a stated statement date and balance.
- Individual postings can be moved from `Uncleared` to `Pending` within a session.
- Closing a session is rejected if `cleared_balance != statement_balance`.
- `get_account_balance(cleared_only: true)` reflects only `Cleared` postings.
- The `ReconciliationClosed` event appears in the outbox within the same transaction as the session close.
