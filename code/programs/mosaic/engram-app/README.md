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
  and the `engram-wasm` APKG byte API. The browser WASM build now performs
  **Anki import and export itself — every format**, rather than reporting a
  native-host delegation error: the package layer builds for `wasm32` since the
  export moved onto the zero-dependency `sqlite-file` writer and decompression
  onto the repo's own `zstd`, which decodes the Huffman and FSE frames real
  encoders emit. Legacy V11 `.apkg` and modern `.anki21b` / `.colpkg` behave
  identically in the browser and in native hosts.
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
- **Qt, SwiftUI, Compose and Flutter reach the engine through the standard
  Mosaic runtime**, not through `engram-capi`. Props, events, snapshot and
  restore all go through `engram-mosaic-app` and the generated binding. The
  only backend-specific file each still ships is an effect handler, declared
  under `[host_effects]` and installed into the generated app:

  | backend | handler | dialogs |
  | --- | --- | --- |
  | Qt | `host/qt/engram_effects.cpp` | `QFileDialog`, `QMessageBox` |
  | SwiftUI | `host/swiftui/EngramEffects.swift` | `NSOpenPanel` / `NSSavePanel` |
  | Compose | `host/compose/EngramEffects.kt` | `JFileChooser`, `JOptionPane` |
  | Flutter | `host/flutter/engram_effects.dart` | `file_selector` |

  Each used to override the generated host with a hand-written `engram-capi`
  binding — 654 lines on Qt, 574 on Compose, 730 on Flutter — that reimplemented
  the whole application boundary against the bespoke ABI. Qt was retired first
  (#13728), once UI47 gave `Effect` a completion path and the file dialogs had
  somewhere to live; SwiftUI, Compose and then Flutter followed. XAML still
  ships its own `MosaicHost`.

  Flutter's override did not merely duplicate the runtime — it broke the build.
  It defined `load()` but not `loadRequired()`, which the native-complete
  `main.dart` calls, so `flutter analyze` on the emitted project reported
  `The method 'loadRequired' isn't defined for the type 'MosaicHost'` and
  nothing compiled. Nothing caught it because `engram-app` was not in the
  Flutter CI lane's acceptance set, so no build had ever emitted Engram on this
  backend with `--profile native-complete`. Both halves are fixed.

  The handlers differ in one way worth knowing before reading them. Qt answers
  inline, because its settle runs on the event-loop thread. SwiftUI and Compose
  must `deferEffect` first: their hosts hold a lock across the handler call, so
  opening a modal dialog inline would hold it for as long as the dialog is
  open. Compose additionally marshals to the EDT, because the props-changed
  handler runs on whichever thread answered and Compose state must be written
  from the UI thread. Flutter defers for a different reason again —
  `effectHandler` is synchronous and the pickers return `Future`s, so there is
  no inline answer to give — and it marshals nothing, because a Dart isolate is
  single-threaded and a deferred answer has no other thread to reach.
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

## Looking at it

Engram is gated on SwiftUI and Qt, and both gates assert the **semantics
tree** — that a node exists, is named, and is marked displayed. All of that can
be true of an app that is unreadable, and for the deck-stat chips it was: the
count was drawn on top of its label (#14828) and every gate stayed green.

To render Engram on Compose Desktop and look at it:

```bash
scripts/render-compose.sh /tmp/engram-shots
```

That builds the Rust runtime, generates the Compose project under
`native-complete` (asserted, so a degraded fallback cannot be mistaken for the
product), runs `conformance/compose/EngramScreenshots.kt`, and writes PNGs.

It is deliberately not a pixel-diff gate — a strict baseline needs a pinned
font stack and renderer, which is a separate decision (#14798). Measure a
defect against the semantics tree before filing it; the render tells you where
to look, not what is wrong.

## Running the smoke test

```bash
cd code/programs/mosaic/engram-app
cargo test
```

## Building the web host

```bash
cd code/programs/mosaic/engram-app
./scripts/build-web.sh --build
```

Compiles the engine to wasm, emits the app as a complete React/Vite project, and
installs the wasm runtime into it. `--build` also produces `dist/`; without it the
script stops at a project ready for `npm install && npm run dev` (a local
convenience; the build itself installs from the committed lock, below). `--theme light`
selects the light stylesheet.

The runtime-install step is not optional: the emitted `src/engram-host.ts` imports
`./engram-mosaic-host-wasm`, and emission copies only the `.d.ts` — the loader
itself lives in `engram-wasm/js`. Without it the build fails with
`Could not resolve "./engram-mosaic-host-wasm"`. That step previously existed only
in `build-all.ps1`, which cannot run on a Linux CI runner.

## Pinned npm dependencies

`build-web.sh --build` and `build-electron.sh --build`/`--package` install the
emitted project with `npm ci` from a lockfile committed here, never with a bare
`npm install`:

| Emitted project | Lockfile                         | Regenerate with                           |
|-----------------|----------------------------------|-------------------------------------------|
| `react/`        | `npm/web/package-lock.json`      | `./scripts/build-web.sh --update-lock`      |
| `electron/`     | `npm/electron/package-lock.json` | `./scripts/build-electron.sh --update-lock` |

The emitter pins every direct dependency, but Vite, Electron and
electron-builder pull in several hundred more that only a lockfile fixes (to one
version and one integrity hash each). The emitted project is regenerated on
every build, so the lock lives in this package and is copied in before `npm ci`.

When the emitter changes an npm version, `npm ci` refuses the stale lock, and so
do two tests that catch it on the PR rather than at release time:
`tests/npm_lockfiles.rs` here and `tests/engram_npm_lockfiles.rs` in
`mosaic-package-artifact-builder`. Run the matching `--update-lock` (it skips the
wasm build and installs nothing), review the lockfile diff, and commit it.
`build-electron.sh` also pins the packaging tools, `electron-builder` and
`@electron/asar`, as devDependencies so they come from the lock too.

Two more guards: `npm ci` runs with `--ignore-scripts` (nothing in the build
needs an install script, and their fallbacks fetch unpinned code), and
`--update-lock` resolves only versions published at least seven days earlier,
since most compromised npm releases are pulled within days. The lock test also
fails on an entry no dependency reaches, a tarball that is not the entry's own,
and any new package that declares an install script. It cannot tell one
in-range version from another (that needs registry data), so a lockfile diff is
reviewed like code.

The Electron runtime itself is not an npm package: the `electron` package only
fetches it. `build-electron.sh --package` runs Electron's own `install.js`, which
checks the downloaded zip against `checksums.json` inside the lock-pinned
`electron` tarball, and points electron-builder at that copy (`electronDist`)
instead of letting it download one of its own.

## Android

CI builds Engram for Android and runs it on an x86_64 emulator (UI89 §3.10).

- **Engine:** `engram-mosaic-app` is built for the four Android ABIs and
  packaged into the generated Compose project's debug APK. Its dependencies
  are pure Rust, so the NDK is the only toolchain it needs.
- **Gate:** three launches must write state under the app's `filesDir`,
  restore it, and quarantine state the runtime refuses.
- **Layout:** the touch layout reaches the Android project with the default
  one, and the layout rules choose between them on a phone as they do on the
  desktop.
- **Anki import and export:** desktop-only. Their handler is installed by the
  desktop hosts' `Main` and never reaches the APK, and CI fails if it does.
  On Android the host fails these two effects as unanswered.
- **Identity:** the manifest defaults, `dev.codingadventures.engramapp` and
  `EngramApp`.

## iOS and iPadOS

CI also runs Engram on the iPhone and iPad simulators (UI89 §2.5):

- **Engine:** linked statically from an `.xcframework`, into the
  generated Xcode project's simulator app.
- **Gate:** three launches must write state into the app's container,
  restore it, and quarantine refused state. The same app then runs on an
  iPad simulator.
- **Anki import and export:** the SwiftUI handler `EngramEffects.swift` is
  compiled into the iOS app. On iOS it answers these two effects through the
  platform library's document picker (UI89 §2.6), with the macOS rules:
  `.apkg`/`.colpkg`, the 256 MiB import limit, and the strict package check
  on export.
- **Identity:** the defaults, `dev.codingadventures.engramapp` and
  `EngramApp`.

## Emitting every host shell

```powershell
cd code/programs/mosaic/engram-app
./scripts/build-all.ps1
```

Windows only, and it emits permissively — it passes neither `--profile
native-complete` nor `--runtime-library`, so it does not check the degradation
report. `build-web.sh` above is the cross-platform path for the web backend.

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
Flutter, and Compose, the `file_selector` dependency the Flutter effect handler
needs for its pickers, JNA/JSON dependencies for the Compose host bridge,
`engram_capi.dll` as XAML project content, and `engram-host-cli` for the
Electron APKG sidecar.
Collection actions such as Anki import/export return `hostIntent` payloads so
hosts can open file pickers, call package bridges, and keep the Mosaic app
interface shared.
