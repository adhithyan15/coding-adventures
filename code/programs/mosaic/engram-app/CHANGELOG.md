# Changelog

## Unreleased

### Changed — Qt reaches the engine through the standard Mosaic runtime (#13728)

Engram's Qt project no longer overrides the generated `MosaicHost`. Props,
events, snapshot and restore go through `engram-mosaic-app` and the standard
binding; the only Qt-specific file left is `host/qt/engram_effects.cpp`, which
answers the Anki import and export effects with `QFileDialog`.

The 654-line `host/qt/MosaicHost.h/.cpp` is retired. It bound to `engram-capi`
— a bespoke ABI of 44 `eg_*` symbols — and reimplemented the entire application
boundary: session lifecycle, prop camel-casing, snapshot persistence, JSON
marshalling. None of that was Engram-specific; all of it is what the standard
runtime already does.

**What this unlocks, and what it cost to get here.** Qt could not reach
`native-complete` while the override existed: that profile switches the
generated `main.cpp` to the strict standard-binding shape — `registerTypes`,
`requireRuntime`, `configureRequiredProps`, `attach` — and the override exposed
`props`/`handleEvent`/`ensureLoaded` instead, so the combination did not
compile at all. The CI lane said so in a comment and deliberately gated the
weaker configuration.

It now emits with **`nativeComplete: true`, zero degradations, and an empty
`replacedGeneratedFiles`**, and the emitted project builds. That is an epic
definition-of-done bullet, and UI47 §5.5.5's stated test of whether this
migration worked.

The path ran through the whole of UI47: `Effect` needed a completion path
(steps 1–3), all five hosts needed to answer one (step 4), Engram needed to
emit its Anki intents as `Await` effects (step 5), and a package needed a way
to supply the handler that connects them (§5.5). Only then could the override
come off.

**Qt only.** SwiftUI, Compose, Flutter, XAML and the two web backends still
ship their own `MosaicHost` through `[host_assets]`, unchanged — each still
reimplements the boundary against `engram-capi`, and `engram-capi` itself stays
for them and for the Anki entry points the standard app ABI does not model.

#### The CI lane's pinned assertion, updated

`replacedGeneratedFiles` was pinned to `["MosaicHost.cpp", "MosaicHost.h"]`
specifically so this migration could not be forgotten. It is now pinned to `[]`
— pinned rather than dropped, because a package that starts overriding the
generated host again has undone this and should fail here rather than pass
quietly. The lane also asserts zero degradations and greps the generated
`main.cpp` for the install call, which is the half no assertion over a source
file can reach.

#### Sixteen substring assertions retired with the file

Engram's own suite asserted that the Qt override contained `eg_engram_app_props`,
`QLibrary`, `mosaicPropName`, `hydrateSession` and a dozen more. None of that
ships for Qt now, so those assertions were testing text nothing builds — the
"substring assertions over emitted text" problem the epic named, in its purest
form.

What replaces them is narrower because the file is: the handler answers two
effects and does nothing else. The assertions cover all three outcomes, because
a handler that only ever answers `ok` leaves a cancelled dialog looking like a
hang, and they pin `Qt::DirectConnection` — a queued connection would return
before the dialog answers, the sweep would fail the effect as unanswered, and
the eventual answer would be rejected as already completed.

#### From the security review of the handler

**An exception could strand the effect and kill persistence for the process.**
The handler runs inside the host's `settleEffects`, and the `emit` there has no
handler around it: anything thrown unwinds past it, `failOutstanding` never
runs, the id stays in `awaiting_`, and the runtime then refuses every snapshot
and restore for the life of the process. That is the silent permanent failure
the whole sweep mechanism exists to prevent, reached by throwing rather than by
forgetting — and an exception leaving a directly-connected slot is undefined
behaviour in Qt 6 besides. Both handlers are now wrapped, and the reason is
answered as the effect's failure, which discharges it and keeps persistence
alive.

**The import read was unbounded.** `readAll` with no size check, then base64,
then UTF-16, then the JSON envelope, then the runtime's own copy — roughly
seven times the file's size in flight, with the application's cap sitting at the
far end of all of it and so unable to prevent any of it. A multi-gigabyte file
would OOM, and `readAll` on a fifo or character device never reaches EOF at all.

