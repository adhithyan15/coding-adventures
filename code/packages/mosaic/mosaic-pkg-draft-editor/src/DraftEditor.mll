// DraftEditor.mll — layout.
//
//   Column [ draft-editor ]
//     If (title-label)  Column [ draft-editor-title-group ]
//                         Text      [ draft-editor-title-label ]
//                         HostInput [ draft-editor-title ]
//     Text  [ draft-editor-body-label ]
//     HostInput [ draft-editor-body ]   (multiline; Ctrl/Cmd+Enter saves)
//     Row   [ draft-editor-actions ]
//       If (save-label)   HostButton [ draft-editor-save ]
//       If (delete-label) HostButton [ draft-editor-delete ]
//       If (cancel-label) HostButton [ draft-editor-cancel ]
//
// The body is the kernel's text area, `HostInput ( multiline : true )`
// (#15930): Enter is a line break, and Ctrl+Enter (Cmd+Enter on macOS)
// raises `onSave`, the same event as the Save button (UI25 §4, UI29 §2.1).
// It used to be the UI25 legacy `Input`, which now lowers identically and is
// on its way out (U29-X1).
//
// The chord raises `onSave` even when `save-label` is empty and no Save
// button is drawn: a layout cannot bind a field's commit conditionally
// without re-creating the field. A host that offers no Save ignores
// `onSave`, as it already ignores every event it did not wire.
//
// The visible label and the accessible name come from the same slot, so a
// screen reader announces exactly what a sighted user reads.

layout DraftEditor {
  Column [ draft-editor ] {
    If ( when: slot: title-label ) {
      Column [ draft-editor-title-group ] {
        Text [ draft-editor-title-label ] (
          content : slot: title-label
        )
        HostInput [ draft-editor-title ] (
          value : slot: title-value ,
          placeholder : slot: title-placeholder ,
          a11y-label : slot: title-label ,
          disabled : false ,
          onChange : emit: onTitleChange
        )
      }
    }
    Text [ draft-editor-body-label ] (
      content : slot: body-label
    )
    HostInput [ draft-editor-body ] (
      value : slot: body-value ,
      multiline : true ,
      a11y-label : slot: body-label ,
      onChange : emit: onBodyChange ,
      onCommit : emit: onSave
    )
    Row [ draft-editor-actions ] {
      If ( when: slot: save-label ) {
        HostButton [ draft-editor-save ] (
          label : slot: save-label ,
          onClick : emit: onSave
        )
      }
      If ( when: slot: delete-label ) {
        HostButton [ draft-editor-delete ] (
          label : slot: delete-label ,
          onClick : emit: onDelete
        )
      }
      If ( when: slot: cancel-label ) {
        HostButton [ draft-editor-cancel ] (
          label : slot: cancel-label ,
          onClick : emit: onCancel
        )
      }
    }
  }
}
