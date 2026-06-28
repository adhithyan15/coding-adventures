# Engram Anki Parity Plan

## Goal

Drive Engram from a focused flashcard prototype into an Anki-class study
system without losing the repo's cross-platform direction.

The product target is not a clone for its own sake. Anki parity means Engram
can faithfully support the habits and data shapes that make Anki powerful:
typed note models, generated cards, mature scheduling, import/export, deck
organization, media, search, review history, offline-first persistence, and
large personal collections. The larger language-learning app can then use
Engram as its memory substrate.

Engram must remain one product codebase. XAML, SwiftUI for macOS and iOS,
Electron, Qt, HTML, and any future shell should consume the same core model,
commands, generated UI contracts, and fixture suite. Target-specific projects
may own packaging and platform integration, but they must not fork product
logic or manually reimplement study behavior.

## Architecture Direction

Engram follows the same pattern as the VisiCalc demos:

```text
React Engram app
Lattice-owned web-shell styles
Mosaic-generated UI
HTML shell
Electron shell
XAML shell
SwiftUI macOS/iOS shell
Qt shell
        |
        v
engram-wasm / engram-capi / platform facades
        |
        v
engram-core
        |
        v
language-core, media-core, storage adapters, sync adapters
```

`engram-core` owns study behavior. UI shells own rendering, input, platform
storage, IDs, timestamps, file pickers, audio playback, and sync transport.

## Non-Negotiables

- Core learning logic lives in Rust.
- Engram-owned web styling is authored in Lattice, not hand-maintained CSS.
- Reusable Engram UI surfaces are authored as Mosaic packages, not per-target
  component forks.
- Host apps pass timestamps and IDs into the core.
- No UI dependencies in `engram-core`.
- No target shell gets a private copy of scheduling, search, import/export, or
  card-generation logic.
- Cross-target behavior is verified by shared fixtures wherever possible.
- Data migrations are explicit and tested.
- Import/export is round-trippable wherever practical.
- Mobile web is a first-class target, not an afterthought.
- Native targets should be unlocked through the Mosaic path, not hand-maintained
  divergent app logic.

## Workstream 1: Rust Core

### 1.1 Foundation

- Define deck, card, progress, session, review, and state types.
- Port SM-2 scheduling from the TypeScript app.
- Port due/new queue construction.
- Port basic reducer transitions.
- Keep `serde` optional for facade crates.
- Add unit tests mirroring TypeScript behavior.

Status: in progress.

Current reducer integration:

- `EngramCommand::RateCard` routes through the scheduler state machine with
  `DeckOptions::default()`.
- `EngramCommand::RateCardWithOptions` lets host shells pass deck-specific
  learning steps, review limits, graduation intervals, and lapse behavior.
- `engram-core-wasm` keeps the original `rateCard` JSON command compatible and
  accepts optional `deckOptions`.
- Suspend and bury are reducer-owned commands (`SuspendCard`, `UnsuspendCard`,
  `BuryCard`, `UnburyCard`) and are exposed through the JSON facade.
- Card flags and marks are reducer-owned metadata commands (`SetCardFlag`,
  `MarkCard`, `UnmarkCard`) and are exposed through the JSON facade.
- `search_cards` provides the first shared collection-browser query engine with
  text, deck, note type, field-side, tag, state, due, suspended, buried, flag,
  marked, negation, parenthesized grouping, and `OR` filters.
- Reviews carry optional previous/resulting progress snapshots so
  `UndoLastReview` can restore card progress, sibling bury snapshots, active
  queue state, review history, and session counters without host-specific
  logic.
- Durable cards can carry optional note/template lineage, and
  `BuryCardSiblings` uses that lineage to bury same-note sibling cards until a
  host-provided boundary.
- `RateCardAndBurySiblings` and `RateCardWithOptionsAndBurySiblings` let hosts
  apply Anki-style sibling burying atomically during review; the JSON facade
  exposes this as optional `burySiblingsUntil` on `rateCard`.
- Generated note-template cards can be materialized into durable `Card` records
  with lineage through the JSON facade and C ABI.
- Anki-style Cloze templates using `{{cloze:Field}}` now generate one card per
  `{{cN::text::hint}}` ordinal in Rust core, with cloze lineage exposed through
  the JSON facade and C ABI.
- `rename_note_type_field` / `EngramCommand::RenameNoteTypeField` migrate
  normal template references, Cloze template references, and required-field
  names while keeping note field IDs stable; the command is exposed through the
  JSON facade.
- `create_engram_snapshot` / `restore_engram_snapshot` define the versioned
  Engram JSON backup shape in Rust and are exposed through the JSON facade.
