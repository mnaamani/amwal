-- Your SQL goes here
-- Enforces two invariants on every account_blocks INSERT:
-- 1. account_id must match from_account_id on the referenced transfer (cross-table integrity)
-- 2. the referenced transfer must still be pending (idempotency guard)
CREATE OR REPLACE FUNCTION check_account_block_integrity()
RETURNS trigger AS $$
DECLARE
  v_from_account_id integer;
  v_status transfer_status;
BEGIN
  SELECT from_account_id, status
  INTO v_from_account_id, v_status
  FROM transfer_internal
  WHERE id = NEW.transfer_id;

  IF v_from_account_id IS NULL THEN
    RAISE EXCEPTION 'transfer % not found', NEW.transfer_id;
  END IF;

  IF v_from_account_id != NEW.account_id THEN
    RAISE EXCEPTION
      'account_blocks.account_id (%) does not match transfer_internal.from_account_id (%) for transfer %',
      NEW.account_id, v_from_account_id, NEW.transfer_id;
  END IF;

  IF v_status != 'pending' THEN
    RAISE EXCEPTION
      'cannot block funds for transfer % with status %', NEW.transfer_id, v_status;
  END IF;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER enforce_account_block_integrity
BEFORE INSERT ON account_blocks
FOR EACH ROW EXECUTE FUNCTION check_account_block_integrity();
