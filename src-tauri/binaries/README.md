# FFmpeg Sidecar Binaries

## Source
- Build: BtbN/FFmpeg-Builds (GitHub)
- URL: https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip
- Variant: win64-gpl (static build)
- Target: x86_64-pc-windows-msvc

## Naming Convention (Tauri Sidecar)
Binaries must follow the pattern `<name>-<target-triple>.exe`:
- `ffmpeg-x86_64-pc-windows-msvc.exe`
- `ffprobe-x86_64-pc-windows-msvc.exe`

Tauri resolves the correct binary automatically based on the host target triple.

## License Obligations
FFmpeg is licensed under **GPL v2+** (this build includes x264, x265, and other GPL libraries).

Before commercial distribution:
1. The full GPL license text must be included with the application.
2. The source code (or a written offer to provide it) for the FFmpeg build must be made available.
3. Consider switching to an LGPL build if relinking/dynamic linking is preferred for compliance.
4. Consult legal counsel for production distribution.

For the prototype phase, GPL is acceptable.

## Binary Sizes (approximate)
- ffmpeg.exe:  ~139 MB
- ffprobe.exe: ~139 MB

These are statically linked and self-contained — no additional DLLs required.