- `export_cards_csv` / `import_cards_csv` define a strict round-trippable card
  CSV shape and are exposed through JSON facade helpers for host preview flows.
- `import_basic_cards_csv` accepts simpler `front,back` CSV and uses
  host-supplied deck/timestamp/ID prefix options for deterministic generated
  cards.
- `export_cards_anki_basic_tsv` emits Anki-compatible Basic text-import files
  with tab separator headers and quoted fields, and is exposed through the JSON
  facade and C ABI.
- `import_anki_basic_tsv` parses Anki Basic front/back text files with headers
  and quoted fields, and is exposed through the JSON facade and C ABI.
- Note-backed Anki TSV import/export now supports Basic and Basic-and-reversed
  rows as `NoteType`, `Note`, and generated lineage cards, including Tags
  column preservation through the JSON facade and C ABI.
- Note-backed Anki TSV import also supports Cloze rows with `Text`, optional
  `Extra`, and `Tags` columns, producing Cloze note models and cloze lineage
  cards through the JSON facade.
- Note-backed Anki TSV import preserves custom note-type field columns as
  notes, including Tags metadata, without inventing cards when the source text
  file does not contain real template definitions.
- `summarize_review_history` derives deck-scoped review-log summaries for a
  timestamp range and is exposed through the JSON facade and C ABI.
- `get_daily_study_limit_usage` and
  `build_session_queue_with_daily_limits` subtract new/review reps already seen
  in a day window from `DeckOptions` and are exposed through the JSON facade and
  C ABI.
- `get_deck_stats` reports suspended and buried counts from the shared core
  hidden-card logic.

### 1.2 Notes and Card Templates

Anki's core abstraction is a note, not a card. A note contains fields and a
note type; cards are generated from templates.

Needed types:

- `NoteType`
- `FieldDef`
- `CardTemplate`
- `Note`
- `GeneratedCard`

Rules:

- A note type owns ordered field definitions.
- A template owns front/back rendering templates.
- A note generates zero or more cards depending on template requirements.
- Generated cards retain stable IDs across edits when the note/template
  identity and ordinal are stable.

Tests:

- One note can generate multiple cards.
- Empty required fields suppress invalid generated cards.
- Renaming a field migrates template references where possible. Initial Rust
  core support covers normal and Cloze template references plus required-field
  names, with reducer and JSON facade command coverage.
- Generated card IDs remain stable across harmless note edits.
- Cloze note templates generate stable sibling cards per cloze ordinal. Initial
  support exists in core plus JSON/C facades.

### 1.3 Scheduler Parity Track

The current scheduler is a compact SM-2 variant. Anki compatibility requires
at least:

- new, learning, review, and relearning queues
- learning steps in minutes
- graduating interval
- easy interval
- lapse handling
- bury siblings until next day. Core and JSON facade support exists for
  lineage-backed sibling burying both as a direct command and as an atomic
  `rateCard` option; UI controls/settings still need to bind to it.
- deck options
- daily limits. Core, JSON facade, and C ABI support exists for review-log-aware
  daily queue limits; UI settings still need to bind to it.
- review history log
- review history summaries for deck/date ranges. Core, JSON facade, and C ABI
  support exists; richer graphing and browser UI still need to bind to it.

Possible later track:

- FSRS-style scheduler support as a selectable scheduler.

Tests:

- New card graduates through learning steps.
- Failed review enters relearning.
- Sibling burying prevents template siblings from appearing in the same day.
- Deck limits are deterministic.
- Scheduler decisions are reproducible from a fixed clock.

### 1.4 Search and Browser Query Engine

Anki-level collections need fast filtering.

Support:

- text search across note fields and rendered cards
- deck filters
- note type filters
- due/new/learning/review state filters
- tag filters
- suspended/buried flags
- flag/mark filters
- simple boolean operators (`OR`, implicit AND, negation, and parentheses)

Tests:

- Query parser diagnostics. Initial core support exists for unknown filters and
  unterminated quotes.
- Search result stability. Initial core tests preserve source card order.
- Tag, deck, and note type filters compose with text filters. Initial core
  support exists for implicit AND, `OR`, negation, and parenthesized groups;
  richer expressions remain.
- Hidden-card stats. Deck stats now report suspended and buried counts from the
  same logic used by queues.

### 1.5 Import/Export Model

Core should expose portable snapshot types that facade crates can serialize.

Formats:

- Engram JSON collection snapshot. Initial Rust core and JSON facade support
  exists and accepts the current web backup shape.
- CSV deck import/export. Initial Rust support exists for full card CSV
  round-trips and simpler generated-ID `front,back` imports.
