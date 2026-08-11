layout DragBoard {
  Column [ root ] {
    HostDraggable [ card ] (
      drag-key : "task-a" ,
      drag-kind : "task" ,
      drag-label : "Write spec" ,
      onDragStart : emit: onDragStart ,
      onDragEnd : emit: onDragEnd
    ) {
      Text ( content : "Write spec" )
    }
    HostDropTarget [ rejected-lane ] (
      drop-key : "rejected" ,
      accepts : "note" ,
      onDrop : emit: onDrop
    ) {
      Text ( content : "Rejected" )
    }
    HostDropTarget [ disabled-lane ] (
      drop-key : "disabled" ,
      accepts : "task" ,
      drop-disabled : true ,
      onDrop : emit: onDrop
    ) {
      Text ( content : "Disabled" )
    }
    HostDropTarget [ lane ] (
      drop-key : "done" ,
      accepts : "task" ,
      onDragEnter : emit: onDragEnter ,
      onDragLeave : emit: onDragLeave ,
      onDropHover : emit: onDropHover ,
      onDrop : emit: onDrop
    ) {
      Text ( content : "Done" )
    }
  }
}
