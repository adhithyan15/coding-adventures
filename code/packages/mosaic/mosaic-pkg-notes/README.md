# mosaic-pkg-notes

> List-plus-editor view over task-core's `Note` entity (create/edit/delete).

A two-column shell adapted from `mosaic-pkg-note-editor` — built for a
different domain (Anki-style flashcard notes: note types, decks, focused-
field editing) — re-pointed at a plain title+body note. See
[task-app-notes-ui-v1.md](../../../specs/task-app-notes-ui-v1.md) for the
full scope and design rationale.

## What this package exports

One component, per `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Notes` | note list + editor | `Notes.mil` / `Notes.mll` / `Notes.{light,dark}.msl` |

## How it fits in the stack

```
          ┌──────────────────────────────────────────┐
          │  Host application (task-app's Notes view) │
          └─────────────────────┬──────────────────────┘
                                │ component reference
                                ▼
          ┌──────────────────────────────────────────┐
          │  mosaic-pkg-notes (this package)          │
          │  Notes → Column/Row/Text/HostButton/      │
          │          HostInput/Input(multiline)       │
          └──────────────────────────────────────────┘
```

## Fat engine, dumb UI

Notes does no persistence, filtering, or id-minting of its own. The host
builds `note-rows` from task-core's `Note` entities (read via
`workspace()` — no dedicated query needed, the whole-project dump already
includes `notes`), owns the in-progress edit buffer
(`title-value`/`body-value`), and decides what Save/Delete/Cancel mean as
engine ops (`upsertNote`/`deleteNote`).

## A deliberate legacy-primitive use

UI29's kernel `HostInput` is single-line only — multiline support lives
only on the legacy `Input` (UI25) primitive, and was never carried
forward into a userland `MultilineInput` component (which doesn't exist
yet). The note body genuinely needs multiple lines, so
`notes-body-input` uses `Input ( multiline: true )` — the one such use in
this package, disclosed rather than hidden. See the spec's "A real kernel
gap found while designing this" section.

## Usage

```moslayout
// In a host component's .mll:
pkg::mosaic-pkg-notes::Notes (
  notes-title:      slot: notes-title ,
  note-rows:        slot: note-rows ,
  selected-note-id: slot: selected-note-id ,
  title-value:      slot: note-title-value ,
  body-value:       slot: note-body-value ,
  onSelectNote:  emit: onSelectNote ,
  onNewNote:     emit: onNewNote ,
  onTitleChange: emit: onNoteTitleChange ,
  onBodyChange:  emit: onNoteBodyChange ,
  onSave:        emit: onSaveNote ,
  onDelete:      emit: onDeleteNote ,
  onCancel:      emit: onCancelNote
)
```

The host mints a new note's id up front (on "+ New note", not on Save —
see `selectedNoteId`'s own comment in `main.tsx`), builds `note-rows` from
`engine.workspace()`, and calls `upsertNote`/`deleteNote` in response to
Save/Delete — see `code/programs/mosaic/task-app/host/web/src/main.tsx`
(`noteRows()` and the `saveNote`/`deleteNote`/`selectNote` dispatch cases)
for the reference host consumer.

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-notes
cargo test
```

Mirrors `mosaic-pkg-calendar`'s own smoke test: manifest parses and
declares the expected export; `Notes.mil` compiles via `mosmodel-compiler`;
`Notes.mll` compiles against that interface via `moslayout-compiler`
(with explicit pins that it actually uses a multiline body field, and
that the selected-note-id slot is referenced by its camelCase identifier
inside expressions — a real bug found live-testing this package, not a
hypothetical, see the test's own comment); both themes' `.msl` compile
against the resulting part map.

## License

MIT OR Apache-2.0.
