-- Task 43/44: server-owned AI quota and append-only credit accounting.
-- This migration is intentionally forward-only and is compatible with the
-- deployed 20260818093300 + task32 schema (reference_id is text).

ALTER TABLE public.credit_ledger
  ADD COLUMN IF NOT EXISTS reason text NOT NULL DEFAULT 'unspecified';

ALTER TABLE public.ai_usage
  ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'completed',
  ADD COLUMN IF NOT EXISTS prompt_version text;

CREATE UNIQUE INDEX IF NOT EXISTS credit_ledger_user_idempotency_key
  ON public.credit_ledger(user_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ai_usage_user_request_id
  ON public.ai_usage(user_id, request_id)
  WHERE request_id IS NOT NULL;

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
  v_balance integer;
BEGIN
  IF p_user_id IS NULL
     OR p_amount IS NULL
     OR p_amount <= 0
     OR p_amount > 100000000
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid credit reservation';
  END IF;

  -- Serialize all credit mutations for one user without making the ledger
  -- mutable. The advisory lock is released automatically at transaction end.
  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  -- A retry of the same request is already safe. The eventual commit/refund
  -- is also idempotent, so returning true does not create another reservation.
  IF EXISTS (
    SELECT 1 FROM public.credit_ledger
    WHERE user_id = p_user_id
      AND reference_id = p_request_id
      AND transaction_type IN ('reservation', 'usage_commit', 'refund')
  ) THEN
    RETURN true;
  END IF;

  SELECT COALESCE(SUM(amount), 0)::integer
    INTO v_balance
    FROM public.credit_ledger
   WHERE user_id = p_user_id;

  IF v_balance < p_amount THEN
    RETURN false;
  END IF;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id, idempotency_key
  ) VALUES (
    p_user_id,
    -p_amount,
    'AI request reservation',
    'reservation',
    p_request_id,
    'reserve:' || p_request_id
  )
  ON CONFLICT (user_id, idempotency_key) DO NOTHING;

  RETURN true;
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

  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  IF EXISTS (
    SELECT 1 FROM public.credit_ledger
    WHERE user_id = p_user_id
      AND idempotency_key = 'commit:' || p_request_id
  ) THEN
    RETURN true;
  END IF;

  SELECT -amount INTO v_reserved
    FROM public.credit_ledger
   WHERE user_id = p_user_id
     AND reference_id = p_request_id
     AND transaction_type = 'reservation'
   ORDER BY created_at DESC
   LIMIT 1;

  IF v_reserved IS NULL THEN
    RETURN false;
  END IF;

  -- Neutralize the reservation, then apply the actual usage. This keeps the
  -- ledger append-only and makes the net balance exactly -p_amount_used.
  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id, idempotency_key
  ) VALUES (
    p_user_id,
    v_reserved,
    'Release AI request reservation',
    'refund',
    p_request_id,
    'release:' || p_request_id
  )
  ON CONFLICT (user_id, idempotency_key) DO NOTHING;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id, idempotency_key
  ) VALUES (
    p_user_id,
    -p_amount_used,
    'AI request usage commit',
    'usage_commit',
    p_request_id,
    'commit:' || p_request_id
  )
  ON CONFLICT (user_id, idempotency_key) DO NOTHING;

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
    '{}'::jsonb
  )
  ON CONFLICT (user_id, request_id) DO NOTHING;

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
  v_reserved integer;
BEGIN
  IF p_user_id IS NULL
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid credit refund';
  END IF;

  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  IF EXISTS (
    SELECT 1 FROM public.credit_ledger
    WHERE user_id = p_user_id
      AND idempotency_key = 'failure-refund:' || p_request_id
  ) THEN
    RETURN true;
  END IF;

  SELECT -amount INTO v_reserved
    FROM public.credit_ledger
   WHERE user_id = p_user_id
     AND reference_id = p_request_id
     AND transaction_type = 'reservation'
   ORDER BY created_at DESC
   LIMIT 1;

  IF v_reserved IS NULL THEN
    RETURN false;
  END IF;

  INSERT INTO public.credit_ledger(
    user_id, amount, reason, transaction_type, reference_id, idempotency_key
  ) VALUES (
    p_user_id,
    v_reserved,
    'AI request failure refund',
    'refund',
    p_request_id,
    'failure-refund:' || p_request_id
  )
  ON CONFLICT (user_id, idempotency_key) DO NOTHING;

  RETURN true;
END;
$$;

REVOKE ALL ON FUNCTION public.reserve_credits(uuid, integer, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.commit_credits(uuid, integer, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.commit_credits(uuid, integer, text, text, text, numeric) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.refund_credits(uuid, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.reserve_credits(uuid, integer, text) TO service_role;
GRANT EXECUTE ON FUNCTION public.commit_credits(uuid, integer, text) TO service_role;
GRANT EXECUTE ON FUNCTION public.commit_credits(uuid, integer, text, text, text, numeric) TO service_role;
GRANT EXECUTE ON FUNCTION public.refund_credits(uuid, text) TO service_role;
