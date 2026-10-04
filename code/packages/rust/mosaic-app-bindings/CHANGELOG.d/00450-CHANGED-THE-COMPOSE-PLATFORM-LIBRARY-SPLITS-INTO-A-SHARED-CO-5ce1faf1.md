### Changed — the Compose platform library splits into a shared core and the desktop dialogs

- The Compose platform library is two files (UI89 §3.8, step 6's first
  part). `MosaicFileEffects.kt` (`compose_file_effects()`), new, imports
  nothing from AWT or Android: the rules a request must meet, the MIME table,
  routing, `mosaicCheckSaveRequest`, the picker seam (`MosaicDocumentPicker`,
  `MosaicOpenedDocument`, `MosaicSaveTarget`, `MosaicAccept`,
  `MosaicFileFailure`), `mosaicAnswerFilesOpen` / `mosaicAnswerFilesSave`
  and `MosaicPlatformRouter`, which now takes a picker and an optional
  `runInBackground` (its outcome handed back through `runOnUi`), gains
  `failPending`, and is typed on `MosaicPlatformEffectHost`, a new interface
  in `MosaicRuntimeHost.kt` that `MosaicRuntimeHost` implements. `MosaicPlatformEffects.kt`
  keeps the desktop's AWT dialogs and in-place writes, adapted by
  `MosaicDialogPicker`; `mosaicRunFilesOpen`, `mosaicRunFilesSave` and
  `installMosaicPlatformEffects` keep their signatures and outcomes.
- The Compose harness checks the asynchronous path with fake pickers: an
  answer that arrives later and is read in the background, a picker that
  answers twice, a cancel, a read over the limit, error text that never
  reaches the app, a picker that cannot be shown, a refused save that shows
  no picker, a provider-chosen name, and a failed write or hand-off; and,
  with a fake host, the router's routing, deferral, one-at-a-time rule,
  background hand-off and `failPending`.
- Every path answers: a throw on the background thread is a failure, an
  answer the host cannot take is answered again with a small failure, and
  `mosaicReadBounded` gives up on a stream that only ever returns nothing.
- The contract tests read the Kotlin rules from both files.
