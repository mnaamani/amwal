-- This file should undo anything in `up.sql`
ALTER TABLE ledger_accounts ALTER COLUMN created_at DROP DEFAULT;