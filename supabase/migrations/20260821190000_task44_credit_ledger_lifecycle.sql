-- Task 44: append-only credit ledger with an explicit reservation lifecycle.
-- The reservation table is a server-only state index; the ledger remains the
-- audit source of truth and is never updated or deleted.

ALTER TABLE public.credit_ledger
  ADD COLUMN IF NOT EXISTS metadata jsonb NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE public.credit_ledger
  DROP CONSTRAINT IF EXISTS credit_ledger_metadata_object;

ALTER TABLE public.credit_ledger
  ADD CONSTRAINT credit_ledger_metadata_object
    CHECK (jsonb_typeof(metadata) = 'object');

CREATE TABLE IF NOT EXISTS public.credit_reservations (
  user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  request_id text NOT NULL,
  reserved_amount integer NOT NULL CHECK (reserved_amount > 0),
  actual_amount integer CHECK (actual_amount IS NULL OR actual_amount >= 0),
  status text NOT NULL DEFAULT 'reserved'
    CHECK (status IN ('reserved', 'committed', 'refunded', 'expired')),
  expires_at timestamptz NOT NULL DEFAULT (now() + interval '5 minutes'),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, request_id)
);

CREATE INDEX IF NOT EXISTS credit_reservations_expiry
  ON public.credit_reservations(status, expires_at);

ALTER TABLE public.credit_reservations ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON public.credit_reservations FROM PUBLIC, anon, authenticated;

CREATE OR REPLACE FUNCTION public.reconcile_credit_reservations(p_user_id uuid)
RETURNS integer
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_reservation record;
  v_reconciled integer := 0;
BEGIN
  IF p_user_id IS NULL THEN
    RAISE EXCEPTION 'invalid reconciliation user';
  END IF;

  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  FOR v_reservation IN
    SELECT request_id, reserved_amount
      FROM public.credit_reservations
     WHERE user_id = p_user_id
       AND status = 'reserved'
       AND expires_at <= pg_catalog.now()
     FOR UPDATE
  LOOP
    INSERT INTO public.credit_ledger(
      user_id, amount, reason, transaction_type, reference_id,
      idempotency_key, metadata
    ) VALUES (
      p_user_id,
      v_reservation.reserved_amount,
      'Expired AI request reservation release',
      'release',
      v_reservation.request_id,
      'expiry-release:' || v_reservation.request_id,
      jsonb_build_object('operation', 'credit_reservation_expiry')
    )
    ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING;

    UPDATE public.credit_reservations
       SET status = 'expired', updated_at = pg_catalog.now()
     WHERE user_id = p_user_id AND request_id = v_reservation.request_id;
    v_reconciled := v_reconciled + 1;
  END LOOP;
  RETURN v_reconciled;
END;
$$;

