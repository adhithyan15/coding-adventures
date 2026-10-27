// JournalApp.mll — layout.
//
//   HostNavigationSplit [ app-shell ]          pane: the timeline; detail: the editor
//     HostScroll [ timeline-scroll ]           the pane scrolls (JO-5)
//       Column [ timeline ]
//         HostButton [ new-entry ]
//         search, Starred only, tags and journals (J4a–J4f)
//         If (has-on-this-day) toolkit RecordList   (on this day, J4c)
//         If (timeline-empty)  toolkit EmptyState
//         Else                 For each row: a mosaic-pkg-card Card + Open button
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
    // The pane scrolls (JO-5). A native window scrolls nothing unless the
    // layout says so, and the pane grows without bound: the timeline, the
    // tags and the journals all grow with the data. Without this, entries
    // past the first two or so were below the fold of the declared
    // 1100x760 window on every native backend, with no scrollbar, no
    // keyboard scroll and no gesture to reach them (only search could).
    // The web never showed it, because a browser page scrolls by default.
    // TaskApp hit the same bug and fixed it the same way (#14734).
    //
    // Card rows (Mosaic backlog item 4) are about 220px tall in that window,
    // so with them the SECOND entry already starts below the fold: this
    // scroll is what keeps every entry reachable (misfit M7).
    //
    // Wrapping from OUTSIDE the column keeps its padding inside the
    // scroll, so the last row keeps its breathing room at the end.
    HostScroll [ timeline-scroll ] {
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
        // Tags (J4d): one option per tag, stacked (the pane is 300px wide);
        // selecting filters every list, selecting it again clears the filter.
        If ( when: slot: has-tags ) {
          pkg::mosaic-pkg-toolkit::SegmentedControl (
            options : slot: tag-options ,
            selected-index : slot: selected-tag-index ,
            vertical : true ,
            disabled : false ,
            onSelect : emit: onSelectTag
          )
        }
        // Journals (J4f): a second SegmentedControl mount, after the tags so
        // the tag options keep their part names. "All journals" first.
        pkg::mosaic-pkg-toolkit::SegmentedControl (
          options : slot: journal-options ,
          selected-index : slot: selected-journal-index ,
          vertical : true ,
          disabled : false ,
          onSelect : emit: onSelectJournal
        )
        // A New journal field and Add; a refused name is said below it.
        Row [ journal-bar ] {
          HostInput [ journal-name-input ] (
            value : slot: new-journal-name ,
            placeholder : "New journal" ,
            a11y-label : "New journal name" ,
            disabled : false ,
            onChange : emit: onNewJournalNameChange
          )
          HostButton [ add-journal ] (
            label : "Add" ,
            onClick : emit: onAddJournal
          )
        }
        // Renaming and deleting the selected journal (J4i). Rename takes the
        // field's name; Delete moves the journal's entries to Personal.
        Row [ journal-actions ] {
          If ( when: slot: can-rename-journal ) {
            HostButton [ rename-journal ] (
              label : "Rename" ,
              onClick : emit: onRenameJournal
            )
          }
          If ( when: slot: can-delete-journal ) {
            HostButton [ delete-journal ] (
              label : "Delete journal" ,
              onClick : emit: onDeleteJournal
            )
          }
        }
        If ( when: slot: journal-error ) {
          Text [ journal-error ] ( content : slot: journal-error )
        }
        // Export (J6a): the whole journal to a file the person chooses, through
        // Mosaic's standard files.save effect -- no host code in this app.
        Row [ export-bar ] {
          HostButton [ export-journal ] (
            label : "Export" ,
            disabled : slot: exporting ,
            onClick : emit: onExportJournal
          )
        }
        If ( when: slot: export-status ) {
          Text [ export-status ] ( content : slot: export-status )
        }
        // On this day (J4c): the app's one RecordList mount, above the
        // timeline, only when earlier years have entries on today's date.
        // (It was the second mount, suffixed `-m2` by #15959, while the
        // timeline was a RecordList too.)
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
            // A fourth EmptyState mount (J4h): the selected journal is empty,
            // which says more than "nothing starred" would.
            If ( when: slot: journal-empty ) {
              pkg::mosaic-pkg-toolkit::EmptyState (
                title : "No entries in this journal" ,
                message : "New entries you write while it is selected are filed here." ,
                action-label : ""
              )
            }
            Else {
              // The last EmptyState mount: the filter is on and nothing is starred.
              If ( when: slot: no-starred ) {
                pkg::mosaic-pkg-toolkit::EmptyState (
                  title : "No starred entries" ,
                  message : "Star an entry to keep it here." ,
                  action-label : ""
                )
              }
              Else {
                // The timeline: each entry is a mosaic-pkg-card Card (its first
                // real consumer), the day's heading above the first card of
                // each day, and an Open button under every card.
                //
                //   Thursday, 24 September 2026        ← timeline-day (heading)
                //   ┌ Card ─────────────────────────┐
                //   │ First light                   │  title      row[2]
                //   │ Walked to the lighthouse …    │  body-text  row[3]
                //   │ #travel                       │  footer     row[4]
                //   └───────────────────────────────┘
                //   [ Open ]  ★                       ← named row[6], badge row[5]
                //
                // Why the button is beside the card, not around it: Card.mil
                // says interaction belongs to "an actionable container", but
                // a HostButton's children are dropped on six of eight backends
                // (UI29 §2.1, EM-8), so `HostButton { Card }` is an empty
                // button almost everywhere. Until that lands, the kernel's rule
                // is a labelled button with the richer content beside it.
                //
                // Why "Open" and not the title: a title button beside a card
                // that also shows the title would say it twice. The button's
                // accessible name is row[6], "Open First light", so each row
                // is still one tab stop named after its entry (as RecordList's
                // title button was), and the name contains the visible word
                // (WCAG 2.5.3). Layout expressions cannot join strings, so the
                // app sends the whole name.
                //
                // The open entry's row has its own "-selected" parts, as in
                // RecordList, so every backend can draw the highlight without
                // native button-state support. Card has no selected state of
                // its own (misfit M5), so the highlight is the frame around
                // it. See journal-component-demand-audit-v1.md, "Card as the
                // timeline row", for every misfit.
                Column [ timeline-list ] {
                  For ( each: slot: timeline-rows , as: row , index: i ) {
                    If ( when: ( row[1] ) ) {
                      Text [ timeline-day ] (
                        content : ( row[1] ) ,
                        a11y-role : heading
                      )
                    }
                    If ( when: ( row[0] == selectedKey ) ) {
                      Column [ timeline-entry-selected ] {
                        pkg::mosaic-pkg-card::Card (
                          title : ( row[2] ) ,
                          body-text : ( row[3] ) ,
                          footer : ( row[4] )
                        )
                        Row [ timeline-entry-actions-selected ] {
                          HostButton [ timeline-entry-open-selected ] (
                            label : "Open" ,
                            a11y-label : ( row[6] ) ,
                            selected : true ,
                            onClick : emit: onSelectEntry
                          )
                          If ( when: ( row[5] ) ) {
                            Text [ timeline-entry-badge-selected ] ( content : ( row[5] ) )
                          }
                        }
                      }
                    }
                    Else {
                      Column [ timeline-entry ] {
                        pkg::mosaic-pkg-card::Card (
                          title : ( row[2] ) ,
                          body-text : ( row[3] ) ,
                          footer : ( row[4] )
                        )
                        Row [ timeline-entry-actions ] {
                          HostButton [ timeline-entry-open ] (
                            label : "Open" ,
                            a11y-label : ( row[6] ) ,
                            selected : false ,
                            onClick : emit: onSelectEntry
                          )
                          If ( when: ( row[5] ) ) {
                            Text [ timeline-entry-badge ] ( content : ( row[5] ) )
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
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
      // An entry's journal (J4g): a third SegmentedControl mount, only when
      // there is more than one journal; Save below files or moves it there.
      If ( when: slot: has-journals ) {
        Column [ journal-block ] {
          Text [ journal-label ] ( content : "Journal" )
          pkg::mosaic-pkg-toolkit::SegmentedControl (
            options : slot: draft-journal-options ,
            selected-index : slot: draft-journal-index ,
            vertical : false ,
            disabled : false ,
            onSelect : emit: onDraftJournalChange
          )
        }
      }
      // An entry's day (J4e), blank for today; saved by Save below.
      Column [ date-block ] {
        Text [ date-label ] ( content : "Date" )
        HostInput [ date-input ] (
          value : slot: draft-date ,
          placeholder : "YYYY-MM-DD (blank for today)" ,
          a11y-label : "Date" ,
          disabled : false ,
          onChange : emit: onDateChange
        )
      }
      // Tags (J4d), comma-separated, saved with the entry by Save below.
      Column [ tags-block ] {
        Text [ tags-label ] ( content : "Tags" )
        HostInput [ tags-input ] (
          value : slot: draft-tags ,
          placeholder : "travel, family" ,
          a11y-label : "Tags" ,
          disabled : false ,
          onChange : emit: onTagsChange
        )
      }
      // Why Save refused the draft (J4e): a bad date or tag. Nothing was
      // written; any other action clears it.
      If ( when: slot: draft-error ) {
        Text [ draft-error ] ( content : slot: draft-error )
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
