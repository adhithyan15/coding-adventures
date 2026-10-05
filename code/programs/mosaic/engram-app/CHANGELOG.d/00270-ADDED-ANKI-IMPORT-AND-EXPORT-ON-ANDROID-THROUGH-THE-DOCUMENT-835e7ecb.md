### Added — Anki import and export on Android through the document picker (UI89 §3.12)

- `host/android/EngramAndroidEffects.kt` is Engram's Android handler. The
  manifest declares it as a `compose-android` `[host_effects]` file, with a
  handler claiming `importAnki` and `exportAnki`. The generated
  `MosaicActivity` installs it ahead of the platform library.
- It hands both kinds to the platform library's router, which uses the
  system document picker, with the iOS rules:
  - Import opens a `.apkg` or `.colpkg` within Engram's 256 MiB limit and
    answers `ok { apkg }`.
  - Export checks its package with strict base64 and a zip local header. It
    saves as `.apkg` only, under `suggestedName` with `.apkg` added when it
    is missing. A name the library would refuse falls back to
    `engram.apkg`. The decode runs on the main thread, so a package past
    the 256 MiB limit is refused before decoding, and running out of
    memory is answered rather than crashing.
  - Without a router it answers "file dialogs are not available on this
    platform". A refusal never throws.
- The desktop handler still stays out of the APK. CI now requires the
  Android handler's class and install function in the dex, and the install
  line in the generated activity.
