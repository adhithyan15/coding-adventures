// DropdownMenu.mll — layout for the DropdownMenu (v0.9).
//
//   Column [ dropdown ]
//     HostButton [ dropdown-toggle ] (label: slot: label, onClick: emit: onToggle)
//     If ( when: slot: open ) {
//       Column [ dropdown-menu ]
//         For (each: slot: items, as: item, index: i)
//           HostButton [ dropdown-item ] (label: item, onClick: emit: onSelect)
//     }
//
// Closed state: only the toggle button is in the tree (the `If`
// collapses to nothing on every backend). The menu being a vertical
// `Column` of HostButtons matches the ListGroup pattern.
//
// Positioning: v0.9 renders the menu inline beneath the toggle
// (i.e. the dropdown takes vertical space when open and pushes
// downstream content down). Bootstrap's absolute-positioned overlay
// effect needs a mosstyle z-index/position story which the kernel
// doesn't fully expose yet; a future PR can layer that on.

layout DropdownMenu {
  Column [ dropdown ] {
    HostButton [ dropdown-toggle ] (
      label : slot: label ,
      onClick : emit: onToggle
    )
    If ( when: slot: open ) {
      Column [ dropdown-menu ] {
        For ( each: slot: items , as: item , index: i ) {
          HostButton [ dropdown-item ] (
            label : item ,
            onClick : emit: onSelect
          )
        }
      }
    }
  }
}
