// ButtonGroup.mll — layout for the ButtonGroup.
//
//   Row [ button-group ]
//     For (each: slot: items, as: item, index: i)
//       HostButton [ button-group-item ] (label: item, onClick: emit: onSelect)
//
// Visually, the buttons share a border and only the outer corners
// are rounded. That's the .msl's job — the .mll is identical in
// shape to Nav, only the part names differ.

layout ButtonGroup {
  Row [ button-group ] {
    For ( each: slot: items , as: item , index: i ) {
      HostButton [ button-group-item ] (
        label : item ,
        onClick : emit: onSelect
      )
    }
  }
}
