// NoteEditor.mll - target-neutral focused note editor layout.

layout NoteEditor {
  Column [ note-editor ] {
    Text [ editor-title ] (
      content : slot: editor-label
    )
    Row [ note-metadata ] {
      Box [ note-id-meta ] {
        Text [ note-id-label ] (
          content : slot: note-id-label
        )
        Text [ note-id-value ] (
          content : slot: note-id-value
        )
      }
      Box [ note-type-meta ] {
        Text [ note-type-label ] (
          content : slot: note-type-label
        )
        Text [ note-type-value ] (
          content : slot: note-type-value
        )
      }
      Box [ deck-meta ] {
        Text [ deck-label ] (
          content : slot: deck-label
        )
        Text [ deck-value ] (
          content : slot: deck-value
        )
      }
    }
    Row [ editor-body ] {
      Column [ field-list-column ] {
        Text [ fields-label ] (
          content : slot: fields-label
        )
        Column [ note-field-list ] {
          For ( each: slot: field-labels , as: field , index: field-index ) {
            HostButton [ note-field-option ] (
              label : field ,
              onClick : emit: onSelectField
            )
          }
        }
      }
      Column [ focused-field-column ] {
        Text [ selected-field-label ] (
          content : slot: selected-field-label
        )
        HostInput [ selected-field-input ] (
          value : slot: selected-field-value ,
          placeholder : slot: selected-field-placeholder ,
          disabled : false ,
          onChange : emit: onFieldValueChange
        )
        Text [ tags-label ] (
          content : slot: tags-label
        )
        HostInput [ tags-input ] (
          value : slot: tags-value ,
          placeholder : slot: tags-placeholder ,
          disabled : false ,
          onChange : emit: onTagsChange
        )
      }
    }
    Row [ editor-actions ] {
      HostButton [ save-button ] (
        label : slot: save-label ,
        onClick : emit: onSaveNote
      )
      HostButton [ delete-button ] (
        label : slot: delete-label ,
        onClick : emit: onDeleteNote
      )
      HostButton [ cancel-button ] (
        label : slot: cancel-label ,
        onClick : emit: onCancel
      )
    }
  }
}
