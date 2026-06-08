CREATE TYPE "account_type" AS ENUM (
  'asset',
  'liability',
  'equity',
  'revenue',
  'expense'
);

CREATE TYPE "posting_direction" AS ENUM (
  'debit',
  'credit'
);

CREATE TABLE "accounts" (
  "id" BIGSERIAL PRIMARY KEY,
  "client_id" varchar(64) UNIQUE NOT NULL,
  "account_type" account_type NOT NULL,
  "name" varchar NOT NULL,
  "active" bool NOT NULL DEFAULT false,
  "debits_posted" bigint NOT NULL DEFAULT 0,
  "credits_posted" bigint NOT NULL DEFAULT 0,
  "amount_pending" bigint NOT NULL DEFAULT 0,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "journal_entries" (
  "id" BIGSERIAL PRIMARY KEY,
  "client_id" varchar(64) UNIQUE NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "ledger_lines" (
  "id" BIGSERIAL PRIMARY KEY,
  "journal_entry_id" bigint NOT NULL,
  "account" bigint NOT NULL,
  "amount" bigint NOT NULL,
  "direction" posting_direction NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT "ledger_lines_positive_amount" CHECK (amount > 0)
);

CREATE TABLE "account_blocks" (
  "id" BIGSERIAL PRIMARY KEY,
  "client_id" varchar(64) UNIQUE NOT NULL,
  "account_id" bigint NOT NULL,
  "amount" bigint NOT NULL,
  "released" bool NOT NULL DEFAULT false,
  "created_at" timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE "outbox" (
  "id" BIGSERIAL PRIMARY KEY,
  "event_type" varchar NOT NULL,
  "payload" jsonb NOT NULL,
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "delivered_at" timestamptz
);

CREATE INDEX outbox_undelivered ON outbox (id) WHERE delivered_at IS NULL;

ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("journal_entry_id") REFERENCES "journal_entries" ("id") DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE "ledger_lines" ADD FOREIGN KEY ("account") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;
ALTER TABLE "account_blocks" ADD FOREIGN KEY ("account_id") REFERENCES "accounts" ("id") DEFERRABLE INITIALLY IMMEDIATE;

-- Immutability: prevent any modification of committed journal data.
CREATE OR REPLACE FUNCTION prevent_modification()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION '% rows are immutable', TG_TABLE_NAME;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER journal_entries_immutable
  BEFORE UPDATE OR DELETE ON journal_entries
  FOR EACH ROW EXECUTE FUNCTION prevent_modification();

CREATE TRIGGER ledger_lines_immutable
  BEFORE UPDATE OR DELETE ON ledger_lines
  FOR EACH ROW EXECUTE FUNCTION prevent_modification();
