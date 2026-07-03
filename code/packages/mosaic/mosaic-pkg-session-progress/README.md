# mosaic-pkg-session-progress

A reusable Mosaic component package for spaced-repetition session counters.

`SessionProgress` owns the target-neutral current / remaining / correct / total
counter strip used by review screens. Engram and future language-learning
surfaces can consume it without copying the same counter layout across HTML,
Electron, SwiftUI, XAML, Qt, Compose, and Flutter targets.

The package is intentionally component-scoped. It does not own queue building,
review history, or scheduling. Hosts provide localized labels and already
computed counter values from the shared Engram core facade.
