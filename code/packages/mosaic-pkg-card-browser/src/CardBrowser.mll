// CardBrowser.mll - target-neutral card browser layout.

layout CardBrowser {
  Column [ card-browser ] {
    Text [ browser-title ] (
      content : slot: browser-label
    )
    Row [ browser-search-row ] {
      Column [ query-column ] {
        Text [ query-label ] (
          content : slot: query-label
        )
        pkg::mosaic-pkg-toolkit::Input (
          value : slot: query ,
          placeholder : slot: query-placeholder ,
          disabled : false ,
          size : "md" ,
          onChange : emit: onQueryChange ,
          onCommit : emit: onSearch
        )
      }
      Column [ filter-column ] {
        Text [ filter-label ] (
          content : slot: filter-label
        )
        HostButton [ filter-toggle-button ] (
          label : slot: filter-value ,
          disabled : false ,
          onClick : emit: onToggleFilter
        )
        If ( when: slot: filter-open ) {
          Column [ filter-options-list ] {
            For ( each: slot: filter-options , as: filter-option , index: i ) {
              HostButton [ filter-option-button ] (
                label : filter-option ,
                onClick : emit: onSetFilter
              )
            }
          }
        }
      }
      HostButton [ search-button ] (
        label : slot: search-label ,
        onClick : emit: onSearch
      )
    }
    Text [ results-label ] (
      content : slot: results-label
    )
    Text [ results-summary ] (
      content : slot: results-summary
    )
    pkg::mosaic-pkg-toolkit::ListGroup (
      items : slot: results ,
      selected-index : slot: selected-index ,
      onSelect : emit: onSelectResult
    )
    Row [ browser-flag-row ] {
      Column [ flag-picker-column ] {
        Text [ flag-label ] (
          content : slot: flag-label
        )
        pkg::mosaic-pkg-toolkit::Select (
          value : slot: flag-value ,
          options : slot: flag-options ,
          placeholder : slot: flag-placeholder ,
          open : slot: flag-open ,
          disabled : false ,
          onToggle : emit: onToggleFlagPicker ,
          onChange : emit: onSetFlagSelected
        )
      }
      Text [ selected-flag-label ] (
        content : slot: selected-flag
      )
    }
    Row [ browser-tag-row ] {
      Column [ tag-edit-column ] {
        Text [ tag-edit-label ] (
          content : slot: tag-edit-label
        )
        HostInput [ tag-edit-input ] (
          value : slot: tag-edit ,
          placeholder : slot: tag-edit-placeholder ,
          disabled : false ,
          onChange : emit: onTagEditChange
        )
      }
      HostButton [ add-tag-button ] (
        label : slot: add-tag-label ,
        onClick : emit: onAddTagSelected
      )
      HostButton [ remove-tag-button ] (
        label : slot: remove-tag-label ,
        onClick : emit: onRemoveTagSelected
      )
    }
    Row [ browser-actions ] {
      HostButton [ open-button ] (
        label : slot: open-label ,
        onClick : emit: onOpenSelected
      )
      HostButton [ edit-button ] (
        label : slot: edit-label ,
        onClick : emit: onEditSelected
      )
      HostButton [ suspend-button ] (
        label : slot: suspend-label ,
        onClick : emit: onToggleSuspendSelected
      )
      HostButton [ mark-button ] (
        label : slot: mark-label ,
        onClick : emit: onToggleMarkSelected
      )
    }
  }
}
