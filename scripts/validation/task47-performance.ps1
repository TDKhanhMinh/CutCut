[CmdletBinding()]
param(
  [string]$FixtureDirectory = "qa/fixtures/performance",
  [string]$ReportPath = "docs/qa/task47-performance-report.json",
  [int[]]$DurationsMinutes = @(5, 20, 60)
)

$ErrorActionPreference = "Stop"
$fixtureRoot = Join-Path (Get-Location) $FixtureDirectory
$reportFile = Join-Path (Get-Location) $ReportPath
New-Item -ItemType Directory -Force -Path $fixtureRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $reportFile) | Out-Null

$results = foreach ($minutes in $DurationsMinutes) {
  $fixture = Join-Path $fixtureRoot "talking-head-$minutes-min.mp4"
  if (-not (Test-Path -LiteralPath $fixture)) {
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & ffmpeg -y -f lavfi -i "testsrc2=size=1280x720:rate=30" -f lavfi -i "anoisesrc=color=pink:sample_rate=48000" -t ($minutes * 60) -c:v libx264 -preset veryfast -pix_fmt yuv420p -c:a aac -shortest $fixture 2>&1 | Out-Null
    $ErrorActionPreference = $previousErrorAction
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg fixture generation failed for $minutes minutes" }
  }

  $measurements = @{}
  $stages = @(
    @{ Name = "ffprobe"; Executable = "ffprobe"; Arguments = @("-v", "error", "-show_format", "-show_streams", "-of", "json", $fixture) },
    @{ Name = "audio-extract"; Executable = "ffmpeg"; Arguments = @("-y", "-i", $fixture, "-map", "0:a:0", "-ac", "1", "-ar", "16000", "-f", "wav", [IO.Path]::ChangeExtension($fixture, ".wav")) }
  )
  foreach ($stage in $stages) {
    $process = [Diagnostics.Process]::new()
    $process.StartInfo.FileName = $stage.Executable
    $process.StartInfo.Arguments = ($stage.Arguments | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join ' '
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardOutput = $false
    $process.StartInfo.RedirectStandardError = $false
    $started = Get-Date
    [void]$process.Start()
    $peakWorkingSet = 0L
    while (-not $process.HasExited) {
      try {
        $peakWorkingSet = [math]::Max($peakWorkingSet, $process.WorkingSet64)
      } catch {
        # The process can exit between HasExited and WorkingSet64; the final
        # sample below is best-effort and the exit code remains authoritative.
      }
      Start-Sleep -Milliseconds 250
    }
    try {
      $peakWorkingSet = [math]::Max($peakWorkingSet, $process.WorkingSet64)
    } catch {
      # Process metrics are best-effort on Windows after process exit.
    }
    $measurements[$stage.Name] = [ordered]@{
      exitCode = $process.ExitCode
      elapsedMs = [int]((Get-Date) - $started).TotalMilliseconds
      peakWorkingSetMb = [math]::Round($peakWorkingSet / 1MB, 2)
      totalProcessorTimeMs = [int]$process.TotalProcessorTime.TotalMilliseconds
    }
  }

  [ordered]@{
    durationMinutes = $minutes
    fixture = $fixture
    measurements = $measurements
  }
}

$report = [ordered]@{
  generatedAt = (Get-Date).ToUniversalTime().ToString("o")
  host = [Environment]::MachineName
  os = [Environment]::OSVersion.VersionString
  durationsMinutes = $DurationsMinutes
  results = $results
  notes = @(
    "Run the full editor flow separately for cold startup, preview, Whisper, export, cancel and cleanup.",
    "Repeat each duration 2-3 times on the supported Windows beta hardware before changing limits."
  )
}
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportFile -Encoding utf8
Write-Output "Wrote $reportFile"
