// TaskApp — layout (moslayout).
//
// One screen: a header, an add-row (name + optional due date + Add), a summary line,
// and the auto-scheduled task list. Clicking a row toggles done; Delete removes it.

layout TaskApp {
  Column [ app-shell ] {
    // Projects. One button per project — the active one styled as selected via the
    // row's marker cell — plus an inline composer for creating another. Selecting is
    // by row index, matching how task rows report which row was acted on.
    Row [ project-bar ] {
      For ( each: slot: project-rows , as: p , index: pi ) {
        // A nested project gets a leading glyph; a top-level one has an empty indent
        // cell and so renders nothing, keeping the row flush left.
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
      HostInput [ project-input ] (
        value : slot: new-project-name ,
        placeholder : "New project" ,
        onChange : emit: onNewProjectNameChange
      )
      HostButton [ project-add ] ( label : "+ Project" , onClick : emit: onAddProject )
      HostButton [ project-sub ] ( label : "+ Sub" , onClick : emit: onAddSubproject )
    }

    Text [ title ] ( content : slot: app-title , a11y-role : heading )

    Row [ add-row ] {
      HostInput [ name-input ] (
        value : slot: new-task-name ,
        placeholder : "What needs doing?" ,
        onChange : emit: onNewTaskNameChange
      )
      HostInput [ due-input ] (
        value : slot: new-task-due ,
        placeholder : "Due YYYY-MM-DD (optional)" ,
        onChange : emit: onNewTaskDueChange
      )
      HostButton [ add-btn ] ( label : "Add" , onClick : emit: onAddTask )
    }

    Row [ summary-row ] {
      Text [ summary ] ( content : slot: summary )
      HostButton [ view-toggle ] (
        label : slot: view-toggle-label ,
        onClick : emit: onToggleView
      )
    }

    // The timeline and the task list are the two views of the same project; exactly one
    // shows at a time, chosen by the non-empty `timeline-mode` marker.
    If ( when: slot: timeline-mode ) {
      Column [ timeline ] {
        Text [ tl-scale ] ( content : slot: timeline-scale )
        For ( each: slot: timeline-rows , as: t , index: ti ) {
          Row [ timeline-row ] {
            Text [ tl-name ] ( content : ( t[0] ) )
            // A real proportional bar: the leading pad and the bar itself take their
            // widths from the row's data (UI36 data-driven sizing), both as percentages
            // of the shared track, so every bar lines up on one date scale. A critical
            // bar differs from a normal one only in colour — the geometry is identical.
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

    Column [ task-list ] {
      // Each row is a list of cells. The toggle and Delete buttons sit directly in
      // the For body so their `number` payload carries the outer row index `i`; the
      // name and the meta chips read individual cells by `( row[n] )`. Each chip is
      // wrapped in an `If` on its own cell so an empty cell renders nothing at all.
      For ( each: slot: task-rows , as: row , index: i ) {
        Row [ task-row ] {
          HostButton [ toggle ] ( label : ( row[0] ) , onClick : emit: onToggleTask )
          // The name is the disclosure control: clicking it opens this row's detail.
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
        // Progressive disclosure: the scheduling detail exists for every task but is
        // only rendered for the open row, so the default list stays a plain to-do list.
        If ( when: ( row[5] ) ) {
          Column [ task-detail ] {
            // Every line is guarded on its own cell, so an unscheduled task shows one
            // explanatory line rather than a panel padded with blanks.
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
