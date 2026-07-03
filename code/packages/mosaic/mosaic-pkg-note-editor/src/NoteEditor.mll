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
    Row [ note-choice-row ] {
      Column [ note-editor-note-type-list-column ] {
        Text [ note-editor-note-type-options-label ] (
          content : slot: note-type-options-label
        )
        Column [ note-editor-note-type-list ] {
          For ( each: slot: note-type-names , as: note-type , index: note-type-index ) {
            HostButton [ note-editor-note-type-option ] (
              label : note-type ,
              onClick : emit: onSelectNoteType
            )
          }
        }
      }
      Column [ note-editor-deck-list-column ] {
        Text [ note-editor-deck-options-label ] (
          content : slot: deck-options-label
        )
        Column [ note-editor-deck-list ] {
          For ( each: slot: deck-names , as: deck , index: deck-index ) {
            HostButton [ note-editor-deck-option ] (
              label : deck ,
              onClick : emit: onSelectDeck
            )
          }
        }
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
