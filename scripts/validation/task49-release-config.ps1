[CmdletBinding()]
param([string]$Root = (Get-Location))

$ErrorActionPreference = "Stop"
$config = Get-Content -LiteralPath (Join-Path $Root "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
$package = Get-Content -LiteralPath (Join-Path $Root "package.json") -Raw | ConvertFrom-Json
$updater = $config.plugins.updater
$bundleTargets = @($config.bundle.targets)
if ($config.version -ne $package.version) {
  throw "Tauri/package versions diverge: $($config.version) vs $($package.version)"
}
if ($bundleTargets -notcontains "nsis" -or $bundleTargets -notcontains "msi") {
  throw "Windows beta bundle must explicitly include NSIS and MSI targets"
}
if (-not $updater.endpoints -or $updater.endpoints.Count -eq 0) {
  throw "Updater must define at least one HTTPS endpoint"
}
foreach ($endpoint in $updater.endpoints) {
  if ($endpoint -notmatch '^https://') { throw "Updater endpoint is not HTTPS: $endpoint" }
  if ($endpoint -notmatch '\{\{target\}\}' -or $endpoint -notmatch '\{\{arch\}\}') {
    throw "Updater endpoint must be target/architecture-specific: $endpoint"
  }
}
if ($updater.pubkey -eq "UNCONFIGURED_MINISIGN_PUBLIC_KEY") {
  Write-Warning "Updater remains fail-closed: replace the placeholder public key before beta publishing."
  exit 2
}
if ($updater.pubkey -notmatch '^untrusted comment:.*') {
  Write-Warning "Updater public key should be a Tauri Minisign public key before publishing."
  exit 2
}
Write-Output "Task 49 release configuration is publishable"
