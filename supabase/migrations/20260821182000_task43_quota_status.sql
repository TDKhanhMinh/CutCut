-- Task 43: server-truth account quota status for desktop diagnostics/UI.

CREATE OR REPLACE FUNCTION public.get_ai_quota_status(p_user_id uuid)
RETURNS TABLE(
  requests_used integer,
  request_limit integer,
  requests_remaining integer,
  window_used integer,
  window_limit integer,
  window_remaining integer,
  entitlement_active boolean,
  entitlement_expires_at timestamptz
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_count integer := 0;
  v_active integer := 0;
  v_window integer := 0;
  v_limit integer := 20;
  v_entitlement_active boolean := false;
  v_expiry timestamptz;
BEGIN
  IF p_user_id IS NULL THEN
    RAISE EXCEPTION 'invalid quota status user';
  END IF;

  SELECT COALESCE(requests_count, 0)
    INTO v_count
    FROM public.trial_usage
   WHERE user_id = p_user_id;

  SELECT count(*)::integer
    INTO v_active
    FROM public.ai_quota_reservations
   WHERE user_id = p_user_id
     AND status = 'reserved'
     AND expires_at > pg_catalog.now();

  SELECT count(*)::integer
    INTO v_window
    FROM public.ai_usage
   WHERE user_id = p_user_id
     AND status = 'completed'
     AND created_at > pg_catalog.now() - interval '1 minute';

  SELECT EXISTS (
    SELECT 1
      FROM public.entitlements
     WHERE user_id = p_user_id
       AND (expires_at IS NULL OR expires_at > pg_catalog.now())
  )
    INTO v_entitlement_active;
  IF v_entitlement_active THEN
    SELECT max(expires_at)
      INTO v_expiry
      FROM public.entitlements
     WHERE user_id = p_user_id
       AND (expires_at IS NULL OR expires_at > pg_catalog.now());
  END IF;
  IF v_entitlement_active THEN
    v_limit := 1000;
  END IF;

  RETURN QUERY
  SELECT least(v_count + v_active, v_limit),
         v_limit,
         greatest(v_limit - v_count - v_active, 0),
         v_window + v_active,
         5,
         greatest(5 - v_window - v_active, 0),
         v_entitlement_active,
         v_expiry;
END;
$$;

REVOKE ALL ON FUNCTION public.get_ai_quota_status(uuid) FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.get_ai_quota_status(uuid) TO service_role;
