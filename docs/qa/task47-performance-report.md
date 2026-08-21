# Task 47 — Long-video performance baseline

The executable baseline is `scripts/validation/task47-performance.ps1`. It
generates local 5/20/60-minute fixtures and records FFprobe/audio extraction
elapsed time and peak working set. The full exit gate still requires running the
editor flow on the supported Windows beta hardware, repeating each duration 2–3
times, and recording import, preview, Whisper, edit-plan, export, cancel and
cleanup metrics.

## Dev baseline (2026-08-21)

- Host: `LAPTOP-TBTLP9L8`, Windows `10.0.26200.0`, FFmpeg `8.1.2`.
- Fixture: generated 1280×720 H.264 + mono AAC testsrc/pink-noise media.
- `ffprobe` stayed below 0.30 s and audio extraction stayed below 1.84 s for
  the 5/20/60 minute fixtures; extraction peak working set stayed below 5.1 MB
  in this process-level smoke measurement.
- Full editor/Whisper/export/cancel/repeat measurements are still open. This
  report is a dev baseline, not a production performance sign-off.

The raw measurements are in `task47-performance-report.json`. The generated
long-video files are intentionally kept out of Git because the 60-minute MP4
is about 1.2 GB; rerun the script to reproduce them on a target machine.
