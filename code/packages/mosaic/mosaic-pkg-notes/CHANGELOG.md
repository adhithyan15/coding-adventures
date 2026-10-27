# Changelog

All notable changes to `mosaic-pkg-notes` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## [Unreleased]

## 0.3.0 — 2026-10-27 — composed from RecordList and DraftEditor (E3)

E3 of `mosaic-editor-convergence-audit-v1.md` (probe C);
[task-app-notes-ui-v1.md](../../../specs/task-app-notes-ui-v1.md)
addendum v1.3. This release also carries the unreleased 0.2.x entries
below it (the E2 gate, the `HostInput` body).

### Changed — BREAKING: Notes is a composition

- The note list is `pkg::mosaic-pkg-toolkit::RecordList`, and the title,
  body and Save/Delete/Cancel actions are
  `pkg::mosaic-pkg-draft-editor::DraftEditor` (0.2.0). Every slot of both
  is bound at Notes' call site. Both packages are now declared in
  `mosaic-package.toml`'s `[dependencies]` (`mosaic-pkg-toolkit = "0.16.0"`,
  `mosaic-pkg-draft-editor = "0.2.0"`), which used to be empty.
- **`note-rows` rows are RecordList's** `[key, heading, title, subtitle,
  meta, badge]`, passed through unchanged. 0.2.0 rows were `[id, title]`;
  a host moves the title to `[2]` and fills `[1]`, `[3]`, `[4]` and `[5]`
  with `""` to keep one-line rows.
- **Every label is a slot**, with no default: `new-note-label`,
  `empty-hint`, `title-label`, `title-placeholder`, `body-label`,
  `body-placeholder`, `task-label`, `task-placeholder`, `save-label`,
  `delete-label` and `cancel-label`. The package draws no English. A
  consumer that does not bind them no longer compiles.
- An empty action label removes that button (DraftEditor's rule). A host
  passes `delete-label: ""` for a new note that was never saved.
- The title and body gain DraftEditor's visible labels, which are also
  their accessible names.
- The attach-to-task field stays Notes' own `HostInput`, now above the
  editor, with a visible label (`notes-task-label`) that is also its
  `a11y-label`. It had no accessible name beyond its placeholder.
- Removed parts (now RecordList's and DraftEditor's): `notes-row-on`,
  `notes-row-off`, `notes-title-input`, `notes-body-input`,
  `notes-actions`, `notes-save`, `notes-delete`, `notes-cancel`. Added:
  `notes-task-label`. A host test that found the body by
  `notes-body-input` should match `draft-editor-body` by prefix (a later
  mount of DraftEditor in the same app would be `draft-editor-body-m2`).
- The body is still the multi-line `HostInput`, and Ctrl/Cmd+Enter still
  saves: DraftEditor's `onSave` is forwarded to Notes' `onSave`. The eight
  events and their payloads are unchanged.

### Changed — the native gate's pins shrink from 12 to 10 per theme

- Removed: Qt `text-align` on `notes-row-on` and `notes-row-off` (the row
  buttons are gone; RecordList's rows ask for no `text-align`).
- Moved: Qt and Flutter `min-height` from `notes-body-input` to
  DraftEditor's `draft-editor-body`, the drop DraftEditor's own gate pins.
- Kept: the list/editor flex split (SwiftUI 3, Qt 3, XAML 1, Flutter 1),
  which PR-6 (`HostNavigationSplit`) removes. Composing adds no new drop.

### Changed — tests, stories and documents

- `package_compiles.rs`: the layout pins now assert the composition, the
  declared dependencies, no literal English, and (on a React build, where
  the mounts are inlined) the key comparison against `selectedNoteId`, the
  Ctrl/Cmd+Enter save, the three accessible names and the label-gated
  Delete.
- The five stories use six-field rows and bind every label; "New unsaved
  note" passes `delete-label: ""`.
- Fixed stale comments in `mosaic-package.toml` and `src/lib.rs`, which
  still said the body was the legacy `Input` because the kernel
  `HostInput` was single-line; and in `Notes.mil`, which said a new
  note's id is minted on Save (both hosts mint it on "+ New note").
- The smoke crate's `Cargo.toml` version (0.1.0) now matches the package,
  0.3.0.

The entries below were unreleased under 0.2.0 and ship in 0.3.0.

### Added — a native-complete gate pinning today's 12 style drops (E2)

- `tests/native_complete_gate.rs`, modelled on `mosaic-pkg-draft-editor`'s:
  the five native backends in both themes, zero capability degradations
  with no allowlist, and style drops pinned in both directions (a pin that
  stops occurring fails too). Measured on the audit base, identical in both
  themes: 12 drops per theme — SwiftUI 3, Qt 6, XAML 1, Flutter 2, Compose
  0 — matching `mosaic-editor-convergence-audit-v1.md` §3.1. Each pin names
  the backend, property **and part**, so E3's move to `RecordList` and
  `DraftEditor` (probe C: 12 → 10) shows up as exactly the pins it deletes.
- MosaicBook: five populated stories replace the single empty `Default` —
  several notes with one open (and an `Untitled` row), a long body, a new
  unsaved note, notes with none selected, and no notes yet.
- The package's layout, styles and interface are unchanged, so the version
  stays 0.2.0.

### Changed — the note body is `HostInput` and saves on Ctrl/Cmd+Enter (UI25 §4)

- `notes-body-input` moved from the retiring legacy `Input` to
  `HostInput ( multiline : true )`, which lowers identically, with
  `onCommit : emit: onSave`: Enter is a line break and Ctrl+Enter (Cmd+Enter
  on macOS) saves, as the Save button does. The package test pins both.

### Changed

- Documentation only: the layout comment and README no longer say the kernel
  `HostInput` is single-line. It takes `multiline: true` on every backend
  since #15930, so `notes-body-input` can move off the legacy `Input` when it
  retires (U29-X1). The generated output is unchanged.

### Added

- `elevation: raised;` on the `notes-row-on` (selected note row) part
  in both themes, alongside its existing `box-shadow:` — additive, not
  a replacement. mosstyle's new `elevation` property (#12028 item 1,
  UI41) is the channel native backends will read to render their own
  native shadow primitive; no backend reads it yet.

## 0.2.0 — 2026-08-06 — minimal attach-to-task control

### Added

- `slot task-name-value : text ;` and `emit onTaskNameChange ( value :
  text ) ;` — a single-line "Attach to task" input in the editor, next
  to title/body. Notes has no notion of task ids; the field holds a
  task NAME and the host resolves it to an id on Save (mirroring the
  Sheet Labels column's discipline in `task-app`). Empty means "no
  attachment." See
  [task-app-notes-ui-v1.md](../../../specs/task-app-notes-ui-v1.md)'s
  addendum for the full scope decision — this is deliberately not a
  dropdown/autocomplete picker, just the minimal write path that makes
  `Note.attached_task` usable at all.

## 0.1.0 — 2026-08-06 — initial release

### Added

- `Notes` component: a list-plus-editor view, adapted from
  `mosaic-pkg-note-editor` (built for a different domain — Anki-style
  flashcard notes — with none of its note-type/deck/focused-field
  machinery carried over). Create, edit, and delete a note (title + plain
  text body) via `upsertNote`/`deleteNote`.
- Deliberate, disclosed use of the legacy `Input` primitive with
  `multiline: true` for the body field — UI29's kernel `HostInput` is
  single-line only, and no userland `MultilineInput` exists yet. See the
  package README's "A deliberate legacy-primitive use" section.
- Both themes, styled to match the rest of TaskApp's existing tokens (no
  design-mock reference exists for this component).

### Fixed (found live-testing, before first ship)

- **A slot referenced by its kebab-case name inside an expression
  silently miscompiled.** `( n[0] == selected-note-id )` compiles cleanly
  at every static layer (mosmodel/moslayout/mosstyle all accept it) but
  is wrong at runtime: the emitted JS parses `selected-note-id` as
  subtraction of three undefined identifiers (`selected - note - id`),
  not a reference to the `selected-note-id` slot. Fixed to the correct
  camelCase form (`selectedNoteId`), matching the convention
  `mosaic-pkg-grid`'s `Grid.mll` already established (`editRow`/`editCol`
  inside `is-editing`/`is-selected` expressions). Caught by live-testing
  in a browser — clicking Save threw `ReferenceError: selected is not
  defined` and blanked the page — not by the package's own compile-layer
  tests, which is exactly why a regression test pinning the camelCase
  form was added alongside the fix.

See [task-app-notes-ui-v1.md](../../../specs/task-app-notes-ui-v1.md) for
the full scope and what's deliberately deferred (attachment picker, tags,
rich text, search).
