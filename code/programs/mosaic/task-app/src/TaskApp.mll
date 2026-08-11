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
        // The bridge arc (task-app-icon-assets-v1.md) — two upright posts
        // joined by an arc, user-picked from a proposed shortlist. All three
        // pieces are independently positioned inside a Stack (position:
        // relative), each a static Box (no data, so nothing here needed the
        // UI36 background extension the ring did).
        Stack [ brand-mark ] {
          Box [ brand-post-left ] { }
          Box [ brand-post-right ] { }
          Box [ brand-arc ] { }
        }
        Text [ brand-name ] ( content : "Trestle" )
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
            // The dot (task-app-icon-assets-v1.md) is `background: currentColor`
            // in mostyle, so it always matches whichever branch's text colour —
            // no separate warn/ok dot styling to keep in sync.
            If ( when: slot: status-warn ) {
              Row [ pill-warn ] {
                Box [ pill-dot-warn ] { }
                Text [ pill-warn-label ] ( content : slot: status-label )
              }
            }
            Else {
              Row [ pill-ok ] {
                Box [ pill-dot-ok ] { }
                Text [ pill-ok-label ] ( content : slot: status-label )
              }
            }
          }
        }

        // The project-progress ring (task-app-icon-assets-v1.md) — a donut via
        // one filled circle (its background bound to the host-computed
        // conic-gradient, UI36) with a smaller same-surface-colour circle
        // stacked on top to punch the hole. No SVG.
        Row [ ring-wrap ] {
          Stack [ ring-circle ] {
            Box [ ring-fill ] ( background : slot: ring-gradient )
            Box [ ring-hole ] { }
          }
          Column [ ring-caption ] {
            Text [ ring-pct ] ( content : slot: ring-percent )
            Text [ ring-label ] ( content : "complete" )
          }
        }

        // The theme toggle (task-app-icon-assets-v1.md) — see `theme-is-dark`'s
        // doc comment in TaskApp.mil. `HostButton` has no way to render a
        // child (only its flat `label`, per mosaic-emit-react's
        // `host_button_label_body` — a real kernel gap, not something this
        // slice works around by inventing one) and no `a11y-label`-style prop
        // either, so the accessible name has to be the `label` text itself
        // — kept real (a screen reader announces it), just visually hidden
        // (`color: transparent` in the .msl part; the button's own box stays
        // its full clickable size, only the text glyphs vanish). The crescent
        // (an inset box-shadow cut into a filled circle) or plain filled sun
        // is drawn entirely by the button's own background/box-shadow — no
        // SVG, no more `position: fixed` button living outside this component.
        If ( when: slot: theme-is-dark ) {
          HostButton [ theme-toggle-sun ] (
            label : "Switch to the light theme" ,
            onClick : emit: onToggleTheme
          )
        }
        Else {
          HostButton [ theme-toggle-moon ] (
            label : "Switch to the dark theme" ,
            onClick : emit: onToggleTheme
          )
        }

        // Flips the active project's complexity tier — see
        // code/specs/task-app-complexity-config-v1.md. Deliberately a single
        // button here rather than a per-project-row control in the rail:
        // this acts on whichever project is currently active, same as the
        // view switcher right next to it.
        HostButton [ complexity-toggle ] (
          label : slot: complexity-label ,
          onClick : emit: onToggleProjectComplexity
        )

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
            // Board-tier projects never show Timeline — see `allow-timeline`'s
            // doc comment. Because `timeline-mode` can only be truthy for a
            // Full-tier project (main.tsx forces the view away from Timeline
            // whenever the active project is Board — see main.tsx's dispatch
            // for onSelectProject/onToggleProjectComplexity), this branch is
            // reachable only when `allow-timeline` is already non-empty; the
            // `If` here is defense-in-depth, not the thing doing the hiding.
            If ( when: slot: allow-timeline ) {
              HostButton [ seg-tl-on ] ( label : "Timeline" , onClick : emit: onShowTimeline )
            }
          }
          Else {
            If ( when: slot: board-mode ) {
              HostButton [ seg-list-off2 ] ( label : "List" , onClick : emit: onShowList )
              HostButton [ seg-board-on ] ( label : "Board" , onClick : emit: onShowBoard )
              HostButton [ seg-sheet-off2 ] ( label : "Sheet" , onClick : emit: onShowSheet )
              HostButton [ seg-cal-off2 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
              HostButton [ seg-notes-off2 ] ( label : "Notes" , onClick : emit: onShowNotes )
              If ( when: slot: allow-timeline ) {
                HostButton [ seg-tl-off2 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
              }
            }
            Else {
              If ( when: slot: sheet-mode ) {
                HostButton [ seg-list-off3 ] ( label : "List" , onClick : emit: onShowList )
                HostButton [ seg-board-off3 ] ( label : "Board" , onClick : emit: onShowBoard )
                HostButton [ seg-sheet-on ] ( label : "Sheet" , onClick : emit: onShowSheet )
                HostButton [ seg-cal-off3 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                HostButton [ seg-notes-off3 ] ( label : "Notes" , onClick : emit: onShowNotes )
                If ( when: slot: allow-timeline ) {
                  HostButton [ seg-tl-off3 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                }
              }
              Else {
                If ( when: slot: calendar-mode ) {
                  HostButton [ seg-list-off4 ] ( label : "List" , onClick : emit: onShowList )
                  HostButton [ seg-board-off5 ] ( label : "Board" , onClick : emit: onShowBoard )
                  HostButton [ seg-sheet-off4 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                  HostButton [ seg-cal-on ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                  HostButton [ seg-notes-off4 ] ( label : "Notes" , onClick : emit: onShowNotes )
                  If ( when: slot: allow-timeline ) {
                    HostButton [ seg-tl-off4 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                  }
                }
                Else {
                  If ( when: slot: notes-mode ) {
                    HostButton [ seg-list-off5 ] ( label : "List" , onClick : emit: onShowList )
                    HostButton [ seg-board-off6 ] ( label : "Board" , onClick : emit: onShowBoard )
                    HostButton [ seg-sheet-off5 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                    HostButton [ seg-cal-off5 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                    HostButton [ seg-notes-on ] ( label : "Notes" , onClick : emit: onShowNotes )
                    If ( when: slot: allow-timeline ) {
                      HostButton [ seg-tl-off5 ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                    }
                  }
                  Else {
                    HostButton [ seg-list-on ] ( label : "List" , onClick : emit: onShowList )
                    HostButton [ seg-board-off4 ] ( label : "Board" , onClick : emit: onShowBoard )
                    HostButton [ seg-sheet-off3 ] ( label : "Sheet" , onClick : emit: onShowSheet )
                    HostButton [ seg-cal-off4 ] ( label : "Calendar" , onClick : emit: onShowCalendar )
                    HostButton [ seg-notes-off5 ] ( label : "Notes" , onClick : emit: onShowNotes )
                    If ( when: slot: allow-timeline ) {
                      HostButton [ seg-tl-off ] ( label : "Timeline" , onClick : emit: onShowTimeline )
                    }
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
            // The day-grid ruler — see code/specs/task-app-richer-gantt-v1.md's
            // "Day-grid feasibility note" for why this is a strip above the
            // bars rather than a per-row background (the kernel has no
            // z-index/absolute-positioning primitive to composite one).
            Row [ tl-grid ] {
              For ( each: slot: timeline-grid , as: g , index: gi ) {
                If ( when: ( g[2] ) ) {
                  Box [ tl-grid-today ] ( width : ( g[0] ) )
                }
                Else {
                  If ( when: ( g[1] ) ) {
                    Box [ tl-grid-weekend ] ( width : ( g[0] ) )
                  }
                  Else {
                    Box [ tl-grid-day ] ( width : ( g[0] ) )
                  }
                }
              }
            }
            // The legend — static copy, not data-bound (see TaskApp.mil's doc
            // comment on why this isn't a slot).
            Row [ tl-legend ] {
              Row [ tl-legend-item ] {
                Box [ tl-legend-swatch ] { }
                Text [ tl-legend-label ] ( content : "On track" )
              }
              Row [ tl-legend-item2 ] {
                Box [ tl-legend-swatch-crit ] { }
                Text [ tl-legend-label2 ] ( content : "Critical path" )
              }
              Row [ tl-legend-item3 ] {
                Box [ tl-legend-swatch-milestone ] { }
                Text [ tl-legend-label3 ] ( content : "Milestone" )
              }
              Row [ tl-legend-item4 ] {
                Box [ tl-legend-swatch-today ] { }
                Text [ tl-legend-label4 ] ( content : "Today" )
              }
            }
            For ( each: slot: timeline-rows , as: t , index: ti ) {
              Row [ timeline-row ] {
                Text [ tl-name ] ( content : ( t[0] ) )
                // A real proportional bar: the leading pad and the bar take their
                // widths from the row's data (UI36 data-driven sizing), both as
                // percentages of the shared track, so every bar sits on one date
                // scale. A milestone (t[5]) renders as a diamond instead of the
                // usual bar; a non-milestone bar carries a percent-complete fill
                // (t[6]) inside it. Every bar is wrapped in HostTooltip (t[7])
                // for the hover detail the design calls for.
                Row [ tl-track ] {
                  Box [ tl-pad ] ( width : ( t[1] ) )
                  If ( when: ( t[5] ) ) {
                    // No width binding here, deliberately: a milestone is
                    // zero-duration by definition, so it's a small FIXED-size
                    // marker (see the .msl part), not a bar sized from t[2]
                    // the way every other row's shape is. UI36's own
                    // precedence rule (a bound size always beats a static
                    // one) means binding width here would make a static
                    // small-diamond style unreachable.
                    HostTooltip ( text : ( t[7] ) ) {
                      Box [ tl-bar-milestone ] { }
                    }
                  }
                  Else {
                    If ( when: ( t[4] ) ) {
                      HostTooltip ( text : ( t[7] ) ) {
                        Column [ tl-bar-crit ] ( width : ( t[2] ) ) {
                          Box [ tl-bar-fill-crit ] ( width : ( t[6] ) )
                        }
                      }
                    }
                    Else {
                      HostTooltip ( text : ( t[7] ) ) {
                        Column [ tl-bar ] ( width : ( t[2] ) ) {
                          Box [ tl-bar-fill ] ( width : ( t[6] ) )
                        }
                      }
                    }
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
              // The dashed-box plus mark ahead of the inputs — decoration, not a
              // button (the mock's own `.composer .plus` is `aria-hidden`); the
              // real "add" action is the `add-btn` below. Two crossed bars in a
              // Stack, no SVG (task-app-icon-assets-v1.md).
              Stack [ composer-plus ] {
                Box [ plus-bar-h ] { }
                Box [ plus-bar-v ] { }
              }
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
                // where it is handed one. The count badge (row[14],
                // task-app-icon-assets-v1.md) is co-present by construction but
                // still gated by its own If, matching row[10]/row[11]'s discipline
                // rather than assuming the pairing.
                If ( when: ( row[9] ) ) {
                  Row [ group-head-row ] {
                    Text [ group-head ] ( content : ( row[9] ) )
                    If ( when: ( row[14] ) ) {
                      Text [ group-count ] ( content : ( row[14] ) )
                    }
                  }
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
