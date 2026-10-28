# mosaic-pkg-draft-editor

One plain editing surface: an optional one-line **title**, a multi-line
**body**, and labelled **actions**. Journal's entry editor is its one consumer
today; Trestle's notes and Engram's focused note field are planned to adopt it
([`mosaic-editor-convergence-audit-v1.md`](../../../specs/mosaic-editor-convergence-audit-v1.md),
E3 and E4). J3b of
[#14416](https://github.com/adhithyan15/coding-adventures/issues/14416); design
in [`code/specs/mosaic-pkg-draft-editor.md`](../../../specs/mosaic-pkg-draft-editor.md).

```
component DraftEditor {
  slot title-label, title-value, title-placeholder : text ;  // "" title-label = no title
  slot body-label, body-value, body-placeholder    : text ;  // body-label required
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
- **The body placeholder is a hint, not a name** (0.2.0). `body-placeholder`
  shows only while the body is empty; `""` shows none. The visible
  `body-label` stays the field's name. Since EM-13 a slot-bound placeholder
  lowers on every backend.
- **Adding a slot breaks consumers.** Every slot here is required: a consumer
  that does not bind one fails with `UnknownSlot`. That is why 0.2.0 is a
  breaking version and Journal binds `body-placeholder : ""`.
- **No preview inside.** A markdown preview is a `node` slot with real costs
  that a plain editor should not carry; a product that wants one composes it
  beside this component.
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
  body-label : "Entry" , body-value : slot: draft-body , body-placeholder : "" ,
  save-label : "Save" , delete-label : "" , cancel-label : "Cancel" ,
  onTitleChange : emit: onDraftTitle , onBodyChange : emit: onDraftBody ,
  onSave : emit: onSaveEntry , onCancel : emit: onCloseEditor
)
```

## Testing

```sh
cargo test   # interface + layout + stories pins, builds on every backend, native gate (both themes)
```
