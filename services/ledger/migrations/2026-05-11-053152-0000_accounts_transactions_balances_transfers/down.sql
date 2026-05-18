-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS "ledger_lines";
DROP TABLE IF EXISTS "balances";
DROP TABLE IF EXISTS "account_blocks";
DROP TABLE IF EXISTS "transfer_internal";
DROP TABLE IF EXISTS "accounts";
DROP TABLE IF EXISTS "journal_entries";

DROP TYPE IF EXISTS "account_type";
DROP TYPE IF EXISTS "transfer_status";