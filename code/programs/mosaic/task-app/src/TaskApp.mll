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

      Text [ rail-label ] ( content : "Projects" )
      Column [ rail-projects ] {
        For ( each: slot: project-rows , as: p , index: pi ) {
          Row [ rail-row ] {
            // A nested project gets a leading glyph; a top-level one has an empty
            // indent cell and renders nothing, keeping the row flush left.
            If ( when: ( p[2] ) ) {
              Text [ project-indent ] ( content : ( p[2] ) )
            }
            If ( when: ( p[1] ) ) {
              HostButton [ project-on ] ( label : ( p[0] ) , onClick : emit: onSelectProject )
            }
            Else {
              HostButton [ project-off ] ( label : ( p[0] ) , onClick : emit: onSelectProject )
            }
          }
        }
      }

      Row [ rail-composer ] {
        HostInput [ project-input ] (
          value : slot: new-project-name ,
          placeholder : "New project" ,
          onChange : emit: onNewProjectNameChange
        )
        HostButton [ project-add ] ( label : "+" , onClick : emit: onAddProject )
      }
      HostButton [ project-sub ] ( label : "+ Sub-project" , onClick : emit: onAddSubproject )
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
          // branches, so each of the four states gets its own name.
          If ( when: slot: timeline-mode ) {
            HostButton [ seg-list-off ] ( label : "List" , onClick : emit: onShowList )
            HostButton [ seg-tl-on ] ( label : "Timeline" , onClick : emit: onShowTimeline )
          }
          Else {
            HostButton [ seg-list-on ] ( label : "List" , onClick : emit: onShowList )
            HostButton [ seg-tl-off ] ( label : "Timeline" , onClick : emit: onShowTimeline )
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
