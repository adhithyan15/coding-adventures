// Calendar.mll — layout for the month-grid calendar view.
//
// Composition (kernel primitives only — no wrapped package, same as
// mosaic-pkg-grid's own Grid.mll):
//
//   Column [ calendar-root ]
//     Row [ calendar-head ]              (title + prev/next nav)
//     Row [ calendar-dow ]                (Sun..Sat labels, static — they never
//                                          change, so no slot for them. Each gets
//                                          its own part name — moslayout rejects a
//                                          literal part name reused across static
//                                          siblings, the same "duplicate part per
//                                          instance" constraint the segmented
//                                          switch's off/off2/off3/off4 already
//                                          works around.)
//     Row [ calendar-grid ]                (flex-wrap: wrap — see below)
//       For over calendar-cells
//         Column [ calendar-cell ]
//           day number (today gets its own part, badge-styled)
//           HostDropTarget [ calendar-cell-drop ]
//             For over calendar-events, filtered to this cell's day-key
//               HostDraggable [ calendar-event ]
//
// A flex-wrap grid, not CSS grid
// -------------------------------
// Mosaic's layout primitives are Box/Row/Column/Stack — there is no grid
// primitive. `calendar-grid` is a Row with `flex-wrap: "wrap"` and each
// `calendar-cell` is `width: "14.2857%"` (100÷7): seven same-width cells per
// row, wrapping every 7, produces the 6-row month grid without a new
// primitive. `flex-wrap` is just another kebab-case CSS property — mosstyle's
// per-property system passes any of them through generically (the same
// mechanism the design-fidelity pass already exercised for directional
// padding).
//
// Placing an event in its cell: keys, not indices — same reasoning as
// TaskApp's Board section (see TaskApp.mil's board-columns/board-cards note).
// `If ( when: ( ev[2] == cell[1] ) )` compares the event's day-key against
// the cell's day-key, nested inside the cell loop, exactly like Board's
// `If ( when: ( card[1] == col[1] ) )` compares a card's column-key against
// the column's.
//
// One draggable part, conditional CHILD chips for state — not four branches
// -------------------------------------------------------------------------
// TaskApp's Board section renders exactly one `board-card` HostDraggable part
// always, and adds a small conditional `card-crit` text child for the
// critical marker, rather than branching the whole card into differently
// -styled variants per state (mosstyle can't express a per-data-value style
// on one part; only per-branch part names can differ, and duplicating the
// entire draggable+drop-target+event-loop across a 3-or-4-way branch was
// judged not worth the resulting file size for what is ultimately a colour
// difference). Calendar follows the identical, already-accepted trade-off:
// one `calendar-event` part, with independent (non-exclusive — a task can be
// both critical AND overdue) conditional chips for critical/completed/
// overdue. This is the same reason Board's own critical treatment is a text
// chip rather than the mock's coloured left border — tracked as a known,
// deferred gap in BACKLOG.md, not something Calendar re-litigates here.

layout Calendar {
  Column [ calendar-root ] {
    Row [ calendar-head ] {
      Text [ calendar-title ] ( content : slot: calendar-title )
      Row [ calendar-nav ] {
        HostButton [ calendar-prev ] ( label : "‹" , onClick : emit: onPrev )
        HostButton [ calendar-next ] ( label : "›" , onClick : emit: onNext )
      }
    }
    Row [ calendar-dow ] {
      Text [ calendar-dow-sun ] ( content : "Sun" )
      Text [ calendar-dow-mon ] ( content : "Mon" )
      Text [ calendar-dow-tue ] ( content : "Tue" )
      Text [ calendar-dow-wed ] ( content : "Wed" )
      Text [ calendar-dow-thu ] ( content : "Thu" )
      Text [ calendar-dow-fri ] ( content : "Fri" )
      Text [ calendar-dow-sat ] ( content : "Sat" )
    }
    Row [ calendar-grid ] {
      For ( each: slot: calendar-cells , as: cell , index: ci ) {
        Column [ calendar-cell ] {
          If ( when: ( cell[2] ) ) {
            Text [ calendar-daynum-today ] ( content : ( cell[0] ) )
          }
          Else {
            Text [ calendar-daynum ] ( content : ( cell[0] ) )
          }
          HostDropTarget [ calendar-cell-drop ] (
            drop-key : ( cell[1] ) ,
            onDrop : emit: onEventDropped
          ) {
            For ( each: slot: calendar-events , as: ev , index: ei ) {
              If ( when: ( ev[2] == cell[1] ) ) {
                HostDraggable [ calendar-event ] (
                  drag-key : ( ev[0] ) ,
                  drag-kind : "task" ,
                  drag-label : ( ev[1] )
                ) {
                  Text [ calendar-event-label ] ( content : ( ev[1] ) )
                  If ( when: ( ev[3] ) ) {
                    Text [ calendar-event-crit ] ( content : "Critical" )
                  }
                  If ( when: ( ev[4] ) ) {
                    Text [ calendar-event-done ] ( content : "Done" )
                  }
                  If ( when: ( ev[5] ) ) {
                    Text [ calendar-event-over ] ( content : "Overdue" )
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
