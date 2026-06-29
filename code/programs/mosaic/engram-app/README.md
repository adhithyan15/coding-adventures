# engram-app

Engram's Mosaic app package.

This package is the product assembly layer. It exports `EngramApp`, owns the
app/root surface, and depends on reusable Mosaic component packages such as
`mosaic-pkg-card-browser`, `mosaic-pkg-deck-stats`,
`mosaic-pkg-review-actions`, `mosaic-pkg-review-card`, and
`mosaic-pkg-session-progress`.
The review card composes further Mosaic packages such as
`mosaic-pkg-rating-controls`; Engram does not fork those components into the app
package.

Reusable UI components should live under `code/packages/mosaic-pkg-*`. Engram
itself should grow here as an app package that composes those components and
binds them to the shared Rust business logic core through host shells.

## Current surface

- `EngramApp.mil` defines the app-facing review slots and events.
- `EngramApp.mll` owns the product shell and mounts
  `pkg::mosaic-pkg-card-browser::CardBrowser`,
  `pkg::mosaic-pkg-deck-stats::DeckStatsPanel`,
  `pkg::mosaic-pkg-session-progress::SessionProgress`, and
  `pkg::mosaic-pkg-review-card::ReviewCard`, plus
  `pkg::mosaic-pkg-review-actions::ReviewActions`.
- `EngramApp.dark.msl` owns app-shell styling only.
- Package artifact builds inline component-package styles through the full
  dependency chain.
- The generated React, SwiftPM, and Flutter shells mount `EngramApp` with
  sample slot values and dispatch callbacks, matching the generated interface
  shapes.
- Smoke tests now assert the generated Qt, SwiftUI, and XAML project shells
  expose the same Engram host contract slots, card-browser events, rating
  events, and Anki-style review action events as the shared Rust
  `EngramSession::engram_app_props` facade.

## Running the smoke test

```bash
cd code/programs/mosaic/engram-app
cargo test
```
