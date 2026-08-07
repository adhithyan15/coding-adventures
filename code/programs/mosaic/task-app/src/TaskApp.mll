// TaskApp — layout (moslayout).
//
// The shell follows code/specs/task-app-ui-design.md: a quiet left RAIL holding the
// workspace's projects, and a MAIN column with a topbar (title, summary, status,
// view switch) over the content. Exactly one view shows at a time.
//
// Everything here is a renderer. Grouping, ordering, formatting, scheduling and the
// status verdict all arrive already decided by the engine; the layout only places them.

layout TaskApp {
  Row [ app-shell ] {

    // ── RAIL ────────────────────────────────────────────────────────────────
    Column [ rail ] {
      Row [ brand ] {
        Box [ brand-mark ] { }
        Text [ brand-name ] ( content : "Planner" )
      }

      // The nested-project tree + add-project composer, extracted verbatim
      // into pkg::mosaic-pkg-project-nav::ProjectNav — see
      // code/specs/task-app-project-nav-v1.md. TaskApp adds no shaping of
      // its own here.
      pkg::mosaic-pkg-project-nav::ProjectNav (
        nav-title : "Projects" ,
        project-rows : slot: project-rows ,
        new-project-name : slot: new-project-name ,
        onSelectProject : emit: onSelectProject ,
        onNewProjectNameChange : emit: onNewProjectNameChange ,
        onAddProject : emit: onAddProject ,
        onAddSubproject : emit: onAddSubproject
      )
    }

    // ── MAIN ────────────────────────────────────────────────────────────────
    Column [ main ] {
      Row [ topbar ] {
        Column [ title-block ] {
          Text [ title ] ( content : slot: app-title , a11y-role : heading )
          Row [ subline ] {
            Text [ summary ] ( content : slot: summary )
            // The engine's own verdict, coloured by tone: a warning reads red.
            If ( when: slot: status-warn ) {
              Text [ pill-warn ] ( content : slot: status-label )
            }
            Else {
              Text [ pill-ok ] ( content : slot: status-label )
            }
          }
        }

        // Segmented view switch. Two explicit emits rather than one toggle, so
        // clicking the view you are already on is a no-op instead of a swap.
        Row [ seg ] {
          // Part names are unique layout-wide, even across mutually-exclusive
          // branches, so each of the six buttons gets its own name in each of
          // the six branches (one "on" + five "off" variants per button).
          If ( when: slot: timeline-mode ) {
            HostButton [ seg-list-off ] ( label : "List" , onClick : emit: onShowList )
            HostButton [ seg-board-off ] ( label : "Board" , onClick : emit: onShowBoard )
            HostButton [ seg-sheet-off ] ( label : "Sheet" , onClick : emit: onShowSheet )
            HostButton [ seg-cal-off ] ( label : "Calendar" , onClick : emit: onShowCalendar )
            HostButton [ seg-notes-off ] ( label : "Notes" , onClick : emit: onShowNotes )
            HostButton [ seg-tl-on ] ( label : "Timeline" , onClick : emit: onShowTimeline )
          }
          Else {
            If ( when: slot: board-mode ) {
              HostButton [ seg-list-off2 ] ( label : "List" , onClick : emit: onShowList )
              HostButton [ seg-board-on ] ( label : "Board" , onClick : emit: onShowBoard )
              HostButton [ seg-sheet-off2 ] ( label : "Sheet" , onClick : emit: onShowSheet )
              HostButton [ seg-cal-off2 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
              HostButton [ seg-notes-off2 ] ( label : "Notes" , onClick : emit: onShowNotes )
              HostButton [ seg-tl-off2 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
            }
            Else {
              If ( when: slot: sheet-mode ) {
                HostButton [ seg-list-off3 ] ( label : "List" , onClick : emit: onShowList )
                HostButton [ seg-board-off3 ] ( label : "Board" , onClick : emit: onShowBoard )
                HostButton [ seg-sheet-on ] ( label : "Sheet" , onClick : emit: onShowSheet )
                HostButton [ seg-cal-off3 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                HostButton [ seg-notes-off3 ] ( label : "Notes" , onClick : emit: onShowNotes )
                HostButton [ seg-tl-off3 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
              }
              Else {
                If ( when: slot: calendar-mode ) {
                  HostButton [ seg-list-off4 ] ( label : "List" , onClick : emit: onShowList )
                  HostButton [ seg-board-off5 ] ( label : "Board" , onClick : emit: onShowBoard )
                  HostButton [ seg-sheet-off4 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                  HostButton [ seg-cal-on ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                  HostButton [ seg-notes-off4 ] ( label : "Notes" , onClick : emit: onShowNotes )
                  HostButton [ seg-tl-off4 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                }
                Else {
                  If ( when: slot: notes-mode ) {
                    HostButton [ seg-list-off5 ] ( label : "List" , onClick : emit: onShowList )
                    HostButton [ seg-board-off6 ] ( label : "Board" , onClick : emit: onShowBoard )
                    HostButton [ seg-sheet-off5 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                    HostButton [ seg-cal-off5 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                    HostButton [ seg-notes-on ] ( label : "Notes" , onClick : emit: onShowNotes )
                    HostButton [ seg-tl-off5 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                  }
                  Else {
                    HostButton [ seg-list-on ] ( label : "List" , onClick : emit: onShowList )
                    HostButton [ seg-board-off4 ] ( label : "Board" , onClick : emit: onShowBoard )
                    HostButton [ seg-sheet-off3 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                    HostButton [ seg-cal-off4 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                    HostButton [ seg-notes-off5 ] ( label : "Notes" , onClick : emit: onShowNotes )
                    HostButton [ seg-tl-off ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                  }
                }
              }
            }
          }
        }
      }

      Column [ content ] {
        If ( when: slot: timeline-mode ) {
          Column [ timeline-card ] {
            Text [ tl-scale ] ( content : slot: timeline-scale )
            For ( each: slot: timeline-rows , as: t , index: ti ) {
              Row [ timeline-row ] {
                Text [ tl-name ] ( content : ( t[0] ) )
                // A real proportional bar: the leading pad and the bar take their
                // widths from the row's data (UI36 data-driven sizing), both as
                // percentages of the shared track, so every bar sits on one date
                // scale. A critical bar differs only in colour — geometry is equal.
                Row [ tl-track ] {
                  Box [ tl-pad ] ( width : ( t[1] ) )
                  If ( when: ( t[4] ) ) {
                    Box [ tl-bar-crit ] ( width : ( t[2] ) )
                  }
                  Else {
                    Box [ tl-bar ] ( width : ( t[2] ) )
                  }
                }
                Text [ tl-window ] ( content : ( t[3] ) )
              }
            }
          }
        }
        Else {
        If ( when: slot: board-mode ) {
          // The board is what the UI35 drag family exists for: each column is a drop
          // target, each card a draggable. A drop dispatches a PROPOSAL — the engine
          // decides whether the move is legal and performs it; the UI never moves a
          // card itself.
          Row [ board ] {
            For ( each: slot: board-columns , as: col , index: ci ) {
              Column [ board-col ] {
                Text [ col-head ] ( content : ( col[0] ) )
                HostDropTarget [ col-drop ] (
                  drop-key : ( col[1] ) ,
                  onDrop : emit: onCardDropped
                ) {
                  For ( each: slot: board-cards , as: card , index: cdi ) {
                    // Place a card by comparing keys rather than nesting a list in a
                    // list — one `If` says everything, and both loops stay flat.
                    If ( when: ( card[1] == col[1] ) ) {
                      HostDraggable [ board-card ] (
                        drag-key : ( card[2] ) ,
                        drag-kind : "task" ,
                        drag-label : ( card[0] )
                      ) {
                        Text [ card-name ] ( content : ( card[0] ) )
                        If ( when: ( card[3] ) ) {
                          Text [ card-crit ] ( content : ( card[3] ) )
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
        Else {
        If ( when: slot: sheet-mode ) {
          // TaskApp wraps the Sheet with its own label composer, above the grid —
          // deliberately NOT inside mosaic-pkg-sheet (see new-label-name's slot
          // comment on TaskApp.mil for why). Every sheet-* slot/emit below is still
          // a straight pass-through to the package itself.
          Column [ sheet-view-wrap ] {
            Row [ label-composer ] {
              HostInput [ label-name-input ] (
                value : slot: new-label-name ,
                placeholder : "New label" ,
                onChange : emit: onNewLabelNameChange
              )
              HostButton [ label-add-btn ] ( label : "+ Label" , onClick : emit: onAddLabel )
            }
            pkg::mosaic-pkg-sheet::Sheet (
              viewport-rows : slot: sheet-viewport-rows ,
              column-headers : slot: sheet-column-headers ,
              column-widths : slot: sheet-column-widths ,
              selected-row : slot: sheet-selected-row ,
              selected-col : slot: sheet-selected-col ,
              edit-row : slot: sheet-edit-row ,
              edit-col : slot: sheet-edit-col ,
              edit-content : slot: sheet-edit-content ,
              filter-text : slot: sheet-filter-text ,
              sort-field : slot: sheet-sort-field ,
              sort-options : slot: sheet-sort-options ,
              sort-open : slot: sheet-sort-open ,
              sort-ascending : slot: sheet-sort-ascending ,
              onNavigate : emit: onSheetNavigate ,
              onFormulaChange : emit: onSheetFormulaChange ,
              onEditCommit : emit: onSheetEditCommit ,
              onEditCancel : emit: onSheetEditCancel ,
              onFilterChange : emit: onSheetFilterChange ,
              onSortFieldChange : emit: onSheetSortFieldChange ,
              onToggleSortOpen : emit: onSheetToggleSortOpen ,
              onToggleSortDirection : emit: onSheetToggleSortDirection
            )
          }
        }
        Else {
        If ( when: slot: calendar-mode ) {
          // Every calendar-* slot/emit is a straight pass-through to the
          // package — TaskApp adds no shaping of its own, see Calendar.mil
          // for the contract and task-app-calendar-v1.md for the scope.
          pkg::mosaic-pkg-calendar::Calendar (
            calendar-title : slot: calendar-title ,
            calendar-cells : slot: calendar-cells ,
            calendar-events : slot: calendar-events ,
            onPrev : emit: onCalendarPrev ,
            onNext : emit: onCalendarNext ,
            onEventDropped : emit: onCalendarEventDropped
          )
        }
        Else {
        If ( when: slot: notes-mode ) {
          // Every notes-* slot/emit is a straight pass-through to the
          // package — TaskApp adds no shaping of its own, see Notes.mil
          // for the contract and task-app-notes-ui-v1.md for the scope.
          pkg::mosaic-pkg-notes::Notes (
            notes-title : slot: notes-title ,
            note-rows : slot: note-rows ,
            selected-note-id : slot: selected-note-id ,
            title-value : slot: note-title-value ,
            body-value : slot: note-body-value ,
            task-name-value : slot: note-task-value ,
            onSelectNote : emit: onSelectNote ,
            onNewNote : emit: onNewNote ,
            onTitleChange : emit: onNoteTitleChange ,
            onBodyChange : emit: onNoteBodyChange ,
            onTaskNameChange : emit: onNoteTaskNameChange ,
            onSave : emit: onSaveNote ,
            onDelete : emit: onDeleteNote ,
            onCancel : emit: onCancelNote
          )
        }
        Else {
          Column [ list-wrap ] {
            Row [ composer ] {
              HostInput [ name-input ] (
                value : slot: new-task-name ,
                placeholder : "What needs doing?" ,
                onChange : emit: onNewTaskNameChange
              )
              HostInput [ due-input ] (
                value : slot: new-task-due ,
                placeholder : "Due (optional)" ,
                onChange : emit: onNewTaskDueChange
              )
              HostButton [ add-btn ] ( label : "Add task" , onClick : emit: onAddTask )
            }

            Column [ task-list ] {
              For ( each: slot: task-rows , as: row , index: i ) {
                // A group heading, present only on the row that opens a group — the
                // engine decides the grouping, so the layout just prints the label
                // where it is handed one.
                If ( when: ( row[9] ) ) {
                  Text [ group-head ] ( content : ( row[9] ) )
                }
                Column [ task-card ] {
                  Row [ task-row ] {
                    HostButton [ toggle ] ( label : ( row[0] ) , onClick : emit: onToggleTask )
                    // The name is the disclosure control: it opens this row's detail.
                    HostButton [ task-name ] ( label : ( row[1] ) , onClick : emit: onExpandTask )
                    If ( when: ( row[2] ) ) {
                      Text [ chip-due ] ( content : ( row[2] ) )
                    }
                    If ( when: ( row[3] ) ) {
                      Text [ chip-sched ] ( content : ( row[3] ) )
                    }
                    If ( when: ( row[4] ) ) {
                      Text [ chip-over ] ( content : ( row[4] ) )
                    }
                    If ( when: ( row[10] ) ) {
                      Text [ chip-priority ] ( content : ( row[10] ) )
                    }
                    If ( when: ( row[11] ) ) {
                      Text [ chip-labels ] ( content : ( row[11] ) )
                    }
                    HostButton [ del-btn ] ( label : "Delete" , onClick : emit: onDeleteTask )
                  }
                  // Progressive disclosure: the scheduling detail exists for every
                  // task but is rendered only for the open row.
                  If ( when: ( row[5] ) ) {
                    Column [ task-detail ] {
                      If ( when: ( row[6] ) ) {
                        Text [ detail-sched ] ( content : ( row[6] ) )
                      }
                      If ( when: ( row[7] ) ) {
                        Text [ detail-slack ] ( content : ( row[7] ) )
                      }
                      If ( when: ( row[8] ) ) {
                        Text [ detail-free ] ( content : ( row[8] ) )
                      }
                      If ( when: ( row[12] ) ) {
                        Text [ detail-deps ] ( content : ( row[12] ) )
                      }
                      If ( when: ( row[13] ) ) {
                        Text [ detail-notes ] ( content : ( row[13] ) )
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
      }
    }
  }
}
