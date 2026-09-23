// DraftEditor.mll — layout.
//
//   Column [ draft-editor ]
//     If (title-label)  Column [ draft-editor-title-group ]
//                         Text      [ draft-editor-title-label ]
//                         HostInput [ draft-editor-title ]
//     Text  [ draft-editor-body-label ]
//     Input [ draft-editor-body ]   (multiline)
//     Row   [ draft-editor-actions ]
//       If (save-label)   HostButton [ draft-editor-save ]
//       If (delete-label) HostButton [ draft-editor-delete ]
//       If (cancel-label) HostButton [ draft-editor-cancel ]
//
// The body is the UI25 legacy `Input ( multiline : true )` — the one
// portable text area. It is reachable here because this package depends on
// nothing: inside the toolkit, a bare `Input` is rewritten to the toolkit's
// single-line `Input` component (see the spec). J3b-pre (#15931) made it a
// real, named text area on every backend.
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
    Input [ draft-editor-body ] (
      value : slot: body-value ,
      multiline : true ,
      a11y-label : slot: body-label ,
      onChange : emit: onBodyChange
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
