### Changed — NoteEditor 0.2.0: the focused field is a body-only DraftEditor (E4)

`mosaic-package.toml` requires `mosaic-pkg-note-editor = "0.2.0"`
(`mosaic-editor-convergence-audit-v1.md` §8). Neither layout changes:
NoteEditor's slots and events are the same, so `EngramApp.mll` and
`EngramApp.touch.mll` bind exactly what they bound before.

What changes on the Add/Edit screen:

- The focused field is multi-line, and Enter is a line break.
- Ctrl+Enter (Cmd+Enter on macOS) raises `onNoteEditorSaveNote`, the event
  the Save note button raises. That is Anki's Add-dialog shortcut.
- The field is named by its visible label, and the tags field by "Tags".
- The field draws inside DraftEditor's frame, as Journal's editor body does.

Tests:

- `native_complete_gate.rs` gains `ALLOWED_PART_STYLE_DROPS`, pins by
  `(backend, property, part)` checked in both directions. It holds the two
  nested-DraftEditor drops that the property list does not pin: SwiftUI `gap`
  on `draft-editor-title-group`, and Qt `min-height` on
  `draft-editor-body`. Pinning them by property would re-license SwiftUI
  `gap` everywhere in Engram, which the property list removed on purpose.
  Flutter `min-height` was already pinned by property. Measured with
  `theme: None`: six more reported drops, the three new ones once per
  layout. (Flutter `width` on `draft-editor-body` also dropped until
  #18467.) Capability degradations stay at zero.
- `package_compiles.rs` expects the 0.2.0 requirement, and checks on the
  HTML build that the focused field is a `<textarea>` whose
  `data-on-commit` is `onNoteEditorSaveNote`.

A search of every conformance tree and native host found no Engram UI test
that types into the focused field by its part tag (audit R3), so none
changes.
