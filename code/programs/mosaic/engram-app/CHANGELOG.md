# Changelog

## Unreleased

- Added the reusable `mosaic-pkg-deck-stats` dependency and mounted its
  `DeckStatsPanel` component in the app shell.
- Added the reusable `mosaic-pkg-session-progress` dependency and mounted its
  `SessionProgress` component in the app shell.
- Added the reusable `mosaic-pkg-review-actions` dependency and mounted its
  `ReviewActions` component so Mosaic/native review screens expose undo, bury,
  suspend, and mark events through the shared Engram event bridge.
- Added a multi-backend artifact-builder smoke test proving `EngramApp` emits
  through HTML, React, SwiftUI, Qt, XAML, and Flutter while consuming
  `mosaic-pkg-deck-stats`, `mosaic-pkg-review-actions`, `mosaic-pkg-review-card`, and
  `mosaic-pkg-session-progress`.
- Asserted that the generated React, SwiftPM, and Flutter shells mount
  `EngramApp` with sample slot values and dispatch callbacks instead of
  non-compiling empty initializers.
- Asserted that nested package styles from `DeckStatsPanel`, `SessionProgress`,
  `ReviewActions`, `ReviewCard`, and `RatingControls` reach the generated
  Engram HTML artifact.

## 0.1.0

- Added the initial Engram Mosaic app package.
- Added an `EngramApp` root component that consumes `ReviewCard` from
  `mosaic-pkg-review-card`.
- Added smoke tests for manifest boundaries, source compilation, and component
  dependency resolution.
