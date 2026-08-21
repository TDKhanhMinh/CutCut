# Windows beta release, updater and rollback checklist

## Before publishing

1. Replace `UNCONFIGURED_MINISIGN_PUBLIC_KEY` in `src-tauri/tauri.conf.json`
   with the public key paired with the release signing secret. The private key
   must stay in the CI secret store and never enter this repository.
2. Publish signed NSIS/MSI artifacts and a manifest for each target/architecture
   at `https://releases.cutcut.app/{{target}}-{{arch}}.json`.
3. Verify the manifest signature, version ordering, HTTPS certificate and
   checksum from a clean Windows installation.
4. Install an older beta, update to the new version, and verify project JSON,
   native credential entries, cache and user-owned media paths are preserved.

## Rollback

- Keep the previous signed artifact and manifest available.
- If a release is unhealthy, publish the previous version with a higher build
  metadata/version according to the release policy; never install an unsigned
  binary or disable signature verification.
- Confirm the app can reopen a project created before and after the rollback.

The updater UI is fail-closed while the placeholder public key remains. This is
an implementation gate, not production approval.
