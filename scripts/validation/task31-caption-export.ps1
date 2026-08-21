[CmdletBinding()]
param(
  [string]$FixtureDirectory = "qa/fixtures"
)

$ErrorActionPreference = "Stop"
$root = (Get-Location).Path
$fixtureRoot = Join-Path $root $FixtureDirectory
$workRoot = Join-Path ([IO.Path]::GetTempPath()) ("cutcut-task31-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $workRoot | Out-Null

function Get-VideoInfo([string]$path) {
  $json = & ffprobe -v error -show_streams -show_format -of json $path
  if ($LASTEXITCODE -ne 0) { throw "ffprobe failed for $path" }
  return ($json | ConvertFrom-Json)
}

function Get-AssFilterPath([string]$path) {
  $normalized = $path.Replace('\', '/')
  if ($normalized -match '^[A-Za-z]:') { $normalized = $normalized.Insert(1, '\') }
  return "ass='$($normalized.Replace("'", "\\'"))'"
}

try {
  $cases = @(
    @{ Name = "landscape"; Input = (Join-Path $fixtureRoot "sample.mp4") },
    @{ Name = "portrait"; Input = (Join-Path $fixtureRoot "portrait.mp4") }
  )
  foreach ($case in $cases) {
    if (-not (Test-Path -LiteralPath $case.Input)) { throw "Missing fixture: $($case.Input)" }
    $inputInfo = Get-VideoInfo $case.Input
    $video = $inputInfo.streams | Where-Object { $_.codec_type -eq "video" } | Select-Object -First 1
    if (-not $video) { throw "Fixture has no video stream: $($case.Input)" }

    $assPath = Join-Path $workRoot "$($case.Name).ass"
    $outputPath = Join-Path $workRoot "$($case.Name)-caption.mp4"
    $ass = @"
[Script Info]
ScriptType: v4.00+
PlayResX: $($video.width)
PlayResY: $($video.height)
WrapStyle: 1

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,42,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,50,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:00.70,Default,,0,0,0,,{\pos($([int]($video.width / 2)),$([int]($video.height * 0.8)))}Xin chào Việt Nam — [] {}
"@
    [IO.File]::WriteAllText($assPath, $ass.Trim(), [Text.UTF8Encoding]::new($true))

    $ffmpegArgs = @(
      "-y", "-i", $case.Input,
      "-vf", (Get-AssFilterPath $assPath),
      "-c:v", "libx264", "-preset", "ultrafast", "-an", $outputPath
    )
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & ffmpeg @ffmpegArgs 2>&1 | Out-Null
    $ffmpegExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorAction
    if ($ffmpegExitCode -ne 0 -or -not (Test-Path -LiteralPath $outputPath)) {
      throw "FFmpeg caption export failed for $($case.Name)"
    }
    $outputInfo = Get-VideoInfo $outputPath
    $outputVideo = $outputInfo.streams | Where-Object { $_.codec_type -eq "video" } | Select-Object -First 1
    if ($outputVideo.width -ne $video.width -or $outputVideo.height -ne $video.height) {
      throw "Output dimensions changed for $($case.Name)"
    }
    if ((Get-Item -LiteralPath $outputPath).Length -le 0) { throw "Empty caption output for $($case.Name)" }
    Write-Output "$($case.Name): PASS ($($video.width)x$($video.height), Unicode ASS filter)"
  }
}
finally {
  if (Test-Path -LiteralPath $workRoot) { Remove-Item -LiteralPath $workRoot -Recurse -Force }
}
