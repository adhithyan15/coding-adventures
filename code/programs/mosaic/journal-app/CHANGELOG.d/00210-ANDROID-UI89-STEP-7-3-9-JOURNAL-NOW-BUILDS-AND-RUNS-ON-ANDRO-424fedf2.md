- **Android (UI89 step 7, §3.9).** Journal now builds and runs on Android
  with no source changes. CI builds `journal-mosaic-app` for the four Android
  ABIs and packages it into the generated Compose project's debug APK. The
  badging must show `dev.codingadventures.journalapp`, `JournalApp` and
  `MosaicActivity`, and every ABI's `libmosaic_app.so` must export
  `mosaic_app_create`. On the x86_64 emulator, after Trestle, the APK must
  write its state under `filesDir`, restore it on relaunch, quarantine state
  the runtime refuses, and keep running.
