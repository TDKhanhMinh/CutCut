# Task 50 telemetry and crash diagnostics

The renderer telemetry queue is explicitly opt-in. A missing
`cutcut-telemetry-enabled` preference does not enqueue startup or operation
events; disabling the setting clears the local queue. Only the allowlisted
technical properties in `src/services/telemetry.ts` are serialized, string
values are redacted/truncated, and a configured endpoint must use HTTPS.

The Rust panic hook writes a sanitized, local `crash.log` in the application
data directory. It does not upload the file or include transcript, media,
credentials, or arbitrary panic payload content. Network delivery is
best-effort and never blocks local editing.

Validation:

```text
pwsh -NoProfile -File scripts/validation/task50-telemetry.ps1
npx playwright test --workers=1
```

Production sign-off still requires a clean Windows run with telemetry disabled
and enabled, a captured sample payload, and offline failure verification.
