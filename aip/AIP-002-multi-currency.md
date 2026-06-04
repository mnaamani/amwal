# AIP-002 — Multi-Currency Amount Model

**Status:** Draft  
**Area:** Ledger core model  
**Depends on:** AIP-009 (Decimal Precision Arithmetic) — multi-currency exchange rates and conversion postings require fractional amounts that `i64` fils cannot represent. AIP-009 must be completed before this proposal is implemented.

---

## Problem

All amounts in the ledger are typed as `i64` with the implicit assumption that the unit is fils (the smallest AED denomination). This assumption is hardwired into every method signature, every database column, and the transfer state machine. It makes it structurally impossible to:

- Hold or transfer USD, SAR, EUR, or any non-AED currency.
- Record FX conversions with the correct exchange rate.
- Support multi-currency Wakalah investment pools or trade-finance facilities denominated in a foreign currency.
- Represent commodity quantities (gold, sukuk units) separate from their AED equivalent.

## Proposed Enhancement

Introduce an `Amount` type that pairs a decimal number with an explicit currency/commodity code:

```
Amount {
    value:    Decimal,   // arbitrary-precision
    currency: String,    // ISO 4217 code or commodity name, e.g. "AED", "USD", "XAU"
}
```

Each posting carries an `Amount` rather than a bare `i64`. The double-entry invariant is extended: debits must equal credits **per currency** (a single journal entry may be multi-currency only when an explicit conversion posting is present).

A **Price** table records exchange rates:

```
Price {
    date:           Date,
    base_currency:  String,
    quote_currency: String,
    rate:           Decimal,
}
```

Balances are stored and returned as a map of currency → decimal amount rather than a single integer.

## Key Design Points

- The `i64 fils` representation is **not** deprecated for single-currency use; `Amount { value: Decimal::from(fils), currency: "AED" }` is the migration path.
- The balance column in PostgreSQL becomes a `JSONB` map (`{"AED": "10000.00", "USD": "500.00"}`), or a separate `account_balances` table with one row per (account, currency).
- `get_available_balance` must accept a currency parameter; `block_funds` operates on a specific `Amount`.
- Multi-currency journal entries require an explicit conversion posting (e.g. DR USD account, CR AED account with a `@ 3.67 AED` price annotation) so the entry is balanced in each currency independently.
- The `LedgerClient` trait surface changes; a versioned migration plan is required for existing single-currency callers.

## Acceptance Criteria

- An account can hold balances in more than one currency simultaneously.
- A journal entry mixing AED and USD legs is accepted when a price annotation makes it balance per currency.
- `get_account_balance` returns a `Vec<Amount>` (one per held currency).
- Historical price entries can be inserted and queried.
- The zakat calculator is updated to accept a target currency and fetch the AED-equivalent balance via the price table.
