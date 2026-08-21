-- Task 32 hardening: make usage accounting server-owned, idempotent and
-- forward-only. No media table/storage bucket is introduced here.

ALTER TABLE public.credit_ledger
  ADD COLUMN IF NOT EXISTS idempotency_key text;

CREATE UNIQUE INDEX IF NOT EXISTS credit_ledger_user_idempotency_key
  ON public.credit_ledger(user_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

ALTER TABLE public.ai_usage
  ADD COLUMN IF NOT EXISTS request_id text,
  ADD COLUMN IF NOT EXISTS input_chars integer,
  ADD COLUMN IF NOT EXISTS latency_ms integer,
  ADD COLUMN IF NOT EXISTS metadata jsonb NOT NULL DEFAULT '{}'::jsonb;

CREATE UNIQUE INDEX IF NOT EXISTS ai_usage_user_request_id
  ON public.ai_usage(user_id, request_id)
  WHERE request_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS public.trial_usage (
  user_id uuid REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
  requests_count integer NOT NULL DEFAULT 0 CHECK (requests_count >= 0),
  window_requests integer NOT NULL DEFAULT 0 CHECK (window_requests >= 0),
  window_started_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE public.trial_usage
  ADD COLUMN IF NOT EXISTS window_requests integer NOT NULL DEFAULT 0;

ALTER TABLE public.trial_usage ENABLE ROW LEVEL SECURITY;

-- Tighten the baseline policies as well: UPDATE must constrain both the
-- existing row and the replacement row, otherwise a client could reassign an
-- owned row to another user id.
DROP POLICY IF EXISTS "Users can view own profile" ON public.profiles;
CREATE POLICY "Users can view own profile"
  ON public.profiles FOR SELECT TO authenticated
  USING ((select auth.uid()) = id);

DROP POLICY IF EXISTS "Users can update own profile" ON public.profiles;
CREATE POLICY "Users can update own profile"
  ON public.profiles FOR UPDATE TO authenticated
  USING ((select auth.uid()) = id)
  WITH CHECK ((select auth.uid()) = id);

DROP POLICY IF EXISTS "Users can view own devices" ON public.devices;
CREATE POLICY "Users can view own devices"
  ON public.devices FOR SELECT TO authenticated
  USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can insert own device" ON public.devices;
CREATE POLICY "Users can insert own device"
  ON public.devices FOR INSERT TO authenticated
  WITH CHECK ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can update own devices" ON public.devices;
CREATE POLICY "Users can update own devices"
  ON public.devices FOR UPDATE TO authenticated
  USING ((select auth.uid()) = user_id)
  WITH CHECK ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can view own trial usage" ON public.trial_usage;
CREATE POLICY "Users can view own trial usage"
  ON public.trial_usage FOR SELECT
  TO authenticated
  USING ((select auth.uid()) = user_id);

CREATE OR REPLACE FUNCTION public.reject_credit_ledger_mutation()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = ''
AS $$
BEGIN
  RAISE EXCEPTION 'credit_ledger is append-only';
END;
$$;

DROP TRIGGER IF EXISTS credit_ledger_append_only ON public.credit_ledger;
CREATE TRIGGER credit_ledger_append_only
  BEFORE UPDATE OR DELETE ON public.credit_ledger
  FOR EACH ROW EXECUTE FUNCTION public.reject_credit_ledger_mutation();

CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
BEGIN
  INSERT INTO public.profiles (id, display_name, avatar_url)
  VALUES (
    new.id,
    COALESCE(new.raw_user_meta_data->>'full_name', new.email),
    new.raw_user_meta_data->>'avatar_url'
  )
  ON CONFLICT (id) DO NOTHING;
  RETURN new;
END;
$$;

-- This function is only invoked by the auth trigger, never by a client RPC.
REVOKE ALL ON FUNCTION public.handle_new_user() FROM PUBLIC;

-- The Edge Function calls this RPC with the service role after JWT
-- verification. The row lock makes concurrent requests deterministic and the
-- request id makes retries free of double charging.
CREATE OR REPLACE FUNCTION public.check_ai_quota(p_user_id uuid)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_limit integer := 20;
  v_count integer;
  v_window_requests integer;
  v_window_started_at timestamptz;
BEGIN
  INSERT INTO public.trial_usage(user_id) VALUES (p_user_id)
    ON CONFLICT (user_id) DO NOTHING;
  SELECT requests_count, window_requests, window_started_at
    INTO v_count, v_window_requests, v_window_started_at
    FROM public.trial_usage WHERE user_id = p_user_id FOR UPDATE;

  IF v_window_started_at > pg_catalog.now() - interval '1 minute' AND v_window_requests >= 5 THEN
    RETURN false;
  END IF;
  IF EXISTS (
    SELECT 1 FROM public.entitlements
    WHERE user_id = p_user_id
      AND (expires_at IS NULL OR expires_at > pg_catalog.now())
  ) THEN
    v_limit := 1000;
  END IF;
  RETURN v_count < v_limit;
END;
$$;

REVOKE ALL ON FUNCTION public.check_ai_quota(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.check_ai_quota(uuid) TO service_role;

CREATE OR REPLACE FUNCTION public.consume_ai_quota(
  p_user_id uuid,
  p_request_id text,
  p_provider text,
  p_model text,
  p_operation_type text,
  p_input_chars integer,
  p_tokens_used integer DEFAULT 0,
  p_cost_estimate numeric DEFAULT 0
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_limit integer := 20;
  v_count integer;
  v_window_requests integer;
  v_window_started_at timestamptz;
BEGIN
  IF p_request_id IS NULL OR length(trim(p_request_id)) < 8 OR length(p_request_id) > 128 THEN
    RAISE EXCEPTION 'invalid request id';
  END IF;

  IF EXISTS (
    SELECT 1 FROM public.ai_usage
    WHERE user_id = p_user_id AND request_id = p_request_id
  ) THEN
    RETURN true;
  END IF;

  INSERT INTO public.trial_usage(user_id) VALUES (p_user_id)
    ON CONFLICT (user_id) DO NOTHING;
  SELECT requests_count, window_requests, window_started_at
    INTO v_count, v_window_requests, v_window_started_at
    FROM public.trial_usage WHERE user_id = p_user_id FOR UPDATE;

  IF v_window_started_at > now() - interval '1 minute' AND v_window_requests >= 5 THEN
    RETURN false;
  END IF;

  IF EXISTS (
    SELECT 1 FROM public.entitlements
    WHERE user_id = p_user_id
      AND (expires_at IS NULL OR expires_at > now())
  ) THEN
    v_limit := 1000;
  END IF;

  IF v_count >= v_limit THEN
    RETURN false;
  END IF;

  UPDATE public.trial_usage
    SET requests_count = requests_count + 1,
        window_requests = CASE WHEN v_window_started_at <= now() - interval '1 minute' THEN 1 ELSE window_requests + 1 END,
        window_started_at = CASE WHEN v_window_started_at <= now() - interval '1 minute' THEN now() ELSE window_started_at END,
        updated_at = now()
    WHERE user_id = p_user_id;

  INSERT INTO public.ai_usage(
    user_id, provider, model, tokens_used, cost_estimate,
    operation_type, request_id, input_chars
  ) VALUES (
    p_user_id, p_provider, p_model, greatest(p_tokens_used, 0),
    greatest(p_cost_estimate, 0), p_operation_type, p_request_id,
    greatest(p_input_chars, 0)
  );
  RETURN true;
END;
$$;

REVOKE ALL ON FUNCTION public.consume_ai_quota(uuid, text, text, text, text, integer, integer, numeric) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.consume_ai_quota(uuid, text, text, text, text, integer, integer, numeric) TO service_role;
