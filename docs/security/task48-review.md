# Task 48 security/privacy review evidence

## Static checks completed

- Hosted Gemini calls are rejected if a URL query contains `generateContent?key=`
  and the edge function must not log API keys or transcript content.
- Native credential access is checked against the allowlisted Supabase session and
  Gemini BYOK keys; project JSON and tracked source files are scanned for common
  credential patterns.
- Tauri sidecars are explicitly named `ffmpeg`, `ffprobe`, and `whisper`.
  Arguments are passed as arrays through Rust validation boundaries, never as a
  renderer-provided shell command.
- Panic diagnostics are checked for sanitized local output only.
- `pwsh -NoProfile -File scripts/validation/task48-security.ps1` passes with one
  warning: the Tauri capability permits dynamic sidecar argument arrays. Every
  caller must remain behind the existing Rust path/argument validators and the
  capability must be re-audited when adding a native command.

## Remaining release gates

The task remains `Blocked` until evidence exists for authenticated Supabase RLS
cross-user tests, an actual local-only network inspection, Windows Credential
Manager behavior on a clean install, Edge Function deployment/secrets in the
target environment, and the release-signing/telemetry gates from Tasks 49–50.
These are environment/release gates, not claims that can be satisfied by static
source inspection.
