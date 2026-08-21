-- Task 42: privacy-aware device activation and server-owned device limits.
-- The desktop sends a one-way installation hash; raw hardware identifiers are
-- never collected or persisted.

ALTER TABLE public.devices
  ADD COLUMN IF NOT EXISTS device_label text NOT NULL DEFAULT 'CutCut Desktop',
  ADD COLUMN IF NOT EXISTS app_version text NOT NULL DEFAULT '0.1.0',
  ADD COLUMN IF NOT EXISTS platform text NOT NULL DEFAULT 'windows',
  ADD COLUMN IF NOT EXISTS created_at timestamptz NOT NULL DEFAULT now();

ALTER TABLE public.devices
  DROP CONSTRAINT IF EXISTS devices_device_hash_format,
  DROP CONSTRAINT IF EXISTS devices_device_label_format,
  DROP CONSTRAINT IF EXISTS devices_app_version_format,
  DROP CONSTRAINT IF EXISTS devices_platform_format;

ALTER TABLE public.devices
  ADD CONSTRAINT devices_device_hash_format
    CHECK (device_hash ~ '^[0-9a-f]{64}$'),
  ADD CONSTRAINT devices_device_label_format
    CHECK (length(device_label) BETWEEN 1 AND 64),
  ADD CONSTRAINT devices_app_version_format
    CHECK (app_version ~ '^[A-Za-z0-9._+-]{1,32}$'),
  ADD CONSTRAINT devices_platform_format
    CHECK (platform ~ '^[A-Za-z0-9._-]{1,16}$');

CREATE INDEX IF NOT EXISTS devices_user_active_last_seen
  ON public.devices(user_id, is_revoked, last_active_at DESC);

DROP POLICY IF EXISTS "Users can insert own device" ON public.devices;
DROP POLICY IF EXISTS "Users can update own devices" ON public.devices;
REVOKE INSERT, UPDATE, DELETE ON public.devices FROM PUBLIC, anon, authenticated;

CREATE OR REPLACE FUNCTION public.activate_device(
  p_user_id uuid,
  p_device_hash text,
  p_device_label text,
  p_app_version text,
  p_platform text,
  p_device_limit integer
)
RETURNS uuid
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
DECLARE
  v_device_id uuid;
  v_active_count integer;
BEGIN
  IF p_user_id IS NULL
     OR p_device_hash !~ '^[0-9a-f]{64}$'
     OR p_device_label IS NULL OR length(p_device_label) NOT BETWEEN 1 AND 64
     OR p_app_version IS NULL OR p_app_version !~ '^[A-Za-z0-9._+-]{1,32}$'
     OR p_platform IS NULL OR p_platform !~ '^[A-Za-z0-9._-]{1,16}$'
     OR p_device_limit IS NULL OR p_device_limit NOT BETWEEN 1 AND 64 THEN
    RAISE EXCEPTION 'invalid device activation payload';
  END IF;

  PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtext(p_user_id::text));

  SELECT id INTO v_device_id
  FROM public.devices
  WHERE user_id = p_user_id AND device_hash = p_device_hash;

  IF v_device_id IS NOT NULL THEN
    UPDATE public.devices
    SET device_label = p_device_label,
        app_version = p_app_version,
        platform = p_platform,
        last_active_at = pg_catalog.now(),
        is_revoked = false
    WHERE id = v_device_id;
    RETURN v_device_id;
  END IF;

  SELECT count(*)::integer INTO v_active_count
  FROM public.devices
  WHERE user_id = p_user_id AND is_revoked = false;

  IF v_active_count >= p_device_limit THEN
    RAISE EXCEPTION 'device_limit_exceeded';
  END IF;

  INSERT INTO public.devices(
    user_id, device_hash, device_label, app_version, platform,
    last_active_at, is_revoked, created_at
  ) VALUES (
    p_user_id, p_device_hash, p_device_label, p_app_version, p_platform,
    pg_catalog.now(), false, pg_catalog.now()
  ) RETURNING id INTO v_device_id;

  RETURN v_device_id;
END;
$$;

CREATE OR REPLACE FUNCTION public.deactivate_device(
  p_user_id uuid,
  p_device_id uuid
)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = ''
AS $$
BEGIN
  UPDATE public.devices
  SET is_revoked = true,
      last_active_at = pg_catalog.now()
  WHERE id = p_device_id AND user_id = p_user_id;
  RETURN FOUND;
END;
$$;

CREATE OR REPLACE FUNCTION public.list_user_devices(p_user_id uuid)
RETURNS TABLE(
  id uuid,
  device_label text,
  app_version text,
  platform text,
  last_active_at timestamptz,
  is_revoked boolean,
  created_at timestamptz
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = ''
AS $$
  SELECT d.id, d.device_label, d.app_version, d.platform,
         d.last_active_at, d.is_revoked, d.created_at
  FROM public.devices AS d
  WHERE d.user_id = p_user_id
  ORDER BY d.is_revoked ASC, d.last_active_at DESC;
$$;

REVOKE ALL ON FUNCTION public.activate_device(uuid, text, text, text, text, integer)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.activate_device(uuid, text, text, text, text, integer)
  TO service_role;

REVOKE ALL ON FUNCTION public.deactivate_device(uuid, uuid)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.deactivate_device(uuid, uuid)
  TO service_role;

REVOKE ALL ON FUNCTION public.list_user_devices(uuid)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.list_user_devices(uuid)
  TO service_role;
