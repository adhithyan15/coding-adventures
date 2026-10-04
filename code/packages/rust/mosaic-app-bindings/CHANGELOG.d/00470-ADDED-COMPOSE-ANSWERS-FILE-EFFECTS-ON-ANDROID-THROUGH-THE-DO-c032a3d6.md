### Added — Compose answers file effects on Android through the document picker

- Compose on Android answers `files.open` and `files.save` (UI89 §3.8):
  `compose_android_platform_effects()`, written as the Android project's
  `MosaicPlatformEffects.kt`, implements the shared picker seam on the
  Storage Access Framework. `MosaicAndroidDocumentPicker` registers an
  `OpenDocument` launcher and a per-request `ACTION_CREATE_DOCUMENT`
  contract in the activity's `onCreate`; documents are read and written
  through the application's `ContentResolver` on a background thread, the
  open bounded while reading. The answer's name is the provider's display
  name when it is an ordinary name ("document" otherwise), and a failed
  write is reported without deleting anything -- the document may be one
  the person already had.
  `installMosaicPlatformEffects(host, picker)` returns the router, so the
  activity can fail a request whose picker can no longer answer.
