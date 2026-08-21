-- Keep account-owned read policies on the authenticated role and evaluate
-- auth.uid() once per statement rather than once per row.

DROP POLICY IF EXISTS "Users can view own entitlements" ON public.entitlements;
CREATE POLICY "Users can view own entitlements"
  ON public.entitlements FOR SELECT
  TO authenticated
  USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can view own credit ledger" ON public.credit_ledger;
CREATE POLICY "Users can view own credit ledger"
  ON public.credit_ledger FOR SELECT
  TO authenticated
  USING ((select auth.uid()) = user_id);

DROP POLICY IF EXISTS "Users can view own ai usage" ON public.ai_usage;
CREATE POLICY "Users can view own ai usage"
  ON public.ai_usage FOR SELECT
  TO authenticated
  USING ((select auth.uid()) = user_id);
