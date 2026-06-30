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
    Row [ media-health-counts ] {
      Box [ referenced-media-count ] {
        Text [ referenced-media-value ] (
          content : slot: referenced-media-value
        )
        Text [ referenced-media-label ] (
          content : slot: referenced-media-label
        )
      }
      Box [ missing-media-count ] {
        Text [ missing-media-value ] (
          content : slot: missing-media-value
        )
        Text [ missing-media-label ] (
          content : slot: missing-media-label
        )
      }
      Box [ unused-media-count ] {
        Text [ unused-media-value ] (
          content : slot: unused-media-value
        )
        Text [ unused-media-label ] (
          content : slot: unused-media-label
        )
      }
    }
    Column [ media-missing-list ] {
      For ( each: slot: missing-media-filenames , as: missing-media-filename , index: i ) {
        Text [ missing-media-item ] (
          content : missing-media-filename
        )
      }
    }
    Column [ media-unused-list ] {
      For ( each: slot: unused-media-asset-ids , as: unused-media-asset-id , index: i ) {
        Text [ unused-media-item ] (
          content : unused-media-asset-id
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
      HostButton [ prune-unused-media-button ] (
        label : slot: prune-unused-media-label ,
        onClick : emit: onPruneUnusedMedia
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