CREATE OR REPLACE FUNCTION public.reserve_credits(
  p_user_id uuid,
  p_amount integer,
  p_request_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_balance bigint;
  v_status text;
  v_expiry timestamptz;
BEGIN
  IF p_user_id IS NULL
     OR p_amount IS NULL
     OR p_amount <= 0
     OR p_amount > 100000000
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid credit reservation';
  END IF;

  PERFORM public.reconcile_credit_reservations(p_user_id);

  SELECT status, expires_at
    INTO v_status, v_expiry
    FROM public.credit_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_status IS NOT NULL THEN
    IF v_status = 'reserved' AND v_expiry > pg_catalog.now() THEN
      RETURN true;
    END IF;
    RETURN false;
  END IF;

  SELECT COALESCE(SUM(amount), 0)::bigint
    INTO v_balance
    FROM public.credit_ledger
   WHERE user_id = p_user_id;

  IF v_balance < p_amount THEN
    RETURN false;
  END IF;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id,
    idempotency_key, metadata
  ) VALUES (
    p_user_id,
    -p_amount,
    'AI request reservation',
    'reservation',
    p_request_id,
    'reserve:' || p_request_id,
    jsonb_build_object('operation', 'credit_reserve')
  )
  ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING;

  INSERT INTO public.credit_reservations(user_id, request_id, reserved_amount)
  VALUES (p_user_id, p_request_id, p_amount);
  RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION public.get_credit_reservation(
  p_user_id uuid,
  p_request_id text
)
RETURNS TABLE(
  status text,
  reserved_amount integer,
  actual_amount integer,
  expires_at timestamptz
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = ''
AS $$
  SELECT r.status, r.reserved_amount, r.actual_amount, r.expires_at
    FROM public.credit_reservations AS r
   WHERE r.user_id = p_user_id AND r.request_id = p_request_id;
$$;

CREATE OR REPLACE FUNCTION public.get_credit_balance(p_user_id uuid)
RETURNS bigint
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_balance bigint;
BEGIN
  IF p_user_id IS NULL THEN
    RAISE EXCEPTION 'invalid balance user';
  END IF;
  PERFORM public.reconcile_credit_reservations(p_user_id);
  SELECT COALESCE(SUM(amount), 0)::bigint
    INTO v_balance
    FROM public.credit_ledger
   WHERE user_id = p_user_id;
  RETURN v_balance;
END;
$$;

CREATE OR REPLACE FUNCTION public.commit_credits(
  p_user_id uuid,
  p_amount_used integer,
  p_request_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_status text;
  v_reserved integer;
BEGIN
  IF p_user_id IS NULL
     OR p_amount_used IS NULL
     OR p_amount_used < 0
     OR p_amount_used > 100000000
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid credit commit';
  END IF;

  PERFORM public.reconcile_credit_reservations(p_user_id);

  SELECT status, reserved_amount
    INTO v_status, v_reserved
    FROM public.credit_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_status = 'committed' THEN
    RETURN true;
  END IF;
  IF v_status IS DISTINCT FROM 'reserved' OR p_amount_used > v_reserved THEN
    RETURN false;
  END IF;

  IF v_reserved > p_amount_used THEN
    INSERT INTO public.credit_ledger(
      user_id, amount, reason, transaction_type, reference_id,
      idempotency_key, metadata
    ) VALUES (
      p_user_id,
      v_reserved - p_amount_used,
      'Release unused AI reservation',
      'release',
      p_request_id,
      'release:' || p_request_id,
      jsonb_build_object('operation', 'credit_release_unused')
    )
    ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING;
  END IF;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id,
    idempotency_key, metadata
  ) VALUES (
    p_user_id,
    -p_amount_used,
    'AI request usage commit',
    'usage_commit',
    p_request_id,
    'commit:' || p_request_id,
    jsonb_build_object('operation', 'credit_commit')
  )
  ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING;

  UPDATE public.credit_reservations
     SET status = 'committed', actual_amount = p_amount_used, updated_at = pg_catalog.now()
   WHERE user_id = p_user_id AND request_id = p_request_id;
  RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION public.commit_credits(
  p_user_id uuid,
  p_amount_used integer,
  p_request_id text,
  p_provider text,
  p_model text,
  p_cost_estimate numeric
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_committed boolean;
BEGIN
  v_committed := public.commit_credits(p_user_id, p_amount_used, p_request_id);
  IF NOT v_committed THEN
    RETURN false;
  END IF;

  INSERT INTO public.ai_usage(
    user_id, provider, model, tokens_used, cost_estimate,
    operation_type, request_id, status, metadata
  ) VALUES (
    p_user_id,
    left(COALESCE(p_provider, 'unknown'), 64),
    left(COALESCE(p_model, 'unknown'), 128),
    greatest(COALESCE(p_amount_used, 0), 0),
    greatest(COALESCE(p_cost_estimate, 0), 0),
    'credit_commit',
    p_request_id,
    'completed',
    jsonb_build_object(
      'provider', left(COALESCE(p_provider, 'unknown'), 64),
      'model', left(COALESCE(p_model, 'unknown'), 128),
      'operation', 'credit_commit'
    )
  )
  ON CONFLICT (user_id, request_id) WHERE request_id IS NOT NULL DO NOTHING;

  RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION public.refund_credits(
  p_user_id uuid,
  p_request_id text
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_status text;
  v_reserved integer;
BEGIN
  IF p_user_id IS NULL
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid credit refund';
  END IF;

  PERFORM public.reconcile_credit_reservations(p_user_id);

  SELECT status, reserved_amount
    INTO v_status, v_reserved
    FROM public.credit_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_status IN ('refunded', 'expired') THEN
    RETURN true;
  END IF;
  IF v_status IS DISTINCT FROM 'reserved' THEN
    RETURN false;
  END IF;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id,
    idempotency_key, metadata
  ) VALUES (
    p_user_id,
    v_reserved,
    'AI request failure refund',
    'refund',
    p_request_id,
    'failure-refund:' || p_request_id,
    jsonb_build_object('operation', 'credit_refund')
  )
  ON CONFLICT (user_id, idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING;

  UPDATE public.credit_reservations
     SET status = 'refunded', updated_at = pg_catalog.now()
   WHERE user_id = p_user_id AND request_id = p_request_id;
  RETURN true;
END;
$$;

REVOKE ALL ON FUNCTION public.reconcile_credit_reservations(uuid) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.reconcile_credit_reservations(uuid) TO service_role;
REVOKE ALL ON FUNCTION public.reserve_credits(uuid, integer, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.reserve_credits(uuid, integer, text) TO service_role;
REVOKE ALL ON FUNCTION public.get_credit_reservation(uuid, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.get_credit_reservation(uuid, text) TO service_role;
REVOKE ALL ON FUNCTION public.get_credit_balance(uuid) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.get_credit_balance(uuid) TO service_role;
REVOKE ALL ON FUNCTION public.commit_credits(uuid, integer, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.commit_credits(uuid, integer, text) TO service_role;
REVOKE ALL ON FUNCTION public.commit_credits(uuid, integer, text, text, text, numeric)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.commit_credits(uuid, integer, text, text, text, numeric)
  TO service_role;
REVOKE ALL ON FUNCTION public.refund_credits(uuid, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.refund_credits(uuid, text) TO service_role;
