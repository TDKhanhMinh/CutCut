param(
  [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot "../.."))
)

$ErrorActionPreference = "Stop"
$supabase = Get-Content -LiteralPath (Join-Path $Root "src/lib/supabase.ts") -Raw
$store = Get-Content -LiteralPath (Join-Path $Root "src/stores/useAuthStore.ts") -Raw
$service = Get-Content -LiteralPath (Join-Path $Root "src/services/auth.ts") -Raw
$ai = Get-Content -LiteralPath (Join-Path $Root "src/services/ai.ts") -Raw
$dialog = Get-Content -LiteralPath (Join-Path $Root "src/components/editor/AuthDialog.tsx") -Raw
$panel = Get-Content -LiteralPath (Join-Path $Root "src/components/editor/EditReviewPanel.tsx") -Raw
$native = Get-Content -LiteralPath (Join-Path $Root "src-tauri/src/commands/auth.rs") -Raw

$checks = @(
  @{ Name = "native secure storage adapter"; Text = $supabase; Pattern = 'get_secure_token|set_secure_token|delete_secure_token' },
  @{ Name = "session persistence and refresh"; Text = $supabase; Pattern = 'autoRefreshToken:\s*true|persistSession:\s*true|detectSessionInUrl:\s*false' },
  @{ Name = "session restore and change listener"; Text = $store; Pattern = 'getSession\(\)|onAuthStateChange|SIGNED_OUT' },
  @{ Name = "typed auth service boundary"; Text = $service; Pattern = 'authService|signInWithPassword|signOut' },
  @{ Name = "cloud request auth boundary"; Text = $ai; Pattern = 'authService\.invokeFunction' },
  @{ Name = "auth status boundary"; Text = $store; Pattern = 'session_expired|offline|signed_out' },
  @{ Name = "store-owned auth calls"; Text = $dialog; Pattern = 'useAuthStore|signIn\(|signUp\(' },
  @{ Name = "hosted AI auth gate"; Text = $panel; Pattern = '!session|setAuthDialogOpen\(true\)' },
  @{ Name = "keyring allowlist"; Text = $native; Pattern = 'SUPABASE_AUTH_KEY|Unsupported secure credential key' }
)

foreach ($check in $checks) {
  if ($check.Text -notmatch $check.Pattern) { throw "Task 41 contract missing: $($check.Name)" }
}

if ($supabase -match 'localStorage|sessionStorage|console\.(log|warn|error).*token') {
  throw "Task 41 contract violated: browser storage or token logging found"
}
if ($native -match 'format!\(') { throw "Task 41 contract violated: dynamic keyring key found" }
if ($dialog -match 'supabase\.auth') { throw "Task 41 contract violated: UI calls Supabase directly" }
if ($store -match 'supabase\.auth') { throw "Task 41 contract violated: store calls Supabase directly" }
if ($ai -match 'supabase\.functions\.invoke') { throw "Task 41 contract violated: AI service bypasses auth service" }

Write-Output "Task 41 auth contract: PASS"
