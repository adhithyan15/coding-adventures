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

## Architecture Direction

Engram follows the same pattern as the VisiCalc demos:

```text
React Engram app
Mosaic-generated UI
native shells
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
- Host apps pass timestamps and IDs into the core.
- No UI dependencies in `engram-core`.
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
- Renaming a field migrates template references where possible.
- Generated card IDs remain stable across harmless note edits.

### 1.3 Scheduler Parity Track

The current scheduler is a compact SM-2 variant. Anki compatibility requires
at least:

- new, learning, review, and relearning queues
- learning steps in minutes
- graduating interval
- easy interval
- lapse handling
- bury siblings until next day
- deck options
- daily limits
- review history log

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
- simple boolean operators

Tests:

- Query parser diagnostics.
- Search result stability.
- Tag and deck filters compose with text filters.

### 1.5 Import/Export Model

Core should expose portable snapshot types that facade crates can serialize.

Formats:

- Engram JSON collection snapshot
- CSV deck import/export
- Anki TSV text export compatibility
- APKG import/export eventually, via a dedicated facade or package crate

Tests:

- JSON snapshot round-trip.
- CSV escaping and line ending behavior.
- Import produces deterministic IDs when caller supplies an ID strategy.

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

## Workstream 3: Engram Web App

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
- Undo last review.
- Bury card/note.
- Suspend card/note.
- Flag/mark card.
- Review remaining counts.

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

## Workstream 5: Mosaic and Native

### 5.1 Mosaic Component Pilot

Start with small components:

- review card
- rating controls
- session progress
- deck stats panel

Emit React first, then validate one native target.

### 5.2 Native Target Order

Suggested order:

1. React, because Engram already runs there.
2. Flutter or Qt, because current Mosaic tests show practical backend coverage.
3. Compose/SwiftUI as the native-mobile strategic targets.
4. XAML after Windows shell needs are clearer.

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

