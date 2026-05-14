## double-entry book keeping crash course
https://accountingstuff.com/accounting-basics/accounting-equation

## High‑level objects
- Account:accounts	Every ledger account the bank owns (cash, customer deposits, loans, fees, equity, etc.).

Think about:
For incoming transfers from outside the bank, which account do they go into?
   Clearning bank/Corresponding bank account/Cash (deposit).
Where to store balance, or that is always computed from transactions?.. -> sql trigger update separate balances table, or should it be database agnostic -> have a service poll/react to events when tx are posted to update the balances in a separate database/table?
store it in separate table, update it with every transaction..(atomic, safer)
How to handle reservation/blocking of funds during an async transfer process.

Account types(enum): ASSET, LIABILITY, EQUITY, REVENUE, EXPENSE

Quick “rule of thumb”
If the bank owes the customer money → Liability (deposits, refunds, pending reimbursements).
If the customer owes the bank money → Asset (loans, accrued interest, fees due).
If the transaction records earnings from the customer → Revenue (interest earned, service fees).
If the transaction records costs paid because of the customer → Expense (interest paid on deposits, service‑cost allocations).

Do we need concept of parent accounts/sub-accounts.
Ownership model, differentiate a customer account from bank internal accounts?


- Transaction (Journal entry?): Date, description, reference, status
1-many mapping to debit/credit records

- Movement (Ledger line?) - the debits/credits: One row per leg: account_id, debit_amount, credit_amount, optional currency(or is that better on account?), memo.
1-1 mapping to Transaction record

When do we 'post' transactions? After reconciliation/confirmation from external services/partners? Do we post individual movements or the transaction as a whole?

## Trial Balance / Integrity check
Normal Balance:
ASSET	balance = total_debits − total_credits
EXPENSE	balance = total_debits − total_credits
LIABILITY	balance = total_credits − total_debits
EQUITY	balance = total_credits − total_debits
REVENUE	balance = total_credits − total_debits
Is it valid for an account to have negative balance (overdraft)?

ASSET_balance + EXPENSE_balance = LIABILITY_balance + EQUITY_balance + REVENUE_balance

ASSET and EXPENSE are “debit‑nature” accounts; they sit on the left side of the equation.
LIABILITY, EQUITY, and REVENUE are “credit‑nature” accounts; they sit on the right side.

Important: Before closing vs. after closing
During the period (Revenue & Expense still have balances)	Expanded equation: A + E = L + Eq + R

After period‑end closing (Revenue & Expense have been zeroed out into Equity)	Classic equation: Assets = Liabilities + Equity

So for a live banking application that operates continuously, Formula 2 (the expanded equation) is the one you want as your daily system health check.

## Tech stack for the Ledger/Account Service
Compile-time safety is non-negotiable, and the project will live in production for years and correctness matters, so picking Diesel over SeaORM I think is the right call.

Rust + Diesel + PostgresDB
https://github.com/actix/actix-web web framework
https://github.com/juhaku/utoipa
use OpenAPI documentation + swaggerUi
GraphQL? I don't think we need GraphQL for the ledger? GRPC? ->
  (Think hexagonal - make the library agnostic to REST/GraphQL/GRPC - build each as an adapter)


## Database Schema Design/Migrations etc..
Pure Diesel?
Prisma for schema design, migrations? Or use a tool like https://dbdiagram.io define Database schema in
DBML https://dbml.dbdiagram.io/home - use cli `npx -p @dbml/cli dbml2sql dbml/main.dml --postgres -o migrations/new_migration/up.sql`
Diesel can generate Rust types from the database schema.
`diesel print-schema` -> schema.rs

Other services in the system can have totally different stack.

SeaORM for services with more complex data structures spread over multiple tables, apps/services need more complex queries.. other options: https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3
