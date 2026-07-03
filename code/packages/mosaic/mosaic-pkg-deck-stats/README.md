# mosaic-pkg-deck-stats

A reusable Mosaic component package for deck-level spaced-repetition stats.

`DeckStatsPanel` owns the target-neutral deck summary surface for total, new,
due, learning, and hidden card counts. Engram and language-learning shells can
consume it without copying the same stats layout across HTML, Electron,
SwiftUI, XAML, Qt, Compose, and Flutter targets.

The package is intentionally component-scoped. Hosts compute the actual values
from the shared Engram core facade and pass localized labels plus formatted
values into the component.