- Anki TSV text compatibility. Basic front/back import/export exists in core,
  JSON facade, and C ABI; note-backed Basic and Basic-and-reversed TSV
  import/export now creates notes, generated cards, and tag metadata. Cloze TSV
  import creates cloze note models and cards. Custom note-type TSV import now
  preserves arbitrary field columns as notes without generated cards. Richer
  custom note-template/media export remains.
- APKG import/export eventually, via a dedicated facade or package crate.
  `engram-anki-package` now provides the archive-inspection foundation for
  legacy and modern collection members plus legacy JSON media maps, and can
  resolve media payloads or write deterministic legacy package envelopes from
  existing `collection.anki2` bytes plus media assets. It also parses
  legacy/V11 SQLite collection bytes into owned Anki decks, note types, notes,
  cards, revlog rows, and graves, and maps them into `engram-core::AppState`
  with deterministic IDs, rendered card fronts/backs, cloze card rendering,
  and preserved scheduled-card color flags. The same crate now generates
  deterministic legacy/V11 SQLite collection bytes from `AppState` and wraps
  them in an APKG envelope for the first native export path.

Next APKG SQLite milestone:

- Target Anki legacy/V11 collection files first (`collection.anki2` and
  `collection.anki21`). `engram-anki-package` now exposes
  `read_v11_collection`, `parse_v11_collection_bytes`, and
  `read_v11_collection_bytes` as the package boundary for this reader.
- Keep `collection.anki21b` detected but explicitly unsupported until Engram
  adds modern package handling for V18, zstd-compressed collection payloads, and
  protobuf media entries.
- Added a real SQLite-file dependency in `engram-anki-package`; APKG-specific
  parsing remains outside `engram-core`.
- Parsed V11 decks from `col.decks`, models from `col.models`, notes from
  `notes.flds`/`notes.tags`, card lineage from `cards.nid`/`cards.ord`, card
  progress from `cards`, and review history from `revlog.ease`/`revlog.id` now
  map into `engram-core::AppState`.
- V11 due values now preserve Anki's distinction between intraday learning
  timestamps and collection-day-based review/day-learning due dates.
- Card flags on new V11 cards now import as metadata-only progress overlays;
  shared queue, stats, and search logic still treat those cards as new while
  preserving their flag filters.
- Search now follows `Card.lineage.note_id` before falling back to generated
  `note::template` card IDs, so imported numeric Anki card IDs still match
  note-field text, tags, and note-type filters.
- Template rendering now supports simple Anki-style conditional/inverted
  sections, `{{FrontSide}}` for generated/imported card backs, and `hint:` /
  `type:` field prefixes. APKG import now infers required fields from imported
  template question formats instead of making every model field mandatory.
- Imported V11 revlog rows now populate Engram review progress snapshots with
  previous/resulting intervals and ease factors, allowing legacy APKG export to
  round-trip `ivl`, `lastIvl`, and `factor` for imported reviews.
- `engram-core::AppState` now includes durable external source records.
  V11 import stores collection config/graves, raw deck/model JSON, note
  GUID/checksum/data, and card scheduler/filter metadata there; V11 export
  uses those records to preserve CSS/template extras, deck config, tags, note
  metadata, `odue`/`odid`, card data, and graves while still letting Engram's
  current fields win when edited.
- `engram-capi` now exposes `eg_parse_anki_apkg` for native import previews and
  `eg_import_anki_apkg` for applying supported APKG bytes into the shared
  session state. Both functions use the same JSON `{ ok, state/error }`
  contract as the rest of the native facade.
- Native package inspection and single-media extraction are available through
  `eg_inspect_anki_apkg` and `eg_read_anki_apkg_media`, so SwiftUI/XAML/Qt
  shells can inspect package contents and copy referenced audio/images without
  duplicating ZIP/media-map parsing.
- `eg_export_anki_apkg` now exports the current session as a deterministic
  legacy/V11 APKG JSON byte array for native shells. The writer preserves
  imported numeric IDs when possible, allocates stable numeric IDs for
  Engram-native rows, writes decks/models/notes/cards/progress/revlog rows, and
  falls back to a synthetic Basic note type for standalone front/back cards.
- Next export fidelity steps: export media payloads from durable state, map
  Anki deck options into Engram scheduler presets, and add modern `.anki21b`
  package writing after the V18 reader exists.
- SQL-built V11 fixtures and package round-trips through the existing ZIP
  envelope helpers now cover the parser. Next: add a small checked-in
  Anki-generated golden fixture.

Tests:

- JSON snapshot round-trip. Initial core/facade tests cover durable data,
  active-session clearing, web backup compatibility, and validation errors.