Now sized up before it is opened, against the 256 MiB the package layer will
accept on native targets, so a file that cannot import is refused before it is
read rather than after. The regular-file test is separate work rather than a
restatement, because `QFileInfo::size()` reports 0 for a fifo.

The generic WebAssembly handler for this same protocol already bounded its read
at 16 MiB. Qt was the outlier.

**Two inputs are sanitised that the retired binding used to sanitise.**
`suggestedName` is forced to a bare filename — `QDir::filePath` joins a relative
path happily, so a suggestion carrying separators would open the save dialog in
a different directory with only the basename visible — and extensions must look
like extensions before reaching the dialog filter, where `)` and `;;` are
structural and `*?[` are globs. Neither is reachable today: nothing sends
`suggestedName`, and both extension lists are literals in the engine. The old
binding scrubbed its deck-derived name anyway, so accepting either unchecked
would be losing a check rather than never having had one.

The CI lane's degradation assertion now checks the key is an array before its
length, since `null | length` is `0` in jq and a report that lost the key would
otherwise pass.


- **Fixed: XAML layout variants generated duplicate partial classes.** The
  touch layout now emits its own `EngramAppTouch` class and event union, so the
  WinUI project can compile both layouts without duplicate members (#14234).

- The deck list on the home screen shows each deck's due and new counts, so the
  screen answers "what should I study?" without selecting each deck in turn.
- `onSelectDeck` carries a row index; see the `mosaic-pkg-deck-stats` changelog
  for why.

- **Fixed: the web bundle only worked when served from a domain root.** The
  React host's `WASM_URL` was `"/engram_engine.wasm"` — root-absolute — while
  its sibling `engram-host.mjs` had always used the relative form. The `.ts`
  copy is the one the Vite bundle ships. Combined with Vite's default
  `base: "/"`, the published v0.3.0 bundle returned 200 for `index.html` and
  404 for both its script and its engine when served from any subdirectory,
  rendering a blank page that looks like a working deploy.

  Verified by serving the corrected bundle from `/deep/nested/engram/`: entry
  point, hashed chunk, and wasm all 200, with the engine's magic bytes intact.
  The `base` fix is in the Mosaic React emitter, since Engram is the first app
  to build through `--emit-project` and so the first to depend on the
  generated Vite config at all.

- Added `scripts/build-web.sh`, a cross-platform build for the web host:
  compiles the engine to wasm, emits the app as a complete React/Vite project,
  and installs the wasm runtime into it. `--build` also produces `dist/`;
  `--theme` selects a stylesheet.

  The runtime-install step is what makes the emitted project buildable at all.
  `src/engram-host.ts` imports `./engram-mosaic-host-wasm` and emission copies
  only the `.d.ts`, so without it the build fails with
  `Could not resolve "./engram-mosaic-host-wasm"`. That step previously existed
  only in `build-all.ps1` — PowerShell, and therefore unavailable on the Linux
  CI runner, which is why no lane could produce a web bundle.

  With `--build` the script also `cmp`s the wasm in `dist/` against the one it
  compiled. Vite copies `public/` into `dist/`, and a missing engine there is a
  runtime failure behind a successful build — the same shape of bug the install
  step exists to prevent, so it is checked rather than assumed.

- **Fixed: the generated Qt project never compiled.** `main.cpp` calls
  `MosaicHost::registerTypes()` and `mosaicHost.attach(root)` on whatever
  `MosaicHost` the project ships. Engram installs its own over the generated one
  through `[host_assets]`, and it declared neither:

  ```
  main.cpp:27: error: 'registerTypes' is not a member of 'MosaicHost'
  main.cpp:47: error: 'class MosaicHost' has no member named 'attach'
  ```

  Added as no-ops, matching Mosaic's generated host, which declares both and
  leaves both empty — they are extension points, not behaviour. This host reaches
  the engine over `engram-capi` and needs neither QML type registration nor a
  root-object handle.

  Nothing caught this because nothing ever compiled the output: `build-all.ps1`
  emits without building, and `tests/package_compiles.rs` asserts on emitted text.
  The new Qt CI gate builds it, which is how it surfaced.

- Preserved Engram Anki import/export host-side error details in Qt, SwiftUI,
  and Compose `hostResult` statuses so generated shells show actionable read,
  import, export, and write failures instead of generic status text.
- Added `EngramApp.touch.mll`, a touch / mobile layout variant of the app shell
  (UI30). The desktop shell's horizontal header — app title beside a Row of six
  nav buttons — overflows on a phone-width viewport, so the touch variant stacks
  the header and nav vertically (Row → Column) while keeping the interface
  (`EngramApp.mil`) and every component mount / slot binding byte-for-byte
  identical. `mosaic-package-artifact-builder`'s `discover_variants` auto-emits
  it, so every backend now produces both `EngramApp` (desktop) and
  `EngramApp.touch` artifacts. Verified across all nine backends
  (react/html/webcomponent/swiftui/qt/flutter/compose/xaml); the emitted React
  differs from desktop only in the two `flexDirection: row → column` containers.
- Added a light-theme stylesheet (`EngramApp.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).
- Wired the generated HTML, WebComponent, and React Engram web hosts to handle
  Anki import/export `hostIntent` payloads with browser file input/download
  helpers and the `engram-wasm` APKG byte API, surfacing `hostResult` errors
  when the current browser WASM build delegates package parsing to native hosts.
- Wired the generated Flutter Engram shell to handle Anki import/export
  `hostIntent` payloads with `file_selector` dialogs and the shared
  `engram-capi` APKG import/export functions.
- Wired the generated Compose Desktop Engram shell to handle Anki import/export
  `hostIntent` payloads with desktop file choosers and the shared `engram-capi`
  APKG import/export functions.
- Wired the generated Electron Engram shell to handle Anki import/export
  `hostIntent` payloads with native Electron dialogs, snapshot persistence,
  and a native `engram-host-cli` sidecar for APKG import/export.
- Wired the generated SwiftUI macOS Engram shell to handle Anki import/export
  `hostIntent` payloads with AppKit file panels and the shared `engram-capi`
  APKG import/export functions, with an explicit unsupported result on
  non-macOS SwiftUI targets until Mosaic has an async document-picker bridge.
- Wired the generated Qt Engram shell to handle Anki import/export
  `hostIntent` payloads with Qt file dialogs and the shared `engram-capi` APKG
  import/export functions.
- Wired the generated XAML Engram shell to handle Anki import/export
  `hostIntent` payloads with WinUI file pickers and the shared `engram-capi`
  APKG import/export functions, so package files round-trip through the native
  Rust core instead of stopping at status text.
- Persisted Engram Mosaic SwiftUI, Qt, XAML, Flutter, and Compose native host
  snapshots through `engram-capi`, matching the web/Electron snapshot behavior
  with an `ENGRAM_SNAPSHOT_PATH` override and a shared home-directory fallback.
- Persisted Engram Mosaic web, React, WebComponent, and Electron host snapshots
  across launches, seeding from the built-in language-learning demo only when no
  saved state is available.
- Wired the generated Flutter Engram shell to the optional Mosaic host contract
  with a Dart FFI `engram-capi` adapter, shared slot hydration, and generated
  event-envelope routing through the Rust core.
- Added Anki `initialFactor` import/export and a shared initial-ease deck option
  control across the Rust core, WASM facade, and generated Mosaic shells.
- Aligned state-aware study queues with imported Anki new-card `due` positions
  so Mosaic/web/native hosts share the same new-card order.
- Cleared stale imported Anki card-row scheduling and flag fields after shared
  reducer mutations, keeping browser filters and APKG export current.
- Moved HTML/React host adapter activation into Mosaic package host asset
  emission, leaving `build-all.ps1` responsible for generated runtime binaries
  and loaders instead of editing generated shell entry files.
- Added shared browser support for Anki `preset:` deck option searches and
  `prop:pos` / `prop:position` new-card queue position filters.
- Aligned `note:` / `noteType:` and `card:` / `template:` browser filters with
  Anki-style exact-or-wildcard name matching.
- Aligned `tag:*` browser searches with Anki's universal tag-filter behavior.
- Added Anki-style no-combining `tag:nc:` browser searches.
- Aligned `deck:` browser searches for imported filtered cards with preserved
  original deck metadata.
- Added imported Anki card-flag support for `flag:` and `is:flagged` browser
  searches.
- Aligned `is:marked` / `marked:true` searches with Anki's `marked` note tag.
- Added imported Anki card-id timestamp support for `added:` browser searches.
- Added imported Anki card-row metric support for `prop:ivl`, `prop:reps`,
  `prop:lapses`, and `prop:ease` browser filters.
- Added shared browser support for Anki custom card data searches with
  `has-cd:`, `prop:cdn:`, and `prop:cds:` filters, including Anki's nested
  `cd` payload.
- Added imported Anki queue-aware browser filters for `is:buried-manually` and
  `is:buried-sibling`.
- Aligned browser `is:learn` / `is:review` semantics so relearning cards match
  Anki-style lapsed-card search intersections.
- Added imported Anki revlog-aware `resched:` and `prop:resched` manual
  reschedule browser filters, while excluding those rows from imported
  `rated:` searches.
- Normalized Anki-style recent-day browser searches so top-level `:0` windows
  behave as one-day searches for added, edited, introduced, rated, and
  rescheduled cards.
- Added Anki-style answer-button suffix support for `prop:rated`, such as
  `prop:rated<-7:again`.
- Aligned `introduced:` with Anki revlog semantics by ignoring imported manual
  reschedule rows when detecting a card's first real review.
- Treated unknown `key:value` browser searches as Anki-style custom field
  searches, enabling queries such as `Extra:` and `Sentence:re:...`.
- Aligned unqualified browser text searches with Anki note-field scope, while
  keeping standalone Engram cards searchable by front/back text.
- Added Anki-style `did:` deck ID and `mid:` notetype ID browser filters,
  including preserved original IDs from imported packages.
- Added Anki-style `dupe:notetype,text` duplicate first-field browser searches,
  including imported sort-field metadata and HTML/media filename normalization.
- Added imported Anki FSRS stability, difficulty, and retrievability browser
  filters via `prop:s`, `prop:d`, and `prop:r`.
- Aligned imported Anki state and due browser filters with preserved type,
  queue, due, original-due, and collection day metadata.
- Expanded the shared Engram browser search core with Anki-style `w:`, `nc:`,
  `sc:`, and `re:` text modifiers, field-scoped regex searches, tag regexes,
  and single-character `_` wildcards.
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
- Updated `scripts/build-all.ps1` to build the Engram WASM host and install the
  React/Electron host adapter assets into generated app shells automatically.
- Added an Engram XAML `MosaicHost` bridge that calls `engram-capi` for shared
  Rust facade props/events, and installs it into generated WinUI project shells.
- Added an Engram SwiftUI `MosaicHost` bridge plus `CEngram` module staging so
  generated SwiftPM shells can hydrate from and dispatch into `engram-capi`.
- Added an Engram Qt `MosaicHost` bridge that runtime-loads `engram-capi` and
  installs into generated Qt/CMake shells so QML properties/events share the
  Rust facade.
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
  through HTML, React, SwiftUI, Qt, XAML, Flutter, and Compose while consuming
  `mosaic-pkg-card-browser`, `mosaic-pkg-collection-actions`,
  `mosaic-pkg-deck-options`, `mosaic-pkg-deck-stats`,
  `mosaic-pkg-review-actions`, `mosaic-pkg-review-card`, and
  `mosaic-pkg-session-progress`.
- Added `scripts/build-all.ps1` to emit HTML, WebComponent, React, Electron,
  SwiftUI, Qt, XAML, Flutter, and Compose artifacts from the same Engram
  Mosaic app package into `target/mosaic-engram-app/`.
- Asserted that the generated React, SwiftPM, and Flutter shells mount
  `EngramApp` with sample slot values and dispatch callbacks instead of
  non-compiling empty initializers.
- Added a pinned Gradle Compose Desktop project shell for generated Compose
  artifacts and asserted it mounts `EngramApp` with sample slot values and
  Mosaic event-envelope logging.
- Asserted that nested package styles from `DeckStatsPanel`, `SessionProgress`,
  `ReviewActions`, `ReviewCard`, and `RatingControls` reach the generated
  Engram HTML artifact.

## 0.1.0

- Added the initial Engram Mosaic app package.
- Added an `EngramApp` root component that consumes `ReviewCard` from
  `mosaic-pkg-review-card`.
- Added smoke tests for manifest boundaries, source compilation, and component
  dependency resolution.
