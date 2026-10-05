### Added — Engram builds and runs on Android (UI89 step 7, §3.10)

CI now builds `engram-mosaic-app` for the four Android ABIs. It packages the
engine into the generated Compose project's debug APK and checks:
- the badging (`dev.codingadventures.engramapp`, `EngramApp`, `MosaicActivity`,
  SDK 26/36);
- that the dex holds the shell but not the desktop-only Anki handler
  (`EngramEffectsKt`);
- that every ABI's `libmosaic_app.so` exports `mosaic_app_create`.

On the x86_64 emulator, after Trestle and Journal, the APK must write its
state under `filesDir`, restore it, quarantine refused state, and keep
running. No Engram source changes; Anki import and export stay desktop-only.
