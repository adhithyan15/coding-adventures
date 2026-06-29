# Changelog

## Unreleased

- Added the reusable `mosaic-pkg-review-history` dependency and mounted its
  `ReviewHistoryPanel` component so generated host shells expose shared
  review totals, accuracy, per-rating counts, and first/last review fields.
- Added the reusable `mosaic-pkg-deck-options` dependency and mounted its
  `DeckOptionsPanel` component so generated host shells expose shared
  Anki-style deck scheduler option controls.
- Expanded deck options with native checkbox bindings for Anki-style sibling
  burying defaults.
- Expanded the generated deck option contract with learning/relearning step
  list slots and events.
- Routed generated deck option change events through the shared Engram facade
  so native/web hosts can persist settings without platform-specific reducers.
- Updated generated React and Electron renderer shells to use the Mosaic host
  adapter contract (`window.mosaicHost.getProps` / `handleEvent`) with sample
  fallback props, so Engram events can be routed to shared Rust-backed hosts.
- Updated generated Electron preload/main shells to expose those host adapter
  calls over context-isolated IPC channels instead of a placeholder host object.
- Added the Engram WASM Mosaic host bridge so generated React/Electron shells
  can consume shared Rust facade props/events with generated camelCase prop
  names.
- Added the reusable `mosaic-pkg-collection-actions` dependency and mounted its
  `CollectionActions` component so generated host shells expose shared
  collection counts, Anki import/export intents, and note/note-type workflow
  events.
- Routed generated Mosaic browser events through the shared Engram event facade,
  including card-ID-targeted mark and suspend actions for browser rows.
- Added browser result and selected-card metadata slots to the Engram app
  contract, backed by `EngramSession::engram_browser_props`, so generated host
  shells can route browser actions by card ID.
- Added the reusable `mosaic-pkg-card-browser` dependency and mounted its
  `CardBrowser` component so the Mosaic app exposes Anki-style browser/search
  slots and events through the shared host contract.
- Added the reusable `mosaic-pkg-deck-stats` dependency and mounted its
  `DeckStatsPanel` component in the app shell.
- Added the reusable `mosaic-pkg-session-progress` dependency and mounted its
  `SessionProgress` component in the app shell.
- Added the reusable `mosaic-pkg-review-actions` dependency and mounted its
  `ReviewActions` component so Mosaic/native review screens expose undo, bury,
  suspend, and mark events through the shared Engram event bridge.
- Added a multi-backend artifact-builder smoke test proving `EngramApp` emits
  through HTML, React, SwiftUI, Qt, XAML, and Flutter while consuming
  `mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
  `mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
  `mosaic-pkg-review-actions`, `mosaic-pkg-review-card`, and
  `mosaic-pkg-session-progress`.
- Added `scripts/build-all.ps1` to emit HTML, WebComponent, React, Electron,
  SwiftUI, Qt, XAML, and Flutter host shells from the same Engram Mosaic app
  package into `target/mosaic-engram-app/`.
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
