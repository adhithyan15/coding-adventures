### Changed — the Compose app splits along its platform seams (UI89 §3.4)

Groundwork for Android, desktop behaviour unchanged:
- `Main.kt` is now only the desktop half (the window and, for a strict app,
  `loadMosaicHost`, where package effect handlers are installed);
  `MosaicAppShell.kt` holds what every Compose platform shares (`MosaicApp`,
  `MosaicStartup`, which now takes its host loader from the caller, the host
  interface, the prop helpers). `MosaicComposeHostBridge` is `internal`.
- `MosaicPlatform.kt`, the desktop half of drag and drop, ships beside the
  components and in the Gradle source set.

