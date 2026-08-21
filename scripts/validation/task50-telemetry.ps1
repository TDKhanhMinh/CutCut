[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$telemetry = Get-Content -LiteralPath (Join-Path $Root "src/services/telemetry.ts") -Raw
$crash = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/services/crash_reporter.rs") -Raw
$settings = Get-Content -LiteralPath (Join-Path $Root "src/components/settings/TelemetrySettings.tsx") -Raw
$violations = @()

if ($telemetry -notmatch 'localStorage\.getItem\(enabledKey\) === "true"') {
  $violations += "Telemetry must be opt-in when no preference exists."
}
if ($telemetry -notmatch 'allowedProperties = new Set') {
  $violations += "Telemetry properties must use an explicit allowlist."
}
if ($telemetry -match 'Object\.entries\(properties\).*fetch|fetch\([^\)]*properties') {
  $violations += "Raw arbitrary properties are sent directly to the telemetry endpoint."
}
if ($telemetry -notmatch 'new URL\(endpoint\).*https:') {
  $violations += "Telemetry endpoint must be HTTPS-only."
}
if ($crash -match 'writeln!\(file,.*payload\)|writeln!\(file,.*transcript|writeln!\(file,.*path') {
  $violations += "Crash hook writes unsanitized payload/content/path data."
}
if ($crash -notmatch 'is_ascii_alphanumeric\(\)') {
  $violations += "Crash hook must sanitize panic payload characters."
}
if ($settings -notmatch 'settings\.telemetryOptIn') {
  $violations += "Telemetry settings must expose an explicit opt-in label."
}

if ($violations.Count -gt 0) {
  $violations | ForEach-Object { Write-Error $_ }
  exit 1
}

Write-Output "Task 50 telemetry/crash static validation PASS (opt-in, allowlist, HTTPS-only, local crash sanitization)"
