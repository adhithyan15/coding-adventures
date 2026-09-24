// JournalApp.mll — layout.
//
//   HostNavigationSplit [ app-shell ]          pane: the timeline; detail: the editor
//     Column [ timeline ]
//       HostButton [ new-entry ]
//       If (timeline-empty)  toolkit EmptyState
//       Else                 toolkit RecordList   (first product consumer)
//     Column [ editor ]
//       DraftEditor                            (title, body, Save/Delete/Cancel)
//
// Every label here is literal English for now; localisation moves them to slots
// when Journal gains a locale (the components already take them as slots).

layout JournalApp {
  HostNavigationSplit [ app-shell ] (
    pane-title : "Journal" ,
    pane-width : 300 ,
    collapse : auto
  ) {
    Column [ timeline ] {
      HostButton [ new-entry ] (
        label : "New entry" ,
        onClick : emit: onNewEntry
      )
      If ( when: slot: timeline-empty ) {
        pkg::mosaic-pkg-toolkit::EmptyState (
          title : "No entries yet" ,
          message : "Write your first entry, then save it. It is filed under today." ,
          action-label : ""
        )
      }
      Else {
        pkg::mosaic-pkg-toolkit::RecordList (
          rows : slot: timeline-rows ,
          selected-key : slot: selected-key ,
          onSelect : emit: onSelectEntry
        )
      }
    }
    Column [ editor ] {
      pkg::mosaic-pkg-draft-editor::DraftEditor (
        title-label : "Title" ,
        title-value : slot: draft-title ,
        title-placeholder : "Untitled" ,
        body-label : "Entry" ,
        body-value : slot: draft-body ,
        save-label : "Save" ,
        delete-label : slot: delete-label ,
        cancel-label : "Cancel" ,
        onTitleChange : emit: onTitleChange ,
        onBodyChange : emit: onBodyChange ,
        onSave : emit: onSaveEntry ,
        onDelete : emit: onDeleteEntry ,
        onCancel : emit: onCancelEdit
      )
    }
  }
}
