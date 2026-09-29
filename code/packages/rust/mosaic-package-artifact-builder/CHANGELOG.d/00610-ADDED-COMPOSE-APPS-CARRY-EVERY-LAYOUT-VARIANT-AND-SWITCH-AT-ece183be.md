### Added — Compose apps carry every layout variant and switch at run time (UI48 ENV2/ENV3, §7.5)

- Compose variants are compiled with `from_pipeline_variant`, and every
  export's `<C>.<variant>.kt` is copied into `src/main/kotlin` beside the
  default, so Gradle compiles every layout.
- `compose_layout_choices` takes the same rules as SwiftUI
  (`[[app.layouts]]` or the conventions), refuses a rule for a missing
  variant, and names each variant's composable.
- `MosaicAppShell.kt` emits the rules as data keyed by wire names and
  `mosaicLayoutVariant(environment)` (first match, else the default), and the
  root becomes `when (mosaicLayoutVariant(environmentReport)) { "touch" ->
  EngramAppTouch(...) else -> EngramApp(...) }`. A sample shell with
  variants measures the window (to choose) but reports to nobody. A package
  without variants gets the same output as before.

