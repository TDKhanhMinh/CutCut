# CutCut V1 beta security/privacy checklist

## Source media and project data

- [x] Media processing remains local by default; the hosted AI path receives transcript data only.
- [x] Project JSON stores edit state and source references, not credentials.
- [ ] Authenticated end-to-end upload guard must be exercised in the Task 46 suite.

## Credentials and native boundary

- [x] Supabase auth and Gemini BYOK credentials use the native OS keyring.
- [x] Native credential commands accept only the allowlisted `supabase-auth-session` and `gemini-byok` keys.
- [x] Full BYOK keys are not returned to React after storage and are not logged.
- [ ] Windows credential-manager behavior must be verified on a clean beta install.
- [ ] Sidecar argument allowlists must be re-checked whenever a new native command is added.

## Hosted AI and backend

- [x] `ai-analyze` verifies JWT, validates bounded transcript input and canonicalizes provider output.
- [x] Server-side quota and credit RPCs use idempotency keys and append-only ledger entries.
- [ ] Supabase security advisor warning for leaked-password protection must be resolved before public beta.
- [ ] Deploy/verify the Edge Function and secrets in the target environment.

## Release and diagnostics

- [ ] Updater signatures, rollback and release artifacts are verified in Task 49.
- [ ] Telemetry/crash redaction and opt-out are verified in Task 50.
