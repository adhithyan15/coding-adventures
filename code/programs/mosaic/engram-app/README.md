# engram-app

Engram's Mosaic app package.

This package is the product assembly layer. It exports `EngramApp`, owns the
app/root surface, and depends on reusable Mosaic component packages such as
`mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
`mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
`mosaic-pkg-note-editor`, `mosaic-pkg-note-type-editor`,
`mosaic-pkg-review-actions`, `mosaic-pkg-review-card`,
`mosaic-pkg-review-history`, and `mosaic-pkg-session-progress`.
The review card composes further Mosaic packages such as
`mosaic-pkg-rating-controls`; Engram does not fork those components into the app
package.

Reusable UI components should live under `code/packages/mosaic/mosaic-pkg-*`. Engram
itself should grow here as an app package that composes those components and
binds them to the shared Rust business logic core through host shells.
`mosaic-pkg-note-editor` provides the reusable focused-field note editor
surface for selected browser notes without folding editor controls directly
into the Engram app package. `mosaic-pkg-note-type-editor` does the same for
Basic-style note-type selection, draft creation, model renaming, stylesheet
editing, and save/delete/cancel controls.

## Current surface

- `EngramApp.mil` defines the app-facing review slots and events.
- `EngramApp.mll` owns the product shell and mounts
  `pkg::mosaic-pkg-card-browser::CardBrowser`,
  `pkg::mosaic-pkg-collection-actions::CollectionActions`,
  `pkg::mosaic-pkg-deck-options::DeckOptionsPanel`,
  `pkg::mosaic-pkg-deck-stats::DeckStatsPanel`,
  `pkg::mosaic-pkg-note-editor::NoteEditor`,
  `pkg::mosaic-pkg-note-type-editor::NoteTypeEditor`,
  `pkg::mosaic-pkg-review-history::ReviewHistoryPanel`,
  `pkg::mosaic-pkg-session-progress::SessionProgress`, and
  `pkg::mosaic-pkg-review-card::ReviewCard`, plus
  `pkg::mosaic-pkg-review-actions::ReviewActions`.
- `EngramApp.touch.mll` is the touch/mobile layout variant of the same
  product shell, keeping the `EngramApp.mil` interface and component mounts
  unchanged while stacking the header and navigation for narrow viewports.
- `EngramApp.dark.msl` and `EngramApp.light.msl` own app-shell styling only;
  component-package styling still comes from the package dependency chain.
- Package artifact builds inline component-package styles through the full
  dependency chain. Layout variants are emitted as suffixed artifacts such as
  `EngramApp.touch.*`; style themes are selected with the package build theme
  option, for example `--theme light`.
- The app shell includes host-status slots so import/export completion,
  cancellation, and host-side file errors can appear in every generated Mosaic
  UI instead of only in host adapter return objects.
- The web, Electron, Qt, SwiftUI, Compose, Flutter, and XAML host adapters
  merge their Anki import/export `hostResult` status back into those shared
  status slots.
- The Electron, web, and native host adapters preserve host-side error details
  in those status messages when a package read/import/export/write step fails.
- The generated React and Electron renderer shells mount `EngramApp` through
  `window.mosaicHost.getProps` and `window.mosaicHost.handleEvent`, with sample
  slot values as a fallback when no host is installed.
- The generated HTML, WebComponent, and React web hosts handle Anki
  import/export `hostIntent` payloads with browser file input/download helpers
  and the `engram-wasm` APKG byte API. The current browser WASM build still
  returns a native-host delegation error for APKG parsing/export, so the host
  reports that through `hostResult` until the package stack is made
  browser-buildable.
- The generated Electron preload/main shell exposes those calls over
  context-isolated IPC channels and can delegate them to an optional
  `electron/host.ts` or `MOSAIC_ELECTRON_HOST_MODULE` host module. The
  `engram-wasm` JS loader can serve that contract from the shared Rust facade,
  and Engram's Electron host handles Anki import/export intents with native
  dialogs plus a native `engram-host-cli` sidecar that imports/exports APKG
  files against the shared snapshot.
