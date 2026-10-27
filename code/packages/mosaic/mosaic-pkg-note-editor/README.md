# mosaic-pkg-note-editor

Reusable Mosaic component for Engram-style note editing.

`NoteEditor` keeps note-editor UI out of the Engram app package. It exposes a
portable deck/model selector, focused-field workflow, tags editor, and
save/delete/cancel controls. Generated HTML, Electron/React, SwiftUI, Qt, XAML,
Flutter, and Compose shells can use the same component while
`engram-core-wasm` owns draft note state and materializes cards through the
shared Rust core.

## How it fits in the stack

`NoteEditor` is an Anki note **form** that contains an editor. The pickers,
the tags field and the action row are Engram's alone and live here. The one
text surface, the focused field, is the shared editor every Mosaic product
uses: since 0.2.0 it is a body-only
[`DraftEditor`](../mosaic-pkg-draft-editor/README.md), mounted with no title
and no actions (E4 of
[`mosaic-editor-convergence-audit-v1.md`](../../../specs/mosaic-editor-convergence-audit-v1.md),
§8).

```
Column [ note-editor ]
  Text [ editor-title ]
  Row  [ note-metadata ]        note id, note type, deck
  Row  [ note-choice-row ]      note-type and deck option lists
  Row  [ editor-body ]
    Column [ field-list-column ]            one button per field
    Box    [ focused-field-column ]
      pkg::mosaic-pkg-draft-editor::DraftEditor   the focused field
      Text      [ tags-label ]
      HostInput [ tags-input ]
  Row  [ editor-actions ]       Save note / Delete / Cancel
```

What that means for a user and a host:

- **The focused field is multi-line.** Enter is a line break, which is what an
  Anki field holds.
- **Ctrl+Enter (Cmd+Enter on macOS) saves the note.** DraftEditor raises its
  `onSave` on that chord, and `NoteEditor` forwards it to `onSaveNote`, the
  event its Save note button raises. That is Anki's own Add-dialog shortcut.
- **Each field is named by its visible label.** The focused field's
  accessible name is `selected-field-label`, and the tags field's is
  `tags-label`. So a host must never send `selected-field-label: ""`. Engram
  sends the field's name, or "Field" while no field is selected.
- **The package draws no English.** Every word is a slot the host binds. The
  `""` bindings on DraftEditor's title and action labels are how a body-only
  mount switches those parts off.

The slot and event sets did not change in 0.2.0, so a consumer's call site
does not change. The parts `selected-field-label` and `selected-field-input`
are now DraftEditor's `draft-editor-body-label` and `draft-editor-body`. A UI
test that finds the field by tag should match `draft-editor-body` by prefix,
because a later mount of DraftEditor in the same app would be suffixed
(`-m2`).

## Usage

```
pkg::mosaic-pkg-note-editor::NoteEditor (
  editor-label : slot: note-editor-label ,
  …
  selected-field-label : slot: note-editor-selected-field-label ,
  selected-field-value : slot: note-editor-selected-field-value ,
  selected-field-placeholder : slot: note-editor-selected-field-placeholder ,
  …
  onFieldValueChange : emit: onNoteEditorFieldValueChange ,
  onSaveNote : emit: onNoteEditorSaveNote ,
  …
)
```

`EngramApp.mll` and `EngramApp.touch.mll` bind all 25 slots and 8 events.
The four stories in `src/NoteEditor.stories.json` show adding a note, editing
a multi-line field, confirming a delete, and an empty collection with no field
to select.

## Native gate

`tests/native_complete_gate.rs` runs the real degradation analyser on the five
native backends in both themes. Capability degradations: zero. Style drops:
five per theme, each pinned by backend, property and part, and checked in both
directions. Four of them come from mounting DraftEditor here (SwiftUI `gap` on
the title group the mount folds away, Qt and Flutter `min-height` on the body,
Flutter `width` on the body under this package's Row). The gate's header says
why each is pinned rather than fixed.
