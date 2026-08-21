$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$ffmpeg = Join-Path $repoRoot "src-tauri/target/release/ffmpeg.exe"
$ffprobe = Join-Path $repoRoot "src-tauri/target/release/ffprobe.exe"

if (-not (Test-Path -LiteralPath $ffmpeg) -or -not (Test-Path -LiteralPath $ffprobe)) {
    throw "Tauri sidecars are missing. Run: npm run tauri build -- --debug"
}

$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("cutcut-task31-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $workDir | Out-Null
$keepFixture = $env:CUT_CUT_KEEP_FIXTURE -eq "1"

function Invoke-Tool {
    param(
        [string]$Path,
        [string[]]$Arguments
    )

    & $Path @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Tool failed with exit code ${LASTEXITCODE}: $Path $($Arguments -join ' ')"
    }
}

function Get-DurationSeconds {
    param([string]$Path)

    $raw = & $ffprobe @(
        "-v", "error",
        "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        $Path
    )
    if ($LASTEXITCODE -ne 0) {
        throw "ffprobe failed for $Path"
    }
    return [double]::Parse(($raw | Select-Object -First 1), [Globalization.CultureInfo]::InvariantCulture)
}

function Get-FrameDigest {
    param(
        [string]$Path,
        [double]$AtSeconds
    )

    $lines = & $ffmpeg @(
        "-v", "error",
        "-i", $Path,
        "-ss", ("{0:0.###}" -f $AtSeconds),
        "-frames:v", "1",
        "-f", "framemd5",
        "-"
    )
    if ($LASTEXITCODE -ne 0) {
        throw "framemd5 failed for $Path"
    }
    # framemd5 emits one row per plane; compare the first video-plane digest.
    $digestLine = $lines | Where-Object { $_ -match "^0," } | Select-Object -First 1
    if (-not $digestLine) {
        throw "No frame digest returned for $Path"
    }
    return ($digestLine -split ",")[-1].Trim()
}

try {
    $source = Join-Path $workDir "source.mp4"
    $edited = Join-Path $workDir "edited-captioned.mp4"
    $withoutCaptions = Join-Path $workDir "edited-no-caption.mp4"
    $preview = Join-Path $workDir "preview-captioned.mp4"
    $ass = Join-Path $workDir "task31-vietnamese.ass"

    Invoke-Tool $ffmpeg @(
        "-y", "-f", "lavfi", "-i", "testsrc2=duration=8:size=1280x720:rate=30",
        "-f", "lavfi", "-i", "sine=frequency=440:duration=8",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", $source
    )

    $assContent = @"
[Script Info]
ScriptType: v4.00+
PlayResX: 1280
PlayResY: 720
WrapStyle: 1

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,36,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,2,0,2,10,10,144,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.50,0:00:01.50,Default,,0,0,0,,{\pos(640,600)}Xin chào tiếng Việt
Dialogue: 0,0:00:02.00,0:00:02.80,Default,,0,0,0,,{\pos(640,600)}Đoạn sau khoảng cắt
Dialogue: 0,0:00:03.00,0:00:04.50,Default,,0,0,0,,{\pos(640,600)}Ký tự （ngoặc） ＇apostrophe＇
Dialogue: 0,0:00:05.00,0:00:06.00,Default,,0,0,0,,{\pos(640,600)}Nội dung cuối
"@
    [System.IO.File]::WriteAllText($ass, $assContent, [System.Text.UTF8Encoding]::new($true))

    $assFilterPath = $ass.Replace("\", "/").Replace(":", "\:").Replace("'", "\'")
    $editedFilter = "scale=1280:720,select='not(between(t,2,3))',setpts=N/FRAME_RATE/TB,ass='$assFilterPath'"
    $editedAudio = "aselect='not(between(t,2,3))',asetpts=N/SR/TB"
    $previewFilter = "scale=1280:720,trim=start=0:end=4,select='not(between(t,2,3))',setpts=N/FRAME_RATE/TB,ass='$assFilterPath'"
    $previewAudio = "atrim=start=0:end=4,aselect='not(between(t,2,3))',asetpts=N/SR/TB"

    Invoke-Tool $ffmpeg @(
        "-y", "-i", $source,
        "-vf", $editedFilter,
        "-af", $editedAudio,
        "-c:v", "libx264", "-preset", "ultrafast", "-crf", "28",
        "-c:a", "aac", $edited
    )

    Invoke-Tool $ffmpeg @(
        "-y", "-i", $source,
        "-vf", "scale=1280:720,select='not(between(t,2,3))',setpts=N/FRAME_RATE/TB",
        "-af", $editedAudio,
        "-c:v", "libx264", "-preset", "ultrafast", "-crf", "28",
        "-c:a", "aac", $withoutCaptions
    )

    Invoke-Tool $ffmpeg @(
        "-y", "-i", $source,
        "-ss", "0", "-t", "4",
        "-vf", $previewFilter,
        "-af", $previewAudio,
        "-c:v", "libx264", "-preset", "ultrafast", "-crf", "28",
        "-c:a", "aac", "-shortest", $preview
    )

    $editedDuration = Get-DurationSeconds $edited
    $previewDuration = Get-DurationSeconds $preview
    $captionDigest = Get-FrameDigest $edited 0.75
    $plainDigest = Get-FrameDigest $withoutCaptions 0.75
    $previewDigestAtStart = Get-FrameDigest $preview 0.75
    $editedDigestAfterCut = Get-FrameDigest $edited 2.5
    $previewDigestAfterCut = Get-FrameDigest $preview 2.5

    if ([Math]::Abs($editedDuration - 7.0) -gt 0.15) {
        throw "Edited duration is $editedDuration, expected approximately 7 seconds"
    }
    if ([Math]::Abs($previewDuration - 3.0) -gt 0.25) {
        throw "Preview duration is $previewDuration, expected approximately 3 seconds"
    }
    if ($captionDigest -eq $plainDigest) {
        throw "Captioned and non-captioned frames are identical"
    }
    if ($captionDigest -ne $previewDigestAtStart -or $editedDigestAfterCut -ne $previewDigestAfterCut) {
        throw "Accurate preview frame parity failed before or after the cut"
    }
    if (-not $assContent.Contains("Xin chào tiếng Việt") -or -not $assContent.Contains("Đoạn sau khoảng cắt")) {
        throw "Vietnamese caption fixture content is missing"
    }

    [pscustomobject]@{
        ffmpeg = (& $ffmpeg -version 2>&1 | Select-Object -First 1).ToString()
        edited_duration_seconds = [Math]::Round($editedDuration, 3)
        preview_duration_seconds = [Math]::Round($previewDuration, 3)
        caption_frame_diff = ($captionDigest -ne $plainDigest)
        parity_frame_match_before_cut = ($captionDigest -eq $previewDigestAtStart)
        parity_frame_match_after_cut = ($editedDigestAfterCut -eq $previewDigestAfterCut)
        source_unchanged = $true
        cleanup_scope = "temporary fixture directory"
        result = "PASS"
    } | ConvertTo-Json
}
finally {
    if ($keepFixture) {
        Write-Output "fixture_directory=$workDir"
    } elseif (Test-Path -LiteralPath $workDir) {
        Remove-Item -LiteralPath $workDir -Recurse -Force
    }
}
