# mosaic-pkg-draft-editor

One plain editing surface: an optional one-line **title**, a multi-line
**body**, and labelled **actions**. It is shared by Journal's entry editor, Trestle's
notes, and (later) Engram's focused note field. J3b of
[#14416](https://github.com/adhithyan15/coding-adventures/issues/14416); design
in [`code/specs/mosaic-pkg-draft-editor.md`](../../../specs/mosaic-pkg-draft-editor.md).

```
component DraftEditor {
  slot title-label, title-value, title-placeholder : text ;  // "" title-label = no title
  slot body-label, body-value                      : text ;  // body-label required
  slot save-label, delete-label, cancel-label      : text ;  // "" = action not offered
  emit onTitleChange ( value : text ) ;  emit onBodyChange ( value : text ) ;
  emit onSave ;  emit onDelete ;  emit onCancel ;
}
```

## How it fits

- **Controlled.** Every keystroke emits the whole new text. The host owns the
  draft and Save commits it, which is how all three products already work.
- **Every label is a slot**, and it is also the field's accessible name. An
  empty label removes that part, so nothing unnamed is ever drawn.
- **No preview inside.** Journal composes its markdown preview beside this
  component. The preview is a `node` slot with real costs that Trestle and
  Engram should not inherit.
- **Ctrl/Cmd+Enter saves.** The body is the kernel's text area,
  `HostInput ( multiline : true )`: Enter is a line break, and Ctrl+Enter
  (Cmd+Enter on macOS) raises `onSave`, as the Save button does (UI25 §4). The
  chord raises `onSave` even when no Save is offered; such a host ignores it.
- **Its own package, not the toolkit.** It was written when the only portable
  text area was the legacy `Input ( multiline : true )`, which inside the
  toolkit is rewritten to the toolkit's single-line component.

## Usage

```
pkg::mosaic-pkg-draft-editor::DraftEditor (
  title-label : "Title" , title-value : slot: draft-title , title-placeholder : "Untitled" ,
  body-label : "Entry" , body-value : slot: draft-body ,
  save-label : "Save" , delete-label : "" , cancel-label : "Cancel" ,
  onTitleChange : emit: onDraftTitle , onBodyChange : emit: onDraftBody ,
  onSave : emit: onSaveEntry , onCancel : emit: onCloseEditor
)
```

## Testing

```sh
cargo test   # interface + layout pins, builds on all 8 backends, native gate (both themes)
```
