CREATE TABLE "outbox" (
  "id" bigserial PRIMARY KEY,
  "event_type" varchar NOT NULL,
  "payload" jsonb NOT NULL,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "delivered_at" timestamp
);

CREATE INDEX outbox_undelivered ON outbox (id) WHERE delivered_at IS NULL;
