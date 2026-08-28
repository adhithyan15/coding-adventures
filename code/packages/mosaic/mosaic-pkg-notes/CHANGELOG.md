# Changelog

All notable changes to `mosaic-pkg-notes` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## [Unreleased]

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
