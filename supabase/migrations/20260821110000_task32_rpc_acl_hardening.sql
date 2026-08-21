-- Supabase exposes function EXECUTE grants to API roles explicitly in some
-- projects. Revoke those direct grants as well as PUBLIC defaults so the
-- quota RPCs remain service-role-only and the auth trigger is not an RPC API.

REVOKE ALL ON FUNCTION public.handle_new_user()
  FROM PUBLIC, anon, authenticated, service_role;

REVOKE ALL ON FUNCTION public.check_ai_quota(uuid)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.check_ai_quota(uuid) TO service_role;

REVOKE ALL ON FUNCTION public.consume_ai_quota(
  uuid, text, text, text, text, integer, integer, numeric
)
  FROM PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.consume_ai_quota(
  uuid, text, text, text, text, integer, integer, numeric
) TO service_role;