- CSV escaping and line ending behavior. Initial core tests cover quoting,
  embedded newlines, CRLF, blank rows, and shape errors.
- Import produces deterministic IDs when caller supplies an ID strategy. Initial
  support exists through `BasicCardCsvImportOptions`.

## Workstream 2: Facades

### 2.1 `engram-core-wasm`

JSON facade over `engram-core`, similar to `spreadsheet-core-wasm`.

Responsibilities:

- own a session object or snapshot boundary
- accept JSON commands
- return JSON state, queue, stats, or diagnostics
- no browser APIs

### 2.2 `engram-wasm`

Zero-dependency `extern "C"` plus linear-memory ABI, following the repo's
spreadsheet WASM convention.

Exports:

- `alloc`
- `dealloc`
- `reset`
- `load_snapshot`
- `dispatch`
- `get_state`
- `build_queue`
- `get_deck_stats`

### 2.3 `engram-capi`

Native C ABI over the JSON facade.

Consumers:

- SwiftUI
- Compose or Android
- Flutter
- Qt
- XAML

Status:

- `code/packages/rust/engram-capi` exposes the first native ABI over
  `engram-core-wasm`, including dispatch, snapshots/backups, queue/stats,
  generated cards, search, and CSV helpers.
- `engram-core-wasm::EngramSession::engram_app_props` and
  `eg_engram_app_props` return a flat props object keyed by the exact
  `EngramApp.mil` Mosaic slots, giving HTML/React/native shells one shared
  binding surface for deck stats, session progress, and the current review
  card.
- Native APKG byte-slice imports are now available through `eg_parse_anki_apkg`
  and `eg_import_anki_apkg`, backed by `engram-anki-package` and returning the
  same JSON result shape as other C ABI calls.
- Native APKG manifest inspection and on-demand media extraction are exposed
  through the C ABI as JSON helpers, keeping ZIP/media-map behavior out of
  target-specific shells.
- Native APKG export is exposed through `eg_export_anki_apkg`, returning a JSON
  byte array for the generated legacy/V11 package.

## Workstream 3: Engram Web App

### 3.0 Lattice Shell Styling

The current web app still uses a React shell, but app-owned styling should be
authored in Lattice while the Mosaic component surface comes online.

Status:

- `engram-app` installs `src/styles/app.lattice` through the Lattice
  transpiler at startup.
- The old app-owned `src/styles/app.css` file has been retired.
- Shared `@coding-adventures/ui-components` styles remain CSS imports for now;
  those are compatibility dependencies until their visual surface is replaced
  by Mosaic/Lattice-native components.

### 3.1 Rust Core Integration

- Keep the TypeScript reducer as a compatibility shell at first.
- Add a JS loader for the WASM facade.
- Add parity tests comparing TypeScript and Rust outputs during migration.
- Gradually route scheduling, queueing, and stats to Rust.

### 3.2 PWA and Offline

- Add manifest and service worker.
- Ensure all study flows work offline.
- Add explicit backup/export reminders.
- Add restore/import flow.

### 3.3 Anki-Like Collection Browser

- Deck list.
- Card/note table.
- Search box.
- Field filters.
- Tags.
- Bulk suspend, unsuspend, delete, move, and tag actions.

### 3.4 Editor

- Note type editor.
- Field editor.
- Template editor with preview.
- Card generation preview.
- Markdown or rich text strategy.
- Media attachment support.

### 3.5 Review UX

- Keyboard shortcuts.
- Touch-friendly rating controls.
- Undo last review. Core and JSON facade support exists; web/native controls
  still need to bind to it.
- Bury card/note. Card-level core commands exist; note-level sibling behavior
  now has lineage-backed core and JSON facade support; web/native controls still
  need to bind to it.
- Suspend card/note. Card-level core commands exist; note-level bulk behavior
  remains a browser/editor workflow.
- Flag/mark card. Core and JSON facade support exists; web/native controls and
  browser filters still need to bind to it.
- Review remaining counts. Core, JSON facade, and C ABI support exists via
  shared active-session progress counters; web/native controls still need to
  render it.

## Workstream 4: Language Learning App Foundation

Engram is the memory engine. A broader language app should use a sibling
`language-core`.

Initial entities:

- language
- script
- grapheme
- phoneme
- lexeme
- gloss
- grammar concept
- etymology edge
- cognate edge
- lesson node
- exercise
- review binding into Engram

Early language focus:

- Tamil
- Hindi
- Kannada
- Malayalam
- Telugu
- Spanish with Latin/English/French cognate stories

Status:

