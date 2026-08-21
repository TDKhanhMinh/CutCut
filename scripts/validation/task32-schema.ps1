[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$violations = [System.Collections.Generic.List[string]]::new()
$migrationRoot = Join-Path $Root "supabase/migrations"
$migrationFiles = @(Get-ChildItem -LiteralPath $migrationRoot -Filter "*.sql" -File | Sort-Object Name)
$migrationSql = ($migrationFiles | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw }) -join "`n"

if ($migrationFiles.Count -eq 0) {
  $violations.Add("No Supabase migrations were found")
}

foreach ($table in @("profiles", "entitlements", "devices", "credit_ledger", "ai_usage", "app_config", "trial_usage")) {
  $createPattern = "(?im)^\s*CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:public\.)?$table\b"
  $createCount = [regex]::Matches($migrationSql, $createPattern).Count
  if ($createCount -ne 1) {
    $violations.Add("Table $table must have exactly one CREATE TABLE migration (found $createCount)")
  }
}

foreach ($table in @("profiles", "entitlements", "devices", "credit_ledger", "ai_usage", "app_config", "trial_usage")) {
  if ($migrationSql -notmatch "(?is)ALTER\s+TABLE\s+(?:public\.)?$table\s+ENABLE\s+ROW\s+LEVEL\s+SECURITY") {
    $violations.Add("RLS is not enabled for $table")
  }
}

foreach ($required in @(
  "CREATE TABLE (?:IF NOT EXISTS )?(?:public\.)?trial_usage",
  "CREATE UNIQUE INDEX IF NOT EXISTS credit_ledger_user_idempotency_key",
  "CREATE UNIQUE INDEX IF NOT EXISTS ai_usage_user_request_id",
  "GRANT EXECUTE ON FUNCTION public\.check_ai_quota\(uuid\) TO service_role",
  "GRANT EXECUTE ON FUNCTION public\.consume_ai_quota\(",
  "REVOKE INSERT, UPDATE, DELETE ON TABLE",
  "REVOKE ALL ON FUNCTION public\.reserve_credits\(uuid, integer, text\)",
  "REVOKE ALL ON FUNCTION public\.commit_credits\(uuid, integer, text\)",
  "REVOKE ALL ON FUNCTION public\.refund_credits\(uuid, text\)",
  "REVOKE ALL ON FUNCTION public.handle_new_user()",
  "USING \(\(select auth\.uid\(\)\) = user_id\)"
)) {
  if ($migrationSql -notmatch $required) {
    $violations.Add("Missing required schema/RLS contract: $required")
  }
}

if ($migrationSql -match "(?i)(storage\.buckets|INSERT\s+INTO\s+storage|CREATE\s+BUCKET)") {
  $violations.Add("Task32 migrations must not create a source-media storage bucket")
}

$entitlementStore = Get-Content -LiteralPath (Join-Path $Root "src/stores/useEntitlementStore.ts") -Raw
if ($entitlementStore -notmatch "\.select\('plan_id, features, expires_at'\)") {
  $violations.Add("Entitlement client must query the canonical plan_id/features schema")
}
if ($entitlementStore -match "\.select\('plan, capabilities, expires_at'\)") {
  $violations.Add("Entitlement client still queries the obsolete plan/capabilities columns")
}

if ($violations.Count -gt 0) {
  $violations | ForEach-Object { Write-Error $_ }
  exit 1
}

Write-Output "Task 32 schema and entitlement contract checks passed ($($migrationFiles.Count) migrations)"
