-- SQL dump generated using DBML (dbml.dbdiagram.io)
-- Database: PostgreSQL
-- Generated at: 2026-05-19T18:27:52.724Z

CREATE TYPE "account_type" AS ENUM (
  'asset',
  'liability',
  'equity',
  'revenue',
  'expense'
);

CREATE TABLE "accounts" (
  "id" SERIAL PRIMARY KEY,
  "client_id" varchar(32) UNIQUE NOT NULL,
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
  "client_id" varchar(32) UNIQUE NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

CREATE TABLE "balances" (
  "account_id" integer PRIMARY KEY,
  "balance" bigint NOT NULL DEFAULT 0,
  "updated_at" timestamp NOT NULL
);

CREATE TABLE "account_blocks" (
  "id" SERIAL PRIMARY KEY,
  "client_id" varchar(32) UNIQUE NOT NULL,
  "account_id" integer NOT NULL,
  "amount" bigint NOT NULL,
  "released" bool NOT NULL DEFAULT false,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("journal_entry_id") REFERENCES "journal_entries" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("account") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "balances" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;
