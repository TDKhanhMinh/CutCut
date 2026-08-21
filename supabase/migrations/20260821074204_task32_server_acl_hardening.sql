-- Task 32 security boundary: account/credit mutations are server-owned.
-- Explicitly revoke direct API-role grants because some Supabase projects
-- grant anon/authenticated privileges independently of PUBLIC defaults.

REVOKE INSERT, UPDATE, DELETE ON TABLE
  public.entitlements,
  public.trial_usage,
  public.ai_usage,
  public.credit_ledger,
  public.app_config
FROM PUBLIC, anon, authenticated;

REVOKE ALL ON FUNCTION public.reserve_credits(uuid, integer, text)
  FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.commit_credits(uuid, integer, text)
  FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.commit_credits(
  uuid, integer, text, text, text, numeric
)
  FROM PUBLIC, anon, authenticated;
REVOKE ALL ON FUNCTION public.refund_credits(uuid, text)
  FROM PUBLIC, anon, authenticated;

GRANT EXECUTE ON FUNCTION public.reserve_credits(uuid, integer, text)
  TO service_role;
GRANT EXECUTE ON FUNCTION public.commit_credits(uuid, integer, text)
  TO service_role;
GRANT EXECUTE ON FUNCTION public.commit_credits(
  uuid, integer, text, text, text, numeric
)
  TO service_role;
GRANT EXECUTE ON FUNCTION public.refund_credits(uuid, text)
  TO service_role;
