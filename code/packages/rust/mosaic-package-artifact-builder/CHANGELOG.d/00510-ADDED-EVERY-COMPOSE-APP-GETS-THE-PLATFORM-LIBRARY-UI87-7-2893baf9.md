### Added — every Compose app gets the platform library (UI87 §7)

- `MosaicPlatformEffects.kt` is written into `src/main/kotlin/` of every Compose
  project, beside `MosaicRuntimeHost.kt`.
- `Main.kt` installs it after the package's own handler, if any:
  `installMosaicPlatformEffects(it, setOf(...claimed kinds))`, or `null` when
  the package declared no `kinds` or no handler at all. A package with no
  Compose handler now gets exactly that one line.
- Compiled on Journal (library only), Engram (with its own handler) and
  photo-picker; Journal and Engram stay native-complete with no degradations.

