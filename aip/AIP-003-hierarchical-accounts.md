# AIP-003 — Hierarchical Chart of Accounts

**Status:** Draft  
**Area:** Ledger core model

---

## Problem

Accounts are currently identified by an opaque integer `AccountId` and a free-text name with no imposed structure. There is no parent-child relationship, so:

- It is impossible to produce a balance sheet that shows `Assets` as the sum of all asset sub-accounts.
- A P&L cannot roll up `Revenue:FeeIncome:Murabaha` and `Revenue:FeeIncome:Ijara` into `Revenue:FeeIncome`.
- Regulatory reports that aggregate by account category require custom grouping logic in every consuming application.
- A customer setting up a multi-entity chart of accounts cannot express the organisational hierarchy in the ledger itself.

## Proposed Enhancement

Adopt a **colon-separated hierarchical name** as the canonical account identifier alongside the existing integer ID:

```
Assets
Assets:Cash
Assets:Cash:AED
Assets:Financing:Murabaha
Liabilities:CustomerDeposits
Liabilities:CustomerDeposits:Retail
Liabilities:CustomerDeposits:Corporate
Equity:RetainedEarnings
Revenue:FeeIncome:Murabaha
Expenses:Operations:Staff
```

Rules:
- A name segment is `[A-Za-z][A-Za-z0-9_-]*`; segments are joined by `:`.
- Creating a child account implicitly validates that the parent path is consistent with the declared `AccountType` (an `Assets:` root cannot have a `Liability` child).
- The integer `AccountId` is retained for all internal storage and foreign keys; the hierarchical name is a unique, human-visible identifier.

Queries gain a **prefix filter**: `find_accounts_by_prefix("Assets:Cash")` returns all accounts whose name starts with that path.

## Key Design Points

- The `accounts` table gains a `full_name` column (unique, not-null) with a check constraint enforcing the character set.
- A parent's `AccountType` is inferred from the first segment (`Assets` → Asset, `Liabilities` → Liability, etc.) when the account is created at the root level. Sub-accounts inherit the type of their root ancestor and cannot override it.
- `aggregate_balances_by_prefix(prefix)` sums all accounts that are descendants of the given path, returning the total per currency. This is the primitive that powers balance sheets and P&L reports.
- The `create_account` API accepts `full_name` as an optional field; when absent the name is used as a flat leaf account (backwards-compatible).
- Renaming or reparenting an account is explicitly out of scope for V1 to avoid history rewriting concerns.

## Acceptance Criteria

- Accounts can be created with a full hierarchical name.
- `find_accounts_by_prefix("Liabilities:CustomerDeposits")` returns all matching sub-accounts.
- `aggregate_balances_by_prefix` returns the correct sum across all descendants.
- The trial balance report uses prefix aggregation to produce standard five-bucket output (Assets, Liabilities, Equity, Revenue, Expenses) without hardcoded account IDs.
- Existing accounts created without a hierarchical name remain accessible by integer ID with no migration required.
