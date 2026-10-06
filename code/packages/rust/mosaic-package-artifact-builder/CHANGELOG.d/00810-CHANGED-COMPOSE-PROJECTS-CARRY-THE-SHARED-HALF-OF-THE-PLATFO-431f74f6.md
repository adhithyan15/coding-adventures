### Changed — Compose projects carry the shared half of the platform library

- Every Compose desktop project also gets `MosaicFileEffects.kt`, the shared
  half of the platform library (UI89 §3.8), beside `MosaicPlatformEffects.kt`.
  Its public types (`MosaicAccept`, `MosaicFileFailure`,
  `MosaicOpenedDocument`, `MosaicSaveTarget`, `MosaicDocumentPicker`,
  `MosaicSaveRequest`), the host's `MosaicPlatformEffectHost` and the
  desktop's `MosaicDialogPicker` join the
  Compose shell's reserved names. The Android project does not get it yet.
