-- Your SQL goes here
CREATE TYPE "account_type" AS ENUM (
  'asset',
  'liability',
  'equity',
  'revenue',
  'expense'
);

CREATE TABLE "ledger_accounts"(
	"id" SERIAL PRIMARY KEY,
	"type" account_type NOT NULL,
	"name" varchar NOT NULL,
	"active" bool NOT NULL DEFAULT FALSE,
	"created_at" timestamp NOT NULL
);
