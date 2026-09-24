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
      // Search (J4a): a query turns the list below into ranked results.
      // Clear appears only while there is something to clear.
      Row [ search-bar ] {
        HostInput [ search-input ] (
          value : slot: search-query ,
          placeholder : "Search" ,
          a11y-label : "Search entries" ,
          disabled : false ,
          onChange : emit: onSearchChange
        )
        If ( when: slot: searching ) {
          HostButton [ search-clear ] (
            label : "Clear" ,
            onClick : emit: onClearSearch
          )
        }
      }
      // Stars (J4b): the filter is a toggle button. Two parts, so the "on"
      // state wears its own style on every backend (as Trestle's outline).
      If ( when: slot: starred-only ) {
        HostButton [ starred-filter-on ] (
          label : "Starred only" ,
          selected : true ,
          onClick : emit: onToggleStarredFilter
        )
      }
      Else {
        HostButton [ starred-filter ] (
          label : "Starred only" ,
          selected : false ,
          onClick : emit: onToggleStarredFilter
        )
      }
      // On this day (J4c): a second RecordList mount (#15959), above the
      // timeline, only when earlier years have entries on today's date.
      If ( when: slot: has-on-this-day ) {
        Text [ on-this-day-title ] (
          content : "On this day" ,
          a11y-role : heading
        )
        pkg::mosaic-pkg-toolkit::RecordList (
          rows : slot: on-this-day-rows ,
          selected-key : slot: selected-key ,
          onSelect : emit: onSelectOnThisDay
        )
      }
      If ( when: slot: timeline-empty ) {
        pkg::mosaic-pkg-toolkit::EmptyState (
          title : "No entries yet" ,
          message : "Write your first entry, then save it. It is filed under today." ,
          action-label : ""
        )
      }
      Else {
        // A second EmptyState mount (#15959): the journal has entries, but
        // none matches every word of the query.
        If ( when: slot: no-matches ) {
          pkg::mosaic-pkg-toolkit::EmptyState (
            title : "No entries match" ,
            message : "Every word must appear in an entry. Try fewer words, or Clear." ,
            action-label : ""
          )
        }
        Else {
          // A third EmptyState mount: the filter is on and nothing is starred.
          If ( when: slot: no-starred ) {
            pkg::mosaic-pkg-toolkit::EmptyState (
              title : "No starred entries" ,
              message : "Star an entry to keep it here." ,
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
      }
    }
    Column [ editor ] {
      // Stars the entry in the editor; hidden for a new draft.
      If ( when: slot: star-label ) {
        HostButton [ star-toggle ] (
          label : slot: star-label ,
          onClick : emit: onToggleStar
        )
      }
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
