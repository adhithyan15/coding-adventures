# engram-app

Engram's Mosaic app package.

This package is the product assembly layer. It exports `EngramApp`, owns the
app/root surface, and depends on reusable Mosaic component packages such as
`mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
`mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
`mosaic-pkg-note-editor`, `mosaic-pkg-review-actions`, `mosaic-pkg-review-card`,
`mosaic-pkg-review-history`, and `mosaic-pkg-session-progress`.
The review card composes further Mosaic packages such as
`mosaic-pkg-rating-controls`; Engram does not fork those components into the app
package.

Reusable UI components should live under `code/packages/mosaic-pkg-*`. Engram
itself should grow here as an app package that composes those components and
binds them to the shared Rust business logic core through host shells.
`mosaic-pkg-note-editor` provides the reusable focused-field note editor
surface for selected browser notes without folding editor controls directly
into the Engram app package.

## Current surface

- `EngramApp.mil` defines the app-facing review slots and events.
- `EngramApp.mll` owns the product shell and mounts
  `pkg::mosaic-pkg-card-browser::CardBrowser`,
  `pkg::mosaic-pkg-collection-actions::CollectionActions`,
  `pkg::mosaic-pkg-deck-options::DeckOptionsPanel`,
  `pkg::mosaic-pkg-deck-stats::DeckStatsPanel`,
  `pkg::mosaic-pkg-note-editor::NoteEditor`,
  `pkg::mosaic-pkg-review-history::ReviewHistoryPanel`,
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
  context-isolated IPC channels and can delegate them to an optional
  `electron/host.ts` or `MOSAIC_ELECTRON_HOST_MODULE` host module. The
  `engram-wasm` JS loader can serve that contract from the shared Rust facade.
- The generated XAML project shell has an optional `MosaicHost` hook. Engram's
  `host/xaml/MosaicHost.cs` implements it with `engram-capi`, hydrating the
  generated WinUI dependency properties from the shared Rust facade and routing
  generated Mosaic event envelopes back into the same core.
- The generated SwiftUI project shell has an optional `MosaicHost` hook.
  Engram's `host/swiftui/MosaicHost.swift` implements it with `engram-capi`
  through a staged `CEngram` Swift module, hydrating SwiftUI props and routing
  generated Mosaic event envelopes back into the same core.
- The generated Qt project shell has an optional `MosaicHost` hook. Engram's
  `host/qt/MosaicHost.h/.cpp` implements it with a runtime-loaded
  `engram-capi` library, hydrating QML properties and routing generated Mosaic
  event envelopes back into the same core.
- The generated Compose Desktop shell has an optional reflection-based
  `MosaicHost` hook. Engram's `host/compose/MosaicHost.kt` implements it with
  `engram-capi` through JNA, hydrating Compose slot props and routing generated
  Mosaic event envelopes back into the same core.
- The generated Flutter shell has an optional `MosaicHost` hook. Engram's
  `host/flutter/mosaic_host.dart` implements it with Dart FFI and
  `engram-capi`, hydrating Flutter slot props and routing generated Mosaic
  event envelopes back into the same core.
- Smoke tests now assert the generated Qt, SwiftUI, and XAML project shells
  expose the same Engram host contract slots, collection events, card-browser
  events, rating events, and Anki-style review action events as the shared Rust
  `EngramSession::engram_app_props` facade.
- The browser slots include stable result and selected-card metadata from the
  Rust core so emitted native/web hosts can wire actions to card IDs instead of
  display labels.
- The browser tag-edit slots and events are composed from
  `mosaic-pkg-card-browser` and route selected-card add/remove tag actions back
  into the shared Rust core, keeping Anki-style note tags available to every
  generated host shell.
- The browser flag slots and events expose Anki card flags from the shared
  search/progress model and route selected-card flag changes through
  `EngramCommand::SetCardFlag`.
- Browser open/edit host intents also carry selected note, template, and
  scheduling-state metadata so host editors can be launched without re-querying
  or duplicating browser selection logic.
- The collection slots expose note, note-type, and media counts plus shared
  Anki import/export and note workflow intents for host shells.
- The deck option slots expose the selected deck's shared scheduler settings,
  including learning/relearning steps, daily limits, graduation intervals,
  initial ease factor, maximum interval, interval modifier, hard/easy
  multipliers, and lapse multiplier plus Anki-style sibling-bury defaults and
  FSRS desired-retention, parameter, search, ignored-history, historical
  retention, and easy-day factor fields.
- Deck option events carry numeric, text, or checkbox values and route through
  the shared `EngramSession::handle_engram_app_event` contract, which persists
  them with `EngramCommand::SetDeckOptions`.
- The review history slots expose lifetime deck review totals, accuracy,
  per-rating counts, and first/last review timestamps from the shared Rust
  history summary.

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

The script writes HTML, WebComponent, React, Electron, SwiftUI, Qt, XAML,
Flutter, and Compose outputs under `target/mosaic-engram-app/` by default. The
Compose backend emits a pinned Gradle Compose Desktop shell plus the reusable
Kotlin component source so it can be run with `gradle run`.

The script also builds `code/packages/rust/engram-wasm` and
`code/packages/rust/engram-capi`, then installs the generated runtime assets
that host adapters need. Static host adapters are declared in
`mosaic-package.toml`, so the Mosaic package builder copies and activates
`src/engram-host.ts`, `engram-host.mjs`, `electron/host.js`, and the native
bridge sources during project emission. The script adds the JS loader and
`engram_engine.wasm` for web/Electron shells, `Sources/CEngram` plus the static
`engram-capi` library for SwiftUI, the dynamic `engram-capi` library for Qt,
Flutter, and Compose, the Dart FFI dependency for the Flutter host bridge,
JNA/JSON dependencies for the Compose host bridge, and `engram_capi.dll` as
XAML project content. Collection actions such as Anki
import/export return `hostIntent` payloads so hosts can open file pickers or
save APKG bytes while keeping the Mosaic app interface shared.
