[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

function Assert-True([bool] $Condition, [string] $Message) {
  if (-not $Condition) { throw $Message }
}

function Read-Json([string] $Path) {
  return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Invoke-Probe([string] $Path, [switch] $ExpectFailure) {
  $output = & ffprobe -v error -show_streams -show_format -of json -- $Path 2>&1
  $exitCode = $LASTEXITCODE
  if ($ExpectFailure) {
    Assert-True ($exitCode -ne 0) "Expected ffprobe to reject $Path"
    return $null
  }
  Assert-True ($exitCode -eq 0) "ffprobe failed for ${Path}: $output"
  return ($output -join "`n" | ConvertFrom-Json)
}

$root = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$fixtureRoot = Join-Path $root "qa/fixtures"
$matrixPath = Join-Path $fixtureRoot "fixture-matrix.json"
$matrix = Read-Json $matrixPath
$fixtureNames = @($matrix.cases | ForEach-Object { $_.media }) + @(
  $matrix.cases | Where-Object { $_.project } | ForEach-Object { $_.project }
)

foreach ($name in ($fixtureNames | Sort-Object -Unique)) {
  Assert-True (Test-Path -LiteralPath (Join-Path $fixtureRoot $name)) "Missing fixture: $name"
}

$samplePath = Join-Path $fixtureRoot "sample.mp4"
$portraitPath = Join-Path $fixtureRoot "portrait.mp4"
$noAudioPath = Join-Path $fixtureRoot "no-audio.mp4"
$corruptPath = Join-Path $fixtureRoot "corrupted-container.mp4"
$projectPath = Join-Path $fixtureRoot "Untitled.cutcut"
$unicodeProjectPath = Join-Path $fixtureRoot "Unicode-Việt.cutcut"
$missingProjectPath = Join-Path $fixtureRoot "missing-media.cutcut"

$sampleHashBefore = (Get-FileHash -LiteralPath $samplePath -Algorithm SHA256).Hash
$sampleMtimeBefore = (Get-Item -LiteralPath $samplePath).LastWriteTimeUtc
$sampleProbe = Invoke-Probe $samplePath
$portraitProbe = Invoke-Probe $portraitPath
$noAudioProbe = Invoke-Probe $noAudioPath
Invoke-Probe $corruptPath -ExpectFailure | Out-Null

$sampleVideo = @($sampleProbe.streams | Where-Object { $_.codec_type -eq "video" })[0]
$portraitVideo = @($portraitProbe.streams | Where-Object { $_.codec_type -eq "video" })[0]
Assert-True ($sampleVideo.width -eq 640 -and $sampleVideo.height -eq 360) "sample.mp4 is not 16:9 640x360"
Assert-True ($portraitVideo.width -eq 720 -and $portraitVideo.height -eq 1280) "portrait.mp4 is not 9:16 720x1280"
Assert-True (-not @($noAudioProbe.streams | Where-Object { $_.codec_type -eq "audio" })) "no-audio.mp4 unexpectedly has an audio stream"

$project = Read-Json $projectPath
$unicodeProject = Read-Json $unicodeProjectPath
$missingProject = Read-Json $missingProjectPath
Assert-True ($project.media[0].path -eq "sample.mp4") "Portable project media reference changed"
Assert-True ($project.transcript.segments[0].text -eq "Xin chào") "Vietnamese transcript fixture changed"
Assert-True ($unicodeProject.transcript.segments[0].text -match "Xin chào") "Unicode project fixture is missing Vietnamese text"
Assert-True ($missingProject.media[0].path -eq "does-not-exist.mp4") "Missing-media fixture no longer exercises relink"

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("cutcut-task46-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempRoot | Out-Null
try {
  $roundTripPath = Join-Path $tempRoot "round-trip.cutcut"
  $project | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $roundTripPath -Encoding utf8
  $roundTripProject = Read-Json $roundTripPath
  Assert-True ($roundTripProject.media[0].path -eq "sample.mp4") "Project save/reopen round-trip changed media reference"

  $relinkedPath = Join-Path $tempRoot "relinked.cutcut"
  $missingProject.media[0].path = "sample.mp4"
  $missingProject | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $relinkedPath -Encoding utf8
  $relinkedProject = Read-Json $relinkedPath
  Assert-True ($relinkedProject.media[0].path -eq "sample.mp4") "Relink recovery did not persist the replacement media path"

  $assPath = Join-Path $tempRoot "caption.ass"
  $outputPath = Join-Path $tempRoot "task46-output.mp4"
  $portraitOutputPath = Join-Path $tempRoot "task46-portrait-output.mp4"
  $ass = @"
[Script Info]
ScriptType: v4.00+
PlayResX: 640
PlayResY: 360

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,28,&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,2,20,20,24,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:00.90,Default,,0,0,24,,Xin chào — caption fixture
"@
  [System.IO.File]::WriteAllText($assPath, $ass.TrimStart(), [System.Text.UTF8Encoding]::new($false))

  $filterPath = $assPath.Replace("\", "/").Replace(":", "\:")
  & ffmpeg -hide_banner -loglevel error -y -i $samplePath -vf "subtitles=filename='$filterPath'" -c:v libx264 -pix_fmt yuv420p -c:a aac -movflags +faststart -- $outputPath
  Assert-True ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $outputPath)) "Caption export fixture failed"

  $outputProbe = Invoke-Probe $outputPath
  $outputVideo = @($outputProbe.streams | Where-Object { $_.codec_type -eq "video" })[0]
  Assert-True ($outputVideo.width -eq 640 -and $outputVideo.height -eq 360) "Export output dimensions changed"
  Assert-True ([double]$outputProbe.format.duration -gt 0) "Export output has no duration"
  Assert-True (@($outputProbe.streams | Where-Object { $_.codec_type -eq "audio" }).Count -eq 1) "Export output lost audio"

  & ffmpeg -hide_banner -loglevel error -y -i $portraitPath -vf "scale=720:1280" -an -c:v libx264 -pix_fmt yuv420p -- $portraitOutputPath
  Assert-True ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $portraitOutputPath)) "Portrait export fixture failed"
  $portraitOutputProbe = Invoke-Probe $portraitOutputPath
  $portraitOutputVideo = @($portraitOutputProbe.streams | Where-Object { $_.codec_type -eq "video" })[0]
  Assert-True ($portraitOutputVideo.width -eq 720 -and $portraitOutputVideo.height -eq 1280) "Portrait export dimensions changed"
} finally {
  if (Test-Path -LiteralPath $tempRoot) {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
}

$sampleHashAfter = (Get-FileHash -LiteralPath $samplePath -Algorithm SHA256).Hash
Assert-True ($sampleHashBefore -eq $sampleHashAfter) "Source media was modified during validation"
Assert-True ($sampleMtimeBefore -eq (Get-Item -LiteralPath $samplePath).LastWriteTimeUtc) "Source media mtime changed during validation"
Write-Output "Task46 fixture/output validation PASS (5 matrix cases; source hash preserved)"
