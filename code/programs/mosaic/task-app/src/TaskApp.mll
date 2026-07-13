// TaskApp — layout (moslayout).
//
// One screen: a header, an add-row (name + optional due date + Add), a summary line,
// and the auto-scheduled task list. Clicking a row toggles done; Delete removes it.

layout TaskApp {
  Column [ app-shell ] {
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
      For ( each: slot: task-rows , as: row , index: i ) {
        Row [ task-row ] {
          HostButton [ row-btn ] ( label : row , onClick : emit: onToggleTask )
          HostButton [ del-btn ] ( label : "Delete" , onClick : emit: onDeleteTask )
        }
      }
    }
  }
}
