[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$violations = @()
$warnings = @()
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-analyze/index.ts") -Raw
if ($edge -match "generateContent\?key=") { $violations += "Hosted Gemini key is present in a URL query" }
if ($edge -match "console\.(log|info|warn|error).*apiKey|console\.(log|info|warn|error).*transcript") {
  $violations += "Hosted AI logging includes a credential or transcript"
}
$auth = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/auth.rs") -Raw
if ($auth -match 'format!\("cutcut_app_\{\}"') { $violations += "Secure credential key is not allowlisted" }
$capabilities = Get-Content -LiteralPath (Join-Path $Root "src-tauri/capabilities/default.json") -Raw | ConvertFrom-Json
$sidecarPermission = $capabilities.permissions | Where-Object { $_.identifier -eq "shell:allow-execute" }
if (-not $sidecarPermission) { $violations += "No explicit sidecar execution capability is configured" }
else {
  $sidecars = @($sidecarPermission.allow | ForEach-Object { $_.name })
  foreach ($required in @("ffmpeg", "ffprobe", "whisper")) {
    if ($sidecars -notcontains $required) { $violations += "Required sidecar is not allowlisted: $required" }
  }
  if (@($sidecarPermission.allow | Where-Object { $_.sidecar -eq $true -and $_.args -eq $true }).Count -gt 0) {
    $warnings += "Sidecar capability allows dynamic argument arrays; keep all callers behind Rust validators and re-audit on every new command."
  }
}
$crash = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/services/crash_reporter.rs") -Raw
if ($crash -match 'writeln!\(file,.*payload\)|writeln!\(file,.*transcript|writeln!\(file,.*path') {
  $violations += "Crash report writes unsanitized content"
}
$tracked = & git -C $Root ls-files
foreach ($path in $tracked) {
  if ($path -match '(^|/)(\.env|node_modules|target|playwright-report|qa/test-results)(/|$)') { continue }
  $content = Get-Content -LiteralPath (Join-Path $Root $path) -Raw -ErrorAction SilentlyContinue
  if ($content -match '(?i)(AIza[0-9A-Za-z_-]{20,}|SUPABASE_SERVICE_ROLE_KEY\s*[:=]\s*[A-Za-z0-9._-]{20,}|-----BEGIN (RSA|EC|OPENSSH) PRIVATE KEY-----)') {
    $violations += "Tracked file contains a credential-like secret: $path"
  }
}
if ($violations.Count -gt 0) {
  $violations | ForEach-Object { Write-Error $_ }
  exit 1
}
$warnings | ForEach-Object { Write-Warning $_ }
Write-Output "Task 48 static security checks passed; live RLS, clean-install and network inspection remain required"
