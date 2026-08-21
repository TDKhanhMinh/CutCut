# CutCut UI localization convention

- The app UI locale is `en` or `vi`, detected from the OS/browser and persisted
  in `localStorage` under `cutcut-locale`. It is not part of Project JSON.
- User-visible UI strings use semantic keys through `useI18n().t(key)`; new
  strings must be added to both locale resources before they are rendered.
- `UI Locale != Transcript Language != Caption Language != AI Content
  Language`. Project names, filenames, model IDs, codecs, resolutions and
  transcript/caption content are never translated automatically.
- Use the shared `formatNumber`, `formatDate`, `formatDuration` and
  `formatBytes` helpers for locale-sensitive values. Use `{name}` interpolation
  for dynamic messages rather than concatenating translated fragments.
- Native/provider failures cross the boundary as stable error codes and are
  mapped with `translateErrorCode`; raw provider detail is secondary debug
  context only and must be safe to display.
- Missing keys fall back deterministically to Vietnamese and then the key
  itself, so a missing translation cannot crash the editor.
