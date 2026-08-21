param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$auth = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/auth.rs") -Raw
$ai = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/ai.rs") -Raw
$store = Get-Content -LiteralPath (Join-Path $Root "src/stores/useAIConfigStore.ts") -Raw
$ui = Get-Content -LiteralPath (Join-Path $Root "src/components/settings/BYOKManager.tsx") -Raw
$service = Get-Content -LiteralPath (Join-Path $Root "src/services/gemini.ts") -Raw

foreach ($check in @(
  @{ Name = "OS keyring adapter"; Text = $auth; Pattern = "keyring::Entry|GEMINI_BYOK_KEY|read_secure_value|write_secure_value|delete_secure_value" },
  @{ Name = "public session boundary"; Text = $auth; Pattern = "public_session_key\(\&key\)\?" },
  @{ Name = "native BYOK commands"; Text = $ai; Pattern = "get_gemini_key_status|set_gemini_api_key|delete_gemini_api_key|test_gemini_key|call_gemini_direct" },
  @{ Name = "masked status"; Text = $ai; Pattern = "masked_hint|configured" },
  @{ Name = "mode state"; Text = $store; Pattern = "AIMode|hosted|byok|setMode" },
  @{ Name = "settings UX"; Text = $ui; Pattern = 'type="password"|testKey|removeKey|saveKey' },
  @{ Name = "typed frontend service"; Text = $service; Pattern = "getGeminiKeyStatus|setGeminiApiKey|deleteGeminiApiKey|testGeminiKey" }
)) {
  if ($check.Text -notmatch $check.Pattern) { throw "Task 45 contract missing: $($check.Name)" }
}

foreach ($command in @("set_secure_token", "get_secure_token", "delete_secure_token")) {
  if ($auth -notmatch "(?s)pub fn $command\([^}]+public_session_key\(\&key\)\?;") {
    throw "Task 45 contract violated: $command can access non-session credentials"
  }
}

if ($store -match "localStorage\.(getItem|setItem|removeItem).*?(api.?key|gemini.?key|secret|token)") {
  throw "Task 45 contract violated: BYOK secret persisted in localStorage"
}
if (($auth + $ai + $store + $ui) -match "console\.(log|info|warn|error).*?(api.?key|secret|password)") {
  throw "Task 45 contract violated: BYOK secret logging found"
}
if (($store + $ui + $service) -match "GEMINI_BYOK_KEY|read_secure_value|write_secure_value") {
  throw "Task 45 contract violated: frontend references native secret material"
}

Write-Output "Task 45 BYOK secure-storage contract: PASS"
