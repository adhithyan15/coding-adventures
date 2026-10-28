# mosaic-pkg-notes

> List-plus-editor view over task-core's `Note` entity (create/edit/delete).

A two-column shell: a note list beside an editor for the open note. Since
0.3.0 it is a composition of two shared components — the toolkit's
`RecordList` draws the list, and `DraftEditor` draws the title, the body and
the Save/Delete/Cancel actions — plus the parts only Trestle needs: a
"+ New note" button, an attach-to-task field and an empty-state hint. See
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
          │  Notes → HostButton (+ New note),         │
          │          Text + HostInput (attach to task)│
          └───────┬──────────────────────┬────────────┘
                  │ pkg::                │ pkg::
                  ▼                      ▼
   ┌──────────────────────────┐ ┌──────────────────────────────┐
   │ mosaic-pkg-toolkit       │ │ mosaic-pkg-draft-editor      │
   │ RecordList (the list)    │ │ DraftEditor (title, body,    │
   │                          │ │ Save / Delete / Cancel)      │
   └──────────────────────────┘ └──────────────────────────────┘
```

Both are declared in `mosaic-package.toml`'s `[dependencies]`. Their parts
(`record-list-*`, `draft-editor-*`) are styled by their own packages'
themes; `Notes.{light,dark}.msl` style only the parts Notes owns.

## Fat engine, dumb UI

Notes does no persistence, filtering, or id-minting of its own. The host
builds `note-rows` from task-core's `Note` entities (read via
`workspace()` — no dedicated query needed, the whole-project dump already
includes `notes`), owns the in-progress edit buffer
(`title-value`/`body-value`), and decides what Save/Delete/Cancel mean as
engine ops (`upsertNote`/`deleteNote`).

## The note body

The note body is DraftEditor's `draft-editor-body`, the kernel's text area
`HostInput ( multiline : true )`. Enter is a line break, and Ctrl+Enter
(Cmd+Enter on macOS) raises DraftEditor's `onSave`, which Notes forwards to
its own `onSave`, as the Save button does (UI25 §4).

## Labels are the host's

Notes draws no English of its own. Every label is a slot with no default,
so the host binds each one (an unbound slot is a compile error):
`new-note-label`, `empty-hint`, `title-label`, `title-placeholder`,
`body-label`, `body-placeholder`, `task-label`, `task-placeholder`,
`save-label`, `delete-label`, `cancel-label`. A field's visible label is
also its accessible name, the attach-to-task field included. An empty
action label removes that button, so a host passes `delete-label: ""` for
a new note that was never saved.

## Usage

```moslayout
// In a host component's .mll:
pkg::mosaic-pkg-notes::Notes (
  notes-title:       slot: notes-title ,
  note-rows:         slot: note-rows ,       // [key, heading, title, subtitle, meta, badge]
  selected-note-id:  slot: selected-note-id ,
  title-value:       slot: note-title-value ,
  body-value:        slot: note-body-value ,
  task-name-value:   slot: note-task-value ,
  new-note-label:    "+ New note" ,
  empty-hint:        "Select a note, or start a new one." ,
  title-label:       "Title" ,
  title-placeholder: "Untitled" ,
  body-label:        "Note" ,
  body-placeholder:  "Write something…" ,
  task-label:        "Attach to task" ,
  task-placeholder:  "Task name (optional)" ,
  save-label:        "Save" ,
  delete-label:      slot: note-delete-label , // "" for an unsaved note
  cancel-label:      "Cancel" ,
  onSelectNote:     emit: onSelectNote ,
  onNewNote:        emit: onNewNote ,
  onTitleChange:    emit: onNoteTitleChange ,
  onBodyChange:     emit: onNoteBodyChange ,
  onTaskNameChange: emit: onNoteTaskNameChange ,
  onSave:           emit: onSaveNote ,
  onDelete:         emit: onDeleteNote ,
  onCancel:         emit: onCancelNote
)
```

`task-name-value`/`onTaskNameChange` are the v0.2.0 "attach to task"
field — a task NAME (not id), resolved by the host to `upsertNote`'s
`attachedTask` on Save. An unrecognised name rejects the whole save,
mirroring the Sheet Labels column's discipline. See
[task-app-notes-ui-v1.md](../../../specs/task-app-notes-ui-v1.md)'s
addendum.

`note-rows` rows are the toolkit RecordList's six fields. Trestle's hosts
fill the key (the note id) and the title (or "Untitled") and leave the
heading, subtitle, meta and badge empty, so each row is one line.

The host mints a new note's id up front (on "+ New note", not on Save —
see `selectedNoteId`'s own comment in the web host's `controller.ts`),
builds `note-rows` from `engine.workspace()`, and calls
`upsertNote`/`deleteNote` in response to Save/Delete. The reference hosts
are `code/programs/mosaic/task-app/host/web/src/controller.ts` (`noteRows()`
and the `saveNote`/`deleteNote`/`selectNote` cases) and the native adapter
`code/packages/rust/task-mosaic-app`.

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-notes
cargo test   # compile pins, plus the native gate (both themes)
```

`tests/package_compiles.rs`: the manifest parses, declares the expected
export and declares every package `Notes.mll` mounts; `Notes.mil` compiles
via `mosmodel-compiler`; `Notes.mll` compiles against that interface via
`moslayout-compiler` and draws no literal English; both themes' `.msl`
compile against the resulting part map. A few tests build the package,
because only a build inlines RecordList and DraftEditor: the selection
compares row keys with `selectedNoteId` (a kebab-case slot inside an
expression is silently subtraction — a real bug found live-testing 0.1.0),
the body is a multi-line field that saves on Ctrl/Cmd+Enter, every field is
named by its visible label, and Delete is drawn only when labelled.

`tests/native_complete_gate.rs` is the native gate, modelled on
`mosaic-pkg-draft-editor`'s: zero capability degradations on the five
native backends in both themes, and the package's known style drops pinned
in both directions — a new drop fails, and so does a pinned drop that stops
happening. Today that is 10 drops per theme (SwiftUI 3, Qt 4, XAML 1,
Flutter 2), pinned per part so a later change shows exactly which it fixed:
0.2.0 had 12, and 0.3.0's move to RecordList removed Qt's two `text-align`
drops on the old row buttons.

## Stories

`src/Notes.stories.json` gives MosaicBook five populated states: several
notes with one open, a long body, a new unsaved note, notes with none
selected, and no notes yet.

## License

MIT OR Apache-2.0.
