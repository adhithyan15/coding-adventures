// Nav.mll — layout for the horizontal nav.
//
//   Row [ nav ]
//     For (each: slot: items, as: item, index: i)
//       HostButton [ nav-link ] (label: item, onClick: emit: onSelect)
//
// Structurally identical to ListGroup but laid out horizontally
// via Row. Each item is a HostButton so a11y wiring (focus,
// keyboard activation) comes from the kernel for free; .msl
// flat-styles the chrome.

layout Nav {
  Row [ nav ] {
    For ( each: slot: items , as: item , index: i ) {
      HostButton [ nav-link ] (
        label : item ,
        onClick : emit: onSelect
      )
    }
  }
}
