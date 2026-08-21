param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$migration = Get-Content -LiteralPath (Join-Path $Root "supabase/migrations/20260821082555_task42_device_activation.sql") -Raw
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/device-activation/index.ts") -Raw
$device = Get-Content -LiteralPath (Join-Path $Root "src/services/device.ts") -Raw
$store = Get-Content -LiteralPath (Join-Path $Root "src/stores/useEntitlementStore.ts") -Raw
$cache = Get-Content -LiteralPath (Join-Path $Root "src/lib/entitlement-cache.ts") -Raw
$native = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/device.rs") -Raw

$checks = @(
  @{ Name = "device metadata columns"; Text = $migration; Pattern = 'device_label|app_version|platform|created_at' },
  @{ Name = "server-owned device writes"; Text = $migration; Pattern = 'REVOKE INSERT, UPDATE, DELETE ON public\.devices|activate_device' },
  @{ Name = "atomic device limit"; Text = $migration; Pattern = 'pg_advisory_xact_lock|device_limit_exceeded' },
  @{ Name = "authenticated edge function"; Text = $edge; Pattern = 'auth\.getUser\(\)|SUPABASE_SERVICE_ROLE_KEY|activate_device|deactivate_device|list_user_devices' },
  @{ Name = "strict device payload"; Text = $edge; Pattern = 'DEVICE_HASH|SAFE_LABEL|SAFE_VERSION|MAX_BODY_BYTES' },
  @{ Name = "one-way installation hash"; Text = $device; Pattern = 'get_or_create_installation_id|SHA-256|deviceHash' },
  @{ Name = "entitlement cache"; Text = $store; Pattern = 'readEntitlementCache|writeEntitlementCache|refreshIfStale' },
  @{ Name = "server-backed entitlement"; Text = $store; Pattern = '\.from\("entitlements"\)|deviceService\.activate' },
  @{ Name = "deactivate confirmation"; Text = (Get-Content -LiteralPath (Join-Path $Root "src/components/settings/DeviceManager.tsx") -Raw); Pattern = 'window\.confirm|deviceService\.deactivate' }
)

foreach ($check in $checks) {
  if ($check.Text -notmatch $check.Pattern) { throw "Task 42 contract missing: $($check.Name)" }
}

if ($native -match 'MachineGuid|Win32_ComputerSystem|GetAdaptersInfo|MAC') {
  throw "Task 42 contract violated: hardware fingerprinting found"
}
if ($edge -match 'deviceHash.*console|console\.(log|warn|error).*deviceHash') {
  throw "Task 42 contract violated: device hash logging found"
}
if ($cache -match 'accessToken|refreshToken|password|service_role') {
  throw "Task 42 contract violated: credential material in entitlement cache"
}

Write-Output "Task 42 entitlement/device contract: PASS"
