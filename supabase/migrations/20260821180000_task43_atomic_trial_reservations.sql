-- Task 43: reserve trial quota before the provider call and finalize usage
-- atomically after canonical output validation.

CREATE TABLE IF NOT EXISTS public.ai_quota_reservations (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE NOT NULL,
  request_id text NOT NULL,
  status text NOT NULL DEFAULT 'reserved'
    CHECK (status IN ('reserved', 'completed', 'refunded', 'expired')),
  expires_at timestamptz NOT NULL DEFAULT (now() + interval '2 minutes'),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (user_id, request_id)
);

CREATE INDEX IF NOT EXISTS ai_quota_reservations_user_status
  ON public.ai_quota_reservations(user_id, status, expires_at);

ALTER TABLE public.ai_quota_reservations ENABLE ROW LEVEL SECURITY;
REVOKE ALL ON public.ai_quota_reservations FROM PUBLIC, anon, authenticated;

CREATE OR REPLACE FUNCTION public.reserve_ai_quota(
  p_user_id uuid,
  p_request_id text
)
RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_existing_status text;
  v_existing_expiry timestamptz;
  v_count integer;
  v_window_count integer;
  v_active_reservations integer;
  v_limit integer := 20;
BEGIN
  IF p_user_id IS NULL
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid quota reservation';
  END IF;

  INSERT INTO public.trial_usage(user_id)
  VALUES (p_user_id)
  ON CONFLICT (user_id) DO NOTHING;

  -- One account lock covers cleanup, quota checks and the reservation insert.
  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  SELECT status, expires_at
    INTO v_existing_status, v_existing_expiry
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_existing_status IS NOT NULL THEN
    IF v_existing_status = 'completed' THEN
      RETURN 'completed';
    END IF;
    IF v_existing_status = 'reserved' AND v_existing_expiry > pg_catalog.now() THEN
      RETURN 'in_flight';
    END IF;
    IF v_existing_status = 'reserved' THEN
      UPDATE public.ai_quota_reservations
         SET status = 'expired', updated_at = pg_catalog.now()
       WHERE user_id = p_user_id AND request_id = p_request_id;
      RETURN 'expired';
    END IF;
    RETURN v_existing_status;
  END IF;

  UPDATE public.ai_quota_reservations
     SET status = 'expired', updated_at = pg_catalog.now()
   WHERE user_id = p_user_id
     AND status = 'reserved'
     AND expires_at <= pg_catalog.now();

  SELECT requests_count
    INTO v_count
    FROM public.trial_usage
   WHERE user_id = p_user_id
   FOR UPDATE;

  SELECT count(*)::integer
    INTO v_active_reservations
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id
     AND status = 'reserved'
     AND expires_at > pg_catalog.now();

  SELECT count(*)::integer
    INTO v_window_count
    FROM public.ai_usage
   WHERE user_id = p_user_id
     AND status = 'completed'
     AND created_at > pg_catalog.now() - interval '1 minute';

  IF EXISTS (
    SELECT 1
      FROM public.entitlements
     WHERE user_id = p_user_id
       AND (expires_at IS NULL OR expires_at > pg_catalog.now())
  ) THEN
    v_limit := 1000;
  END IF;

  IF v_count + v_active_reservations >= v_limit THEN
    RETURN 'quota_exceeded';
  END IF;
  IF v_window_count + v_active_reservations >= 5 THEN
    RETURN 'rate_limited';
  END IF;

  INSERT INTO public.ai_quota_reservations(user_id, request_id)
  VALUES (p_user_id, p_request_id);
  RETURN 'reserved';
END;
$$;

