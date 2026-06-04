# AIP-009 — Decimal Precision Arithmetic

**Status:** Draft  
**Area:** Ledger core model / data types

---

## Problem

All monetary amounts are stored and computed as `i64` integers in fils (the smallest AED subunit, 1/100 of a dirham). This representation is adequate for whole-fils arithmetic in a single currency, but it breaks down in several commercially critical scenarios:

- **Exchange rates**: 1 USD = 3.6725 AED. Applying this rate to a USD 1,000 amount yields AED 3,672.50 — a value that cannot be exactly represented in fils.
- **Profit-sharing (Mudarabah)**: A 67.3% / 32.7% split of a profit amount almost always produces a non-integer fils figure. Rounding both legs independently can violate the double-entry invariant.
- **Zakat calculation**: 2.5% of an account balance routinely produces a fractional fils amount. The current implementation uses integer division (`/ 40`), which silently truncates.
- **Periodic fee schedules**: Ijara and Murabaha instalments calculated from an annual rate produce repeating decimals that must be tracked to sufficient precision before rounding for the final disbursement.
- **Regulatory reporting**: CBUAE and AAOIFI reports require amounts stated to two or four decimal places depending on the instrument; a fils-rounded figure may not satisfy the stated precision.

## Proposed Enhancement

Replace all `i64` amount fields with the `rust_decimal::Decimal` type, which provides:

- 28 significant digits of precision.
- Exact decimal representation (no floating-point rounding error).
- Configurable rounding modes (`ROUND_HALF_EVEN` is the standard for financial arithmetic under IEEE 754-2008 and AAOIFI).
- Arithmetic operators that propagate scale correctly.

### Migration plan

| Layer | Change |
|-------|--------|
| `ledger-api` | `i64` amount fields → `Decimal`; `NonZeroU64` in `Posting` → `NonZeroDecimal` (newtype) |
| `ledger` domain | `Balance.balance: i64` → `Decimal`; `Posting` arms carry `Decimal` |
| PostgreSQL schema | `BIGINT` amount columns → `NUMERIC(28, 10)` |
| `zakat-calculator` | Zakat rate (`/ 40`) → `Decimal::new(1, 0) / Decimal::new(40, 0)` with `ROUND_HALF_EVEN` |
| `transfers` | `amount: i64` → `Decimal` |

### Rounding policy

Rounding is **only applied at display or disbursement time**, never mid-calculation. The ledger stores values at full precision. When a posting must be stated in whole fils for a payment instruction, the caller applies the rounding and posts any residual to an explicit rounding account (`Expenses:Rounding` or `Revenue:Rounding`) so the double-entry invariant is maintained to the last unit.

## Key Design Points

- `rust_decimal` is a pure-Rust crate with no unsafe code and zero transitive dependencies beyond `serde`. Adding it is a one-line `Cargo.toml` change.
- `Decimal` serialises to JSON as a string (`"3672.50"`) to avoid floating-point loss in API responses.
- Diesel supports `NUMERIC` columns via the `diesel::data_types::BigDecimal` or `rust_decimal` feature flag; no custom SQL type mapping is required.
- The `NonZeroU64` constraint on `Posting` amounts is replaced by a `PositiveDecimal` newtype that enforces `value > Decimal::ZERO` at construction time, preserving the original invariant.
- Existing tests that use literal `i64` amounts are migrated to `Decimal::new(value, 0)` with no semantic change for integer-valued test cases.

## Acceptance Criteria

- `Decimal::new(1000, 0) * Decimal::new(36725, 4)` (1000 USD × 3.6725) yields `Decimal::new(36725, 1)` (3672.5 AED) without truncation.
- The zakat calculator produces `Decimal::new(25, 3)` (0.025) of a balance, not a truncated integer.
- A balanced journal entry where individual leg amounts involve repeating decimals passes the double-entry check without a rounding residual.
- PostgreSQL stores and retrieves the full precision without loss for values up to 18 integer digits and 10 fractional digits.
- All existing integer-valued tests continue to pass after the migration.