- The generated XAML project shell has an optional `MosaicHost` hook. Engram's
  `host/xaml/MosaicHost.cs` implements it with `engram-capi`, hydrating the
  generated WinUI dependency properties from the shared Rust facade and routing
  generated Mosaic event envelopes back into the same core.
- The XAML host also handles Engram's Anki import/export `hostIntent` payloads
  with WinUI file pickers, merging selected `.apkg` / `.colpkg` packages through
  the native C ABI and saving current collection state back to `.apkg`.
- The generated SwiftUI project shell has an optional `MosaicHost` hook.
  Engram's `host/swiftui/MosaicHost.swift` implements it with `engram-capi`
  through a staged `CEngram` Swift module, hydrating SwiftUI props and routing
  generated Mosaic event envelopes back into the same core. On macOS it also
  handles Anki import/export host intents with AppKit file panels, merging
  `.apkg` / `.colpkg` packages and saving current collection state through the
  native C ABI; non-macOS SwiftUI targets return an explicit unsupported result
  until Mosaic grows an async document-picker bridge.
- The generated Qt project shell has an optional `MosaicHost` hook. Engram's
  `host/qt/MosaicHost.h/.cpp` implements it with a runtime-loaded
  `engram-capi` library, hydrating QML properties and routing generated Mosaic
  event envelopes back into the same core. It also handles Anki import/export
  host intents with Qt file dialogs, merging `.apkg` / `.colpkg` packages and
  saving current collection state through the native C ABI.
- The generated Compose Desktop shell has an optional reflection-based
  `MosaicHost` hook. Engram's `host/compose/MosaicHost.kt` implements it with
  `engram-capi` through JNA, hydrating Compose slot props and routing generated
  Mosaic event envelopes back into the same core. It also handles Anki
  import/export host intents with desktop file choosers, merging `.apkg` /
  `.colpkg` packages and saving current collection state through the native C
  ABI.
- The generated Flutter shell has an optional `MosaicHost` hook. Engram's
  `host/flutter/mosaic_host.dart` implements it with Dart FFI and
  `engram-capi`, hydrating Flutter slot props and routing generated Mosaic
  event envelopes back into the same core. It also handles Anki import/export
  host intents with `file_selector` dialogs, merging `.apkg` / `.colpkg`
  packages and saving current collection state through the native C ABI.
- The web, Electron, and native host adapters persist raw Engram state snapshots
  across launches. Set `ENGRAM_SNAPSHOT_PATH` to override the storage file; by
  default host shells use `~/.engram/mosaic-snapshot.v1.json`.
- Smoke tests now assert the generated Qt, SwiftUI, and XAML project shells
  expose the same Engram host contract slots, collection events, card-browser
  events, rating events, and Anki-style review action events as the shared Rust
  `EngramSession::engram_app_props` facade.
- The browser slots include stable result and selected-card metadata from the
  Rust core so emitted native/web hosts can wire actions to card IDs instead of
  display labels.
- The browser state-filter slots and events expose common Anki search filters
  (`All`, `New`, `Due`, `Learning`, `Review`, `Suspended`, and `Buried`) as a
  target-neutral Mosaic dropdown while the Rust facade composes them with the
  free-form search query.
- The browser tag-edit slots and events are composed from
  `mosaic-pkg-card-browser` and route selected-card add/remove tag actions back
  into the shared Rust core, keeping Anki-style note tags available to every
  generated host shell.
- The browser flag slots and events expose Anki card flags from the shared
  search/progress model and route selected-card flag changes through
  `EngramCommand::SetCardFlag`.
- Browser open host intents carry selected note, template, and scheduling-state
  metadata for host-owned viewers. Browser edit hydrates the shared
  `mosaic-pkg-note-editor` surface without re-querying or duplicating browser
  selection logic.
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
Flutter, and Compose, the Dart FFI and `file_selector` dependencies for the
Flutter host bridge, JNA/JSON dependencies for the Compose host bridge,
`engram_capi.dll` as XAML project content, and `engram-host-cli` for the
Electron APKG sidecar.
Collection actions such as Anki import/export return `hostIntent` payloads so
hosts can open file pickers, call package bridges, and keep the Mosaic app
interface shared.
