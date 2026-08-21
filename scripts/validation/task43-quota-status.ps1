param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$migration = Get-Content -LiteralPath (Join-Path $Root "supabase/migrations/20260821182000_task43_quota_status.sql") -Raw
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-quota-status/index.ts") -Raw

foreach ($check in @(
  @{ Name = "quota status RPC"; Text = $migration; Pattern = "get_ai_quota_status|requests_remaining|entitlement_expires_at" },
  @{ Name = "server-only quota status"; Text = $migration; Pattern = "REVOKE ALL ON FUNCTION.*get_ai_quota_status|TO service_role" },
  @{ Name = "JWT status endpoint"; Text = $edge; Pattern = "auth\.getUser\(\)|get_ai_quota_status|SUPABASE_SERVICE_ROLE_KEY" },
  @{ Name = "status response contract"; Text = $edge; Pattern = "requestsRemaining|windowRemaining|entitlementExpiresAt" }
)) {
  if ($check.Text -notmatch $check.Pattern) { throw "Task 43 status contract missing: $($check.Name)" }
}

Write-Output "Task 43 quota status contract: PASS"
