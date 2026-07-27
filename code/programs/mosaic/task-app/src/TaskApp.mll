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

    Text [ summary ] ( content : slot: summary )

    Column [ task-list ] {
      // Each row is a list of cells. The toggle and Delete buttons sit directly in
      // the For body so their `number` payload carries the outer row index `i`; the
      // name and the meta chips read individual cells by `( row[n] )`. Each chip is
      // wrapped in an `If` on its own cell so an empty cell renders nothing at all.
      For ( each: slot: task-rows , as: row , index: i ) {
        Row [ task-row ] {
          HostButton [ toggle ] ( label : ( row[0] ) , onClick : emit: onToggleTask )
          Text [ task-name ] ( content : ( row[1] ) )
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
      }
    }
  }
}