CREATE OR REPLACE FUNCTION public.finalize_ai_quota(
  p_user_id uuid,
  p_request_id text,
  p_provider text,
  p_model text,
  p_operation_type text,
  p_input_chars integer,
  p_tokens_used integer DEFAULT 0,
  p_cost_estimate numeric DEFAULT 0,
  p_prompt_version text DEFAULT NULL,
  p_latency_ms integer DEFAULT NULL,
  p_response jsonb DEFAULT '{}'::jsonb
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_status text;
  v_expiry timestamptz;
  v_inserted integer;
BEGIN
  IF p_user_id IS NULL
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$'
     OR p_provider IS NULL OR p_provider !~ '^[A-Za-z0-9._-]{1,64}$'
     OR p_model IS NULL OR p_model !~ '^[A-Za-z0-9._-]{1,128}$'
     OR p_operation_type IS NULL OR p_operation_type !~ '^[A-Za-z0-9._-]{1,64}$'
     OR p_input_chars IS NULL OR p_input_chars NOT BETWEEN 0 AND 1000000
     OR p_tokens_used IS NULL OR p_tokens_used NOT BETWEEN 0 AND 100000000
     OR p_cost_estimate IS NULL OR p_cost_estimate < 0
     OR p_cost_estimate > 1000000
     OR p_prompt_version IS NOT NULL AND p_prompt_version !~ '^[A-Za-z0-9._-]{1,64}$'
     OR p_latency_ms IS NOT NULL AND p_latency_ms NOT BETWEEN 0 AND 3600000
     OR p_response IS NULL OR jsonb_typeof(p_response) <> 'object' THEN
    RAISE EXCEPTION 'invalid quota finalization';
  END IF;

  INSERT INTO public.trial_usage(user_id)
  VALUES (p_user_id)
  ON CONFLICT (user_id) DO NOTHING;

  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  SELECT status, expires_at
    INTO v_status, v_expiry
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_status = 'completed' THEN
    RETURN true;
  END IF;
  IF v_status IS DISTINCT FROM 'reserved' OR v_expiry <= pg_catalog.now() THEN
    IF v_status = 'reserved' THEN
      UPDATE public.ai_quota_reservations
         SET status = 'expired', updated_at = pg_catalog.now()
       WHERE user_id = p_user_id AND request_id = p_request_id;
    END IF;
    RETURN false;
  END IF;

  INSERT INTO public.ai_usage(
    user_id, provider, model, tokens_used, cost_estimate,
    operation_type, request_id, input_chars, latency_ms,
    status, prompt_version, metadata
  ) VALUES (
    p_user_id,
    p_provider,
    p_model,
    p_tokens_used,
    p_cost_estimate,
    p_operation_type,
    p_request_id,
    p_input_chars,
    p_latency_ms,
    'completed',
    p_prompt_version,
    p_response
  )
  ON CONFLICT (user_id, request_id) WHERE request_id IS NOT NULL DO NOTHING;
  GET DIAGNOSTICS v_inserted = ROW_COUNT;

  UPDATE public.ai_quota_reservations
     SET status = 'completed', updated_at = pg_catalog.now()
   WHERE user_id = p_user_id AND request_id = p_request_id;

  IF v_inserted = 1 THEN
    UPDATE public.trial_usage
       SET requests_count = requests_count + 1,
           updated_at = pg_catalog.now()
     WHERE user_id = p_user_id;
  END IF;
  RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION public.release_ai_quota(
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
BEGIN
  IF p_user_id IS NULL
     OR p_request_id IS NULL
     OR p_request_id !~ '^[A-Za-z0-9_-]{8,128}$' THEN
    RAISE EXCEPTION 'invalid quota release';
  END IF;

  PERFORM pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_user_id::text, 0)
  );

  SELECT status
    INTO v_status
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id AND request_id = p_request_id
   FOR UPDATE;

  IF v_status = 'reserved' THEN
    UPDATE public.ai_quota_reservations
       SET status = 'refunded', updated_at = pg_catalog.now()
     WHERE user_id = p_user_id AND request_id = p_request_id;
    RETURN true;
  END IF;
  RETURN v_status IN ('refunded', 'expired');
END;
$$;

-- Keep the existing read-only check available for diagnostics, but reserve_ai_quota
-- is the only pre-provider gate used by the hosted function.
CREATE OR REPLACE FUNCTION public.check_ai_quota(p_user_id uuid)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_limit integer := 20;
  v_count integer;
  v_active integer;
  v_window integer;
BEGIN
  IF p_user_id IS NULL THEN
    RETURN false;
  END IF;
  SELECT requests_count INTO v_count FROM public.trial_usage WHERE user_id = p_user_id;
  IF v_count IS NULL THEN
    v_count := 0;
  END IF;
  SELECT count(*)::integer INTO v_active
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id AND status = 'reserved' AND expires_at > pg_catalog.now();
  SELECT count(*)::integer INTO v_window
    FROM public.ai_usage
   WHERE user_id = p_user_id AND status = 'completed'
     AND created_at > pg_catalog.now() - interval '1 minute';
  IF EXISTS (
    SELECT 1 FROM public.entitlements
     WHERE user_id = p_user_id
       AND (expires_at IS NULL OR expires_at > pg_catalog.now())
  ) THEN
    v_limit := 1000;
  END IF;
  RETURN v_count + v_active < v_limit AND v_window + v_active < 5;
END;
$$;

REVOKE ALL ON FUNCTION public.reserve_ai_quota(uuid, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.reserve_ai_quota(uuid, text) TO service_role;
REVOKE ALL ON FUNCTION public.finalize_ai_quota(uuid, text, text, text, text, integer, integer, numeric, text, integer, jsonb)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.finalize_ai_quota(uuid, text, text, text, text, integer, integer, numeric, text, integer, jsonb)
  TO service_role;
REVOKE ALL ON FUNCTION public.release_ai_quota(uuid, text) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.release_ai_quota(uuid, text) TO service_role;
REVOKE ALL ON FUNCTION public.check_ai_quota(uuid) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.check_ai_quota(uuid) TO service_role;
