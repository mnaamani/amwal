-- SQL dump generated using DBML (dbml.dbdiagram.io)
-- Database: PostgreSQL
-- Generated at: 2026-05-19T18:26:38.019Z

CREATE TYPE "transfer_status" AS ENUM (
  'pending',
  'cancelling',
  'cancelled',
  'completing',
  'completed',
  'failed'
);

CREATE TABLE "transfer_internal" (
  "id" SERIAL PRIMARY KEY,
  "client_id" varchar(32) UNIQUE NOT NULL,
  "from_account_id" integer NOT NULL,
  "to_account_id" integer NOT NULL,
  "amount" bigint NOT NULL,
  "status" transfer_status NOT NULL DEFAULT 'pending',
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp
);
