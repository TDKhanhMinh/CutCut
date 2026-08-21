# Task 46 — Editor E2E acceptance plan

## Scope

This is the reproducible Windows/dev gate for the V1 editor contract:

`Import → probe → save/reopen → local transcript/edit state → caption → export`.

The suite is local-first. Fixtures are generated/sanitized test assets in
`qa/fixtures`; no user media or secrets are used.

## Fixture matrix

The version-controlled matrix is `qa/fixtures/fixture-matrix.json`.

| Case | Input | Expected | Covered assertions |
| --- | --- | --- | --- |
| `local-16-9-happy` | `sample.mp4` + `Untitled.cutcut` | pass | MP4 probe, Vietnamese transcript, project JSON reopen, caption burn, output metadata |
| `local-9-16-unicode` | `portrait.mp4` + `Unicode-Việt.cutcut` | pass-without-audio | 9:16 dimensions, Unicode project text, portrait output probe |
| `negative-no-audio` | `no-audio.mp4` | recoverable error | no audio stream is reported; input remains unchanged |
| `negative-corrupt-media` | `corrupted-container.mp4` | recoverable error | FFprobe fails without creating an output |
| `relink-missing-media` | `missing-media.cutcut` + `sample.mp4` | recoverable error then relink | missing reference is detected; portable project remains readable |

## Automated checks

- `npm run test:e2e -- --workers=1` runs the browser smoke suite and invokes the
  local fixture/output validator.
- `pwsh -NoProfile -File scripts/validation/task46-e2e.ps1` runs the same
  deterministic fixture, persistence, source-integrity, and FFmpeg/FFprobe
  assertions without a Tauri window.
- `npm run typecheck`, `npm run lint -- --quiet`, and `npm run build` are
  required release checks.

The validator creates all outputs in a temporary directory and removes them in
`finally`; it never overwrites a fixture or source media.

## Release-blocking gates

The following remain explicit runtime gates and are not silently represented as
passing by the local validator:

1. A signed Windows/Tauri runtime with real file dialogs and a downloaded
   Whisper model must repeat Import → STT → Review → Export.
2. An authorized hosted Gemini request and an authorized BYOK Gemini request
   must be run separately. The current session waives those live-provider
   requests, so no production connectivity claim is made.
3. Offline/cancel halfway behavior must be observed in the native window on a
   clean machine before Public Beta sign-off.

Any failure in those gates is a release blocker even when the deterministic
local contract suite passes.