- `code/packages/rust/language-core` adds the first Rust-owned language learning
  model for languages, scripts, graphemes, phonemes, lexemes, etymology links,
  lesson nodes, exercises, and Engram review bindings.
- Initial helpers cover etymology paths, shared-ancestor story candidates, and
  lesson-to-Engram card bindings.

## Workstream 5: Mosaic and Native

### 5.1 Mosaic Component Pilot

Start with small components:

- review card
- rating controls
- session progress
- deck stats panel

Emit React first, then validate one native target.

Status:

- `code/packages/mosaic-pkg-deck-stats` adds reusable total, new, due,
  learning, and hidden deck counters with label/value slots ready to bind to
  shared Engram core deck-stat JSON.
- `code/packages/mosaic-pkg-review-card` adds the first reusable
  `ReviewCard` component package. It now composes
  `mosaic-pkg-rating-controls` instead of owning the rating button row.
- `code/packages/mosaic-pkg-rating-controls` adds reusable
  Again/Hard/Good/Easy answer grading controls with label slots and review
  events.
- `code/packages/mosaic-pkg-session-progress` adds reusable current,
  remaining, correct, and total review-session counters with label/value slots
  ready to bind to shared Engram core facade JSON.
- Mosaic package artifact builds now inline nested package layouts and merge
  dependency package styles before backend emission. Dependency styles apply
  first and parent/app styles apply last, preserving reusable defaults while
  allowing deliberate overrides.
- The component package tests compile the same `.mil/.mll/.msl` sources
  through React, HTML, SwiftUI, XAML, Qt, Compose, and Flutter paths, and the
  app package test verifies the `EngramApp -> DeckStatsPanel`,
  `EngramApp -> SessionProgress`, and `EngramApp -> ReviewCard ->
  RatingControls` chains through HTML, React, SwiftUI, XAML, Qt, and Flutter
  artifacts.
- The Engram app smoke test now compares `EngramSession::engram_app_props`
  keys against the compiled `EngramApp.mil` slots, so shared core/facade
  bindings fail fast when the Mosaic app interface changes.
- `mosaic-package-artifact-builder` now emits XAML project-shell side files
  through the same package build path as React/HTML/Qt/SwiftUI/etc., and the
  Engram app smoke test verifies native project shells for Qt, SwiftUI, and
  XAML from the same Mosaic app sources.
- `code/programs/mosaic/engram-app` adds the Engram Mosaic app package. The
  app exports `EngramApp`, declares dependencies on `mosaic-pkg-deck-stats`,
  `mosaic-pkg-review-card`, and `mosaic-pkg-session-progress`, and mounts
  package components rather than owning deck-stat, review-card, or
  progress-counter component implementations itself.
- Shared `SessionProgress` counters are available in `engram-core`,
  `engram-core-wasm`, and `engram-capi` for Mosaic/native review screens.
- This split is the first concrete pivot point for moving Engram UI out of
  one-off React components and into Mosaic while keeping component packages
  reusable.

### 5.2 Native Target Order

Suggested order:

1. HTML/React, because Engram already runs there and Electron can wrap it.
2. Qt and XAML, because current Mosaic tests show practical desktop backend
   coverage.
3. SwiftUI for macOS and iOS, because Apple-native support is a first-class
   target.
4. Flutter/Compose as additional mobile/runtime validation paths.

Each target should consume generated components and the same `engram-core`
facade contract. If a target needs special behavior, add the capability to the
shared contract first, then let each shell bind to it.

## Initial Autonomous Loop

### Commit A: Foundation

- Engram app build dependency fix.
- Mobile layout fix.
- `engram-core` foundation crate.
- Rust tests passing with MSVC linker override.
- This plan.

### Commit B: Core Notes and Templates

- Add note models and template rendering to `engram-core`.
- Add tests for multi-card note generation.

### Commit C: Core Scheduler Step Toward Anki

- Add learning/review card states.
- Add deck options for learning steps and daily limits.
- Add deterministic scheduler tests.

### Commit D: Web App Import/Export

- Add JSON export/import.
- Add CSV import/export if scope stays controlled.
- Add tests for persistence and restore.

### Commit E: Review UX Improvements

- Add undo last review.
- Add bury/suspend model hooks.
- Add keyboard shortcuts.

### Commit F: PR and CI Stabilization

- Push branch.
- Open draft PR.
- Monitor CI.
- Fix local or CI issues until green.

## Current Local Build Recipe on Windows

The repo currently configures Windows Rust builds to use `lld-link`, but this
machine can reliably use MSVC's `link.exe` through the Visual Studio developer
environment:

```bat
"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64
set CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=link.exe
cargo test -p engram-core
```
