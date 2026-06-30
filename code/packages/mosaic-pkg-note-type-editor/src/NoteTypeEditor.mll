// NoteTypeEditor.mll - target-neutral note-type editor layout.

layout NoteTypeEditor {
  Column [ note-type-editor ] {
    Text [ note-type-editor-title ] (
      content : slot: editor-label
    )
    Row [ note-type-editor-body ] {
      Column [ note-type-list-column ] {
        Text [ note-types-label ] (
          content : slot: note-types-label
        )
        Column [ note-type-list ] {
          For ( each: slot: note-type-names , as: note-type , index: note-type-index ) {
            HostButton [ note-type-option ] (
              label : note-type ,
              onClick : emit: onSelectNoteType
            )
          }
        }
      }
      Column [ note-type-detail-column ] {
        Row [ note-type-metadata ] {
          Box [ note-type-id-meta ] {
            Text [ note-type-id-label ] (
              content : slot: note-type-id-label
            )
            Text [ note-type-id-value ] (
              content : slot: note-type-id-value
            )
          }
        }
            Text [ note-type-name-label ] (
              content : slot: name-label
            )
        HostInput [ note-type-name-input ] (
          value : slot: name-value ,
          placeholder : slot: name-placeholder ,
          disabled : false ,
          onChange : emit: onNameChange
        )
        Row [ note-type-schema-summary ] {
          Column [ field-summary-column ] {
            Text [ note-type-fields-label ] (
              content : slot: fields-label
            )
            Column [ note-type-field-list ] {
              For ( each: slot: field-labels , as: field , index: field-index ) {
                HostButton [ note-type-field-option ] (
                  label : field ,
                  onClick : emit: onSelectField
                )
              }
            }
            Text [ note-type-field-name-label ] (
              content : slot: field-name-label
            )
            HostInput [ note-type-field-name-input ] (
              value : slot: field-name-value ,
              placeholder : slot: field-name-placeholder ,
              disabled : false ,
              onChange : emit: onFieldNameChange
            )
            HostCheckbox [ note-type-field-required-checkbox ] (
              label : slot: field-required-label ,
              checked : slot: field-required-value ,
              disabled : false ,
              indeterminate : false ,
              onToggle : emit: onFieldRequiredChange
            )
          }
          Column [ template-summary-column ] {
            Text [ note-type-templates-label ] (
              content : slot: templates-label
            )
            Column [ note-type-template-list ] {
              For ( each: slot: template-labels , as: template , index: template-index ) {
                Text [ note-type-template-label ] (
                  content : template
                )
              }
            }
          }
        }
        Text [ note-type-stylesheet-label ] (
          content : slot: stylesheet-label
        )
        HostInput [ note-type-stylesheet-input ] (
          value : slot: stylesheet-value ,
          placeholder : slot: stylesheet-placeholder ,
          disabled : false ,
          onChange : emit: onStylesheetChange
        )
      }
    }
    Row [ note-type-editor-actions ] {
      HostButton [ note-type-new-button ] (
        label : slot: new-label ,
        onClick : emit: onNewNoteType
      )
      HostButton [ note-type-save-button ] (
        label : slot: save-label ,
        onClick : emit: onSaveNoteType
      )
      HostButton [ note-type-delete-button ] (
        label : slot: delete-label ,
        onClick : emit: onDeleteNoteType
      )
      HostButton [ note-type-cancel-button ] (
        label : slot: cancel-label ,
        onClick : emit: onCancel
      )
    }
  }
}
