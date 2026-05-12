CREATE TYPE "account_type" AS ENUM (
  'asset',
  'liability',
  'equity',
  'revenue',
  'expense'
);

CREATE TYPE "transfer_status" AS ENUM (
  'pending',
  'canceled',
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
  "tx" integer,
  "account" integer,
  "debit" integer,
  "credit" integer,
  "created_at" timestamp NOT NULL DEFAULT (now())
);

CREATE TABLE "transactions" (
  "id" SERIAL PRIMARY KEY,
  "commited" bool DEFAULT false,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);

CREATE TABLE "balances" (
  "account_id" integer PRIMARY KEY,
  "commited_balance" integer,
  "blocked" integer,
  "updated_at" timestamp NOT NULL DEFAULT (now())
);

CREATE TABLE "account_blocks" (
  "id" SERIAL PRIMARY KEY,
  "account_id" integer,
  "amount" integer,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "transaction_id" integer
);

CREATE TABLE "transfer_internal" (
  "id" SERIAL PRIMARY KEY,
  "transaction_id" integer,
  "from_account_id" integer,
  "to_account_id" integer,
  "amount" integer,
  "initiated_at" timestamp DEFAULT (now()),
  "completed_at" timestamp,
  "status" transfer_status
);

ALTER TABLE "movements" ADD FOREIGN KEY ("tx") REFERENCES "transactions" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "movements" ADD FOREIGN KEY ("account") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "balances" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "account_blocks" ADD FOREIGN KEY ("transaction_id") REFERENCES "transactions" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("transaction_id") REFERENCES "transactions" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("from_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

ALTER TABLE "transfer_internal" ADD FOREIGN KEY ("to_account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;
