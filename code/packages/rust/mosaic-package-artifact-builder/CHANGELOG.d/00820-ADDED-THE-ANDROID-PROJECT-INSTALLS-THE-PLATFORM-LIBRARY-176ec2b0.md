### Added — the Android project installs the platform library

- The Android project carries the platform library (UI89 §3.8): the shared
  `MosaicFileEffects.kt`, copied from the desktop project, and Android's own
  `MosaicPlatformEffects.kt`, the document picker. `MosaicActivity` builds
  the picker in `onCreate`, installs the library on the host as it loads
  (strict and sample alike), and fails a request whose picker is still open
  when it is destroyed. `MosaicAndroidDocumentPicker` joins the Compose
  shell's reserved names.
- The Android project's README no longer says the runtime is not bundled
  (step 5 bundles it with `--runtime-library`) or that file effects are still
  to come.
