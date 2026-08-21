param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$migration = Get-Content -LiteralPath (Join-Path $Root "supabase/migrations/20260821180000_task43_atomic_trial_reservations.sql") -Raw
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-analyze/index.ts") -Raw

$checks = @(
  @{ Name = "reservation table"; Text = $migration; Pattern = "ai_quota_reservations" },
  @{ Name = "atomic reservation"; Text = $migration; Pattern = "reserve_ai_quota|pg_advisory_xact_lock|in_flight" },
  @{ Name = "atomic finalization"; Text = $migration; Pattern = "finalize_ai_quota|ON CONFLICT \(user_id, request_id\)" },
  @{ Name = "failed operation release"; Text = $migration; Pattern = "release_ai_quota|status = 'refunded'" },
  @{ Name = "server-only RPC grants"; Text = $migration; Pattern = "TO service_role" },
  @{ Name = "pre-provider reservation"; Text = $edge; Pattern = "reserve_ai_quota|reserved = true" },
  @{ Name = "atomic usage finalize"; Text = $edge; Pattern = "finalize_ai_quota|p_response" },
  @{ Name = "provider failure release"; Text = $edge; Pattern = "releaseAndRespond|releaseQuota" },
  @{ Name = "request idempotency"; Text = $edge; Pattern = "requestId|request_in_progress|request_replay_unavailable" }
)

foreach ($check in $checks) {
  if ($check.Text -notmatch $check.Pattern) { throw "Task 43 contract missing: $($check.Name)" }
}

if ($edge -match "console\.(log|info|warn|error).*transcript") {
  throw "Task 43 contract violated: transcript content logged"
}
if ($migration -match "CREATE POLICY.*INSERT|GRANT .*anon|GRANT .*authenticated") {
  throw "Task 43 contract violated: client can mutate quota reservations"
}

Write-Output "Task 43 trial quota/usage contract: PASS"
