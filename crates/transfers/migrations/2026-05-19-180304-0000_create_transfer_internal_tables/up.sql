CREATE TYPE "transfer_status" AS ENUM (
  'pending',
  'cancelling',
  'cancelled',
  'completing',
  'completed',
  'failed'
);

CREATE TABLE "transfer_internal" (
  "id" BIGSERIAL PRIMARY KEY,
  "client_id" varchar(64) UNIQUE NOT NULL,
  "from_account_id" bigint NOT NULL,
  "to_account_id" bigint NOT NULL,
  "amount" bigint NOT NULL,
  "status" transfer_status NOT NULL DEFAULT 'pending',
  "created_at" timestamptz NOT NULL DEFAULT now(),
  "updated_at" timestamptz
);
