[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$violations = @()
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-analyze/index.ts") -Raw
if ($edge -match "generateContent\?key=") { $violations += "Hosted Gemini key is present in a URL query" }
if ($edge -match "console\.(log|info|warn|error).*apiKey|console\.(log|info|warn|error).*transcript") {
  $violations += "Hosted AI logging includes a credential or transcript"
}
$auth = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/auth.rs") -Raw
if ($auth -match 'format!\("cutcut_app_\{\}"') { $violations += "Secure credential key is not allowlisted" }
if ($violations.Count -gt 0) {
  $violations | ForEach-Object { Write-Error $_ }
  exit 1
}
Write-Output "Task 48 static security checks passed"
