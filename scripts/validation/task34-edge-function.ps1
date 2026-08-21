[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$violations = [System.Collections.Generic.List[string]]::new()
$edge = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-analyze/index.ts") -Raw
$prompt = Get-Content -LiteralPath (Join-Path $Root "supabase/functions/ai-analyze/prompt.ts") -Raw

foreach ($required in @(
  "auth.getUser()",
  "SUPABASE_SERVICE_ROLE_KEY",
  "GEMINI_API_KEY",
  "MAX_BODY_BYTES",
  "MAX_SEGMENTS",
  "MAX_TOTAL_TEXT",
  "AbortController",
  "x-goog-api-key",
  "validateActions",
  "consume_ai_quota",
  "OPERATION_TYPE"
)) {
  if ($edge -notmatch [regex]::Escape($required)) {
    $violations.Add("Missing Edge Function contract: $required")
  }
}

if ($edge -match "generateContent\?key=") {
  $violations.Add("Gemini API key must not be placed in a URL query")
}
if ($edge -match "(?i)(sourcePath|filePath|videoBinary|audioBinary|storage\.from)") {
  $violations.Add("Hosted AI endpoint must not accept or upload source media paths/binaries")
}
if ($edge -match "console\.(log|info|warn|error).*transcript") {
  $violations.Add("Transcript content must not be logged")
}
if ($prompt -notmatch "SEMANTIC_PROMPT_VERSION|semantic-v2") {
  $violations.Add("Prompt version is not source-controlled")
}

if ($violations.Count -gt 0) {
  $violations | ForEach-Object { Write-Error $_ }
  exit 1
}

Write-Output "Task 34 Edge Function static security checks passed"
