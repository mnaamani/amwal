# AIP-008 — Bank Statement Import

**Status:** Draft  
**Area:** Data ingestion

---

## Problem

There is no mechanism to bring external financial data into the ledger. All postings must be constructed manually through the programmatic API. In practice, every finance team receives transaction data from external sources — bank statement CSV exports, SWIFT MT940 files, payment processor feeds — and needs to reconcile and post those entries into the ledger. Without an import path, the ledger is disconnected from operational reality and every posting requires a bespoke integration.

## Proposed Enhancement

Introduce an **import pipeline** with three stages:

### Stage 1 — Parse
Accept a raw file (CSV or MT940) and parse it into a `Vec<ImportedTransaction>`:

```rust
pub struct ImportedTransaction {
    pub external_id:   String,        // unique ID from the source (e.g. bank ref)
    pub value_date:    Date,
    pub amount:        Decimal,
    pub currency:      String,
    pub direction:     Direction,     // Debit or Credit from the account's perspective
    pub description:   String,        // raw bank narrative
    pub raw:           serde_json::Value, // original row preserved verbatim
}
```

CSV mapping is driven by a user-supplied `ImportProfile` (which column is the date, amount, description, etc.) stored per data source.

### Stage 2 — Match and classify
Each `ImportedTransaction` is matched against existing postings (to detect duplicates) and optionally against a rule set that maps description patterns to account assignments. Unmatched transactions are surfaced for manual review.

```rust
pub enum MatchOutcome {
    AlreadyImported { journal_entry_id: i32 },
    Classified { debit_account: AccountId, credit_account: AccountId },
    NeedsReview,
}
```

### Stage 3 — Post
Approved transactions are posted as standard journal entries via the existing `post_journal_entry` path. The `external_id` is stored as the `reference` field (AIP-001), making the import idempotent: re-importing the same file skips already-posted entries.

## Key Design Points

- **Idempotency**: the `external_id` is indexed uniquely. Re-importing the same statement has no effect on already-imported rows.
- **CSV format agnosticism**: the `ImportProfile` maps source column names to semantic fields. Multiple profiles can be saved (one per bank/feed).
- **No auto-posting**: Stage 3 requires explicit approval. The pipeline never posts without human or system confirmation of the account assignment. This prevents silent mis-postings.
- **Duplicate detection**: before posting, the service checks for any `ledger_line` whose `reference` matches the incoming `external_id`. If found, the entry is skipped.
- **Partial success**: a batch import processes each row independently inside its own transaction. A failure on row 47 does not roll back rows 1–46.
- **Audit**: every import batch is recorded in an `import_batches` table with the source filename, record count, success count, and failure count.
- The MT940 format (used by most regional banks for SWIFT statement delivery) should be supported in V1 alongside CSV, as it is the primary format for UAE corporate banking.

## Acceptance Criteria

- A CSV file with a configured `ImportProfile` is parsed into `ImportedTransaction` records.
- Re-importing the same file does not create duplicate journal entries.
- A transaction whose `external_id` matches an existing `reference` is returned as `AlreadyImported`.
- Classification rules match on description substring and assign debit/credit accounts.
- Unmatched transactions are returned as `NeedsReview` and not posted.
- An `import_batches` record is created for every import run with accurate counts.
- MT940 files parse correctly for standard UAE bank statement format.
