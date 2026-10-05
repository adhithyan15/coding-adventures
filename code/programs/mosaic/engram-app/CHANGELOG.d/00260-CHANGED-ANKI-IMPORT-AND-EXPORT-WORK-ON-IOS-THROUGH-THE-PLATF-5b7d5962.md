### Changed — Anki import and export work on iOS through the platform picker (UI89 §2.6)

- On iOS, `EngramEffects.swift` no longer fails `importAnki` and `exportAnki`.
  It hands them to the platform library's router.
- Import opens a `.apkg` or `.colpkg` with Engram's own 256 MiB limit and
  answers `ok { apkg }`.
- Export checks its package exactly as on macOS: strict base64 and a zip
  local header. It then saves through the document picker as `.apkg` only,
  under `suggestedName` (gaining `.apkg` when it lacks one). A name the
  library would refuse falls back to `engram.apkg`.
- The name sanitising, extension filter and package check are now shared
  between macOS and iOS rather than macOS-only. The macOS panels behave as
  before.
