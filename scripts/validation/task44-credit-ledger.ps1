param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$migration = Get-Content -LiteralPath (Join-Path $Root "supabase/migrations/20260821190000_task44_credit_ledger_lifecycle.sql") -Raw

foreach ($check in @(
  @{ Name = "reservation state table"; Pattern = "credit_reservations|reserved_amount|expires_at" },
  @{ Name = "append-only reconciliation"; Pattern = "reconcile_credit_reservations|expiry-release" },
  @{ Name = "atomic reserve"; Pattern = "reserve_credits|pg_advisory_xact_lock|SUM\(amount\)" },
  @{ Name = "commit release"; Pattern = "commit_credits|credit_release_unused|usage_commit" },
  @{ Name = "failure refund"; Pattern = "refund_credits|failure-refund" },
  @{ Name = "balance and reservation status"; Pattern = "get_credit_balance|get_credit_reservation" },
  @{ Name = "server-only grants"; Pattern = "REVOKE ALL ON FUNCTION|TO service_role" },
  @{ Name = "ledger metadata"; Pattern = "metadata jsonb|credit_ledger_metadata_object" }
)) {
  if ($migration -notmatch $check.Pattern) { throw "Task 44 contract missing: $($check.Name)" }
}

if ($migration -match "UPDATE\s+public\.credit_ledger|DELETE\s+FROM\s+public\.credit_ledger") {
  throw "Task 44 contract violated: credit ledger is not append-only"
}
if ($migration -match "GRANT\s+.*\s+(anon|authenticated)") {
  throw "Task 44 contract violated: API role can mutate credit ledger"
}

Write-Output "Task 44 credit ledger contract: PASS"
