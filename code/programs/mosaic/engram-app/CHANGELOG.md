# Changelog

## Unreleased

- Added shared browser support for Anki `preset:` deck option searches and
  `prop:pos` / `prop:position` new-card queue position filters.
- Aligned `note:` / `noteType:` and `card:` / `template:` browser filters with
  Anki-style exact-or-wildcard name matching.
- Aligned `tag:*` browser searches with Anki's universal tag-filter behavior.
- Added imported Anki card-flag support for `flag:` and `is:flagged` browser
  searches.
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
