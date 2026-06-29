// CollectionActions.mll - target-neutral collection workflow layout.

layout CollectionActions {
  Column [ collection-actions ] {
    Text [ collection-title ] (
      content : slot: collection-label
    )
    Row [ collection-counts ] {
      Box [ note-count ] {
        Text [ note-count-value ] (
          content : slot: note-count-value
        )
        Text [ note-count-label ] (
          content : slot: note-count-label
        )
      }
      Box [ note-type-count ] {
        Text [ note-type-count-value ] (
          content : slot: note-type-count-value
        )
        Text [ note-type-count-label ] (
          content : slot: note-type-count-label
        )
      }
      Box [ media-count ] {
        Text [ media-count-value ] (
          content : slot: media-count-value
        )
        Text [ media-count-label ] (
          content : slot: media-count-label
        )
      }
    }
    Row [ import-export-actions ] {
      HostButton [ import-button ] (
        label : slot: import-label ,
        onClick : emit: onImportAnki
      )
      HostButton [ export-button ] (
        label : slot: export-label ,
        onClick : emit: onExportAnki
      )
    }
    Row [ note-actions ] {
      HostButton [ add-note-button ] (
        label : slot: add-note-label ,
        onClick : emit: onAddNote
      )
      HostButton [ add-note-type-button ] (
        label : slot: add-note-type-label ,
        onClick : emit: onAddNoteType
      )
    }
    Row [ destructive-actions ] {
      HostButton [ delete-note-button ] (
        label : slot: delete-note-label ,
        onClick : emit: onDeleteNote
      )
      HostButton [ delete-note-type-button ] (
        label : slot: delete-note-type-label ,
        onClick : emit: onDeleteNoteType
      )
    }
  }
}
