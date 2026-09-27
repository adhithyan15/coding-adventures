### Added — Compose apps report their environment (UI48 ENV4, §7.4)

The strict `MosaicAppShell.kt` wraps the app's root in `BoxWithConstraints`
and reports the environment to the runtime from a `LaunchedEffect` keyed on
the six values, so it is sent when the window is first measured and again
only when a bucket flips: `sizeClass` at 600 / 1024 dp (SwiftUI's
thresholds), `orientation` from height against width, `colorScheme` from
`isSystemInDarkTheme()`, and the host's pointer, hover and reduced motion.
It reaches `reportEnvironment` through `as? MosaicRuntimeHost`, so
`MosaicComposeHost` and its test copies are unchanged, and only an answer that
carries props replaces what is showing. A sample shell does not observe.
