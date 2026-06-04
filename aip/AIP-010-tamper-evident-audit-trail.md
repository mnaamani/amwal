# AIP-010 — Tamper-Evident Audit Trail

**Status:** Draft  
**Area:** Ledger integrity / compliance

---

## Problem

The ledger's historical record can be silently altered. A direct database `UPDATE` or `DELETE` on `ledger_lines` or `journal_entries` leaves no trace in the application layer. The transactional outbox records that events were published, but it does not prove that the underlying ledger rows still match what was originally posted.

For a Shari'ah-compliant financial institution this is a material risk:

- A Shari'ah supervisory board audit requires assurance that historical profit distributions, zakat calculations, and customer account entries have not been amended after the fact.
- A central bank examination may require the institution to prove the immutability of its books over a given period.
- Internal fraud scenarios — a privileged database user modifying balances — cannot be detected after the fact under the current architecture.

## Proposed Enhancement

Introduce a **rolling ledger checksum** that creates a cryptographic chain over journal entries, making any post-hoc modification to historical data detectable.

### Mechanism

When each journal entry is committed, a `LedgerCheckpoint` row is written in the same database transaction:

```rust
pub struct LedgerCheckpoint {
    pub id:              i64,
    pub journal_entry_id: i32,         // the entry this checkpoint covers
    pub entry_hash:      String,        // SHA-256 of the canonical entry representation
    pub chain_hash:      String,        // SHA-256 of (entry_hash || prev_chain_hash)
    pub created_at:      DateTime<Utc>,
}
```

The `chain_hash` is computed as:

```
chain_hash[n] = SHA-256(entry_hash[n] || chain_hash[n-1])
```

where `chain_hash[0]` uses a fixed genesis value (e.g. the all-zero hash). This is equivalent to a hash chain (the same structure used in audit logs and simplified blockchain designs).

### Canonical entry representation

The input to `entry_hash` is the deterministic JSON serialisation of the journal entry and all its ledger lines, sorted by `ledger_line_id`, with fields in a fixed alphabetical order:

```json
{
  "amount_credit": "500.00",
  "amount_debit": "0.00",
  "account_id": 7,
  "client_id": "je-tx-2024-11-01-001",
  "created_at": "2024-11-01T09:00:00.000000Z",
  "currency": "AED",
  "journal_entry_id": 142,
  "ledger_line_id": 284
}
```

### Verification

`verify_ledger_integrity(from_checkpoint_id, to_checkpoint_id)` re-computes every `entry_hash` and `chain_hash` in the specified range from live data and compares them to the stored values. Any mismatch identifies the first tampered entry.

A scheduled daily job runs this check and emits a `LedgerIntegrityAlert` domain event on failure.

## Key Design Points

- The `LedgerCheckpoint` table is append-only. The database role used by the application has `INSERT` but not `UPDATE` or `DELETE` on this table. Verification reads the table directly with a read-only role.
- The genesis `chain_hash` is a publicly documented constant so the chain can be verified by a third party with no secret keys.
- Checkpoints do not replace database-level backups or point-in-time recovery. They are a detection mechanism, not a recovery mechanism.
- The checksum computation adds negligible overhead per journal entry (a single SHA-256 over a small JSON payload).
- The `verify_ledger_integrity` command is exposed as a CLI binary (similar to existing `activate_account`, `create_account` bins) so that auditors can run it independently with read-only database credentials.
- For high-assurance requirements, the `chain_hash` at period-end can be signed with a hardware security module (HSM) key or published to an immutable external log. This is out of scope for V1 but the data structure supports it without schema changes.
- When git-based storage of transaction data is adopted, the `chain_hash` can be compared against the git commit SHA, providing a second independent integrity check.

## Acceptance Criteria

- Every committed journal entry has a corresponding `LedgerCheckpoint` row written in the same transaction.
- `chain_hash[n]` is a deterministic function of `entry_hash[n]` and `chain_hash[n-1]`.
- `verify_ledger_integrity` returns `Ok(())` for an untampered ledger.
- Directly `UPDATE`ing a `ledger_lines` amount in the database causes `verify_ledger_integrity` to return an error identifying the affected journal entry.
- The verification CLI exits with a non-zero status code on failure, suitable for use in a CI or monitoring pipeline.
- The `LedgerIntegrityAlert` domain event is published via the outbox on any failed verification run.
