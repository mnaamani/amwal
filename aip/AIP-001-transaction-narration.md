# AIP-001 — Transaction Narration and Payee Metadata

**Status:** Draft  
**Area:** Ledger core model

---

## Problem

Every journal entry currently carries only an opaque `client_id` (an idempotency key). There is no structured way to record who a transaction is with, what it is for, or any supplementary reference data. This makes the ledger unsuitable for producing customer-facing account statements, supporting dispute resolution, or satisfying Shari'ah audit requirements, all of which demand human-readable context on every entry.

## Proposed Enhancement

Extend the journal entry model with the following optional fields:

| Field | Type | Purpose |
|-------|------|---------|
| `payee` | `String` | Counterparty name (e.g. `"Emirates Steel LLC"`) |
| `narration` | `String` | Free-text description of the transaction |
| `reference` | `String` | External reference number (bank ref, invoice number, etc.) |

Additionally, support a key-value **metadata** map at both the transaction level and the individual posting level. This enables arbitrary extensibility — tax codes, cost centres, document attachment paths, Shari'ah product type codes — without schema migrations for each new field.

## Key Design Points

- `payee` and `narration` are distinct: payee identifies the counterparty, narration describes the economic event. Both can be empty.
- Metadata keys are strings; values are strings or numbers. The pair is stored as a JSON column in PostgreSQL.
- Posting-level metadata overrides transaction-level metadata for the same key when both are present.
- The `client_id` field is retained as-is; it is an infrastructure concern (idempotency) not a business one (description).
- The outbox `BalanceChanged` event should be extended to carry narration so downstream consumers (statements, notifications) do not need to query back.

## Acceptance Criteria

- `post_journal_entry` accepts an optional `payee`, `narration`, `reference`, and metadata map.
- `get_balance_history` returns narration and payee alongside each ledger line.
- Metadata is round-trippable: what is stored is exactly what is returned.
- Existing calls with no narration fields continue to work without change.
