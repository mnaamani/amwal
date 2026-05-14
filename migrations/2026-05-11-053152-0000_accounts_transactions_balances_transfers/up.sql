-- SQL dump generated using DBML (dbml.dbdiagram.io)
-- Database: PostgreSQL
-- Generated at: 2026-05-14T05:30:07.496Z

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

CREATE TABLE "movements" (
  "id" SERIAL PRIMARY KEY,
  "tx" integer NOT NULL,
  "account" integer NOT NULL,
  "debit" integer NOT NULL DEFAULT 0,
  "credit" integer NOT NULL DEFAULT 0,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  CONSTRAINT "movements_valid_leg" CHECK (debit >= 0 AND credit >= 0 AND (debit > 0) != (credit > 0))
);

CREATE TABLE "transactions" (
  "id" SERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

CREATE TABLE "balances" (
  "account_id" integer PRIMARY KEY,
  "balance" integer NOT NULL DEFAULT 0,
  "updated_at" timestamp NOT NULL DEFAULT (now())
);

CREATE TABLE "account_blocks" (
  "id" SERIAL PRIMARY KEY,
  "account_id" integer NOT NULL,
  "amount" integer NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "transfer_id" integer UNIQUE NOT NULL
);

CREATE TABLE "transfer_internal" (
  "id" SERIAL PRIMARY KEY,
  "transaction_id" integer,
  "from_account_id" integer NOT NULL,
  "to_account_id" integer NOT NULL,
  "amount" integer NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "completed_at" timestamp,
  "status" transfer_status NOT NULL
);

ALTER TABLE "movements" ADD FOREIGN KEY ("tx") REFERENCES "transactions" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "movements" ADD FOREIGN KEY ("account") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "balances" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("transfer_id") REFERENCES "transfer_internal" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("transaction_id") REFERENCES "transactions" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("from_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("to_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;
