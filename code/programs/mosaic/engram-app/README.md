# engram-app

Engram's Mosaic app package.

This package is the product assembly layer. It exports `EngramApp`, owns the
app/root surface, and depends on reusable Mosaic component packages such as
`mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
`mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
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
  `pkg::mosaic-pkg-collection-actions::CollectionActions`,
  `pkg::mosaic-pkg-deck-options::DeckOptionsPanel`,
  `pkg::mosaic-pkg-deck-stats::DeckStatsPanel`,
  `pkg::mosaic-pkg-session-progress::SessionProgress`, and
  `pkg::mosaic-pkg-review-card::ReviewCard`, plus
  `pkg::mosaic-pkg-review-actions::ReviewActions`.
- `EngramApp.dark.msl` owns app-shell styling only.
- Package artifact builds inline component-package styles through the full
  dependency chain.
- The generated React and Electron renderer shells mount `EngramApp` through
  `window.mosaicHost.getProps` and `window.mosaicHost.handleEvent`, with sample
  slot values as a fallback when no host is installed.
- The generated Electron preload/main shell exposes those calls over
  context-isolated IPC channels so native hosts can bind them to app state.
- The generated SwiftPM and Flutter shells mount `EngramApp` with sample slot
  values and dispatch callbacks, matching the generated interface shapes.
- Smoke tests now assert the generated Qt, SwiftUI, and XAML project shells
  expose the same Engram host contract slots, collection events, card-browser
  events, rating events, and Anki-style review action events as the shared Rust
  `EngramSession::engram_app_props` facade.
- The browser slots include stable result and selected-card metadata from the
  Rust core so emitted native/web hosts can wire actions to card IDs instead of
  display labels.
- The collection slots expose note, note-type, and media counts plus shared
  Anki import/export and note workflow intents for host shells.
- The deck option slots expose the selected deck's shared scheduler settings,
  including learning/relearning steps, daily limits, graduation intervals,
  maximum interval, interval modifier, hard/easy multipliers, and lapse
  multiplier.
- Deck option events carry numeric values and route through the shared
  `EngramSession::handle_engram_app_event` contract, which persists them with
  `EngramCommand::SetDeckOptions`.

## Running the smoke test

```bash
cd code/programs/mosaic/engram-app
cargo test
```

## Emitting host shells

```powershell
cd code/programs/mosaic/engram-app
./scripts/build-all.ps1
```

The script writes HTML, WebComponent, React, Electron, SwiftUI, Qt, XAML, and
Flutter outputs under `target/mosaic-engram-app/` by default.
