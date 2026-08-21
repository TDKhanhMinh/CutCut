# Editor fixtures

The E2E suite keeps media fixtures local. `sample.mp4` is a one-second generated
H.264 clip used for smoke coverage; `corrupted.mp4` is a legacy truncated
container and `corrupted-container.mp4` is a deterministic invalid-container
negative fixture.
`portrait.mp4` is a 9:16 video-only clip and `no-audio.mp4` is a 16:9 video-only
negative-path fixture. `Untitled.cutcut` is a portable project JSON fixture with
a single source media reference. `Unicode-Việt.cutcut` covers Vietnamese
content and `missing-media.cutcut` covers relink/recovery. Tests must never
upload these files or use a real user project.
