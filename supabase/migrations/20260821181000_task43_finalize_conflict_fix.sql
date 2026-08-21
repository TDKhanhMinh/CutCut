-- Task 43 forward fix: match the partial ai_usage idempotency index when
-- inserting finalized usage.

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

REVOKE ALL ON FUNCTION public.finalize_ai_quota(uuid, text, text, text, text, integer, integer, numeric, text, integer, jsonb)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.finalize_ai_quota(uuid, text, text, text, text, integer, integer, numeric, text, integer, jsonb)
  TO service_role;
