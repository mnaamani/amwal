-- SQL dump generated using DBML (dbml.dbdiagram.io)
-- Database: PostgreSQL
-- Generated at: 2026-05-15T06:35:57.957Z

CREATE TYPE "account_type" AS ENUM (
  'asset',
  'liability',
  'equity',
  'revenue',
  'expense'
);

CREATE TYPE "transfer_status" AS ENUM (
  'pending',
  'cancelled',
  'completed'
);

CREATE TABLE "accounts" (
  "id" SERIAL PRIMARY KEY,
  "account_type" account_type NOT NULL,
  "name" varchar NOT NULL,
  "active" bool NOT NULL DEFAULT false,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

CREATE TABLE "ledger_lines" (
  "id" SERIAL PRIMARY KEY,
  "journal_entry_id" integer NOT NULL,
  "account" integer NOT NULL,
  "debit" bigint NOT NULL DEFAULT 0,
  "credit" bigint NOT NULL DEFAULT 0,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  CONSTRAINT "ledger_lines_valid_leg" CHECK (debit >= 0 AND credit >= 0 AND (debit > 0) != (credit > 0))
);

CREATE TABLE "journal_entries" (
  "id" SERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

CREATE TABLE "balances" (
  "account_id" integer PRIMARY KEY,
  "balance" bigint NOT NULL DEFAULT 0,
  "updated_at" timestamp NOT NULL DEFAULT (now())
);

CREATE TABLE "account_blocks" (
  "id" SERIAL PRIMARY KEY,
  "account_id" integer NOT NULL,
  "amount" bigint NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "transfer_id" integer UNIQUE NOT NULL
);

CREATE TABLE "transfer_internal" (
  "id" SERIAL PRIMARY KEY,
  "journal_entry_id" integer,
  "from_account_id" integer NOT NULL,
  "to_account_id" integer NOT NULL,
  "amount" bigint NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "completed_at" timestamp,
  "status" transfer_status NOT NULL
);

ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("journal_entry_id") REFERENCES "journal_entries" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("account") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "balances" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("transfer_id") REFERENCES "transfer_internal" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("journal_entry_id") REFERENCES "journal_entries" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("from_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("to_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;
