// ListGroup.mll - layout for the ListGroup.
//
//   Column [ list-group ]
//     For ( each: slot: items, as: item, index: i )
//       If ( when: i == selectedIndex )
//         HostButton [ list-group-item-selected ] (...)
//       Else
//         HostButton [ list-group-item ] (...)
//
// Each item lowers to a full-width HostButton row. Clicking fires
// onSelect with the row's index as the payload. The .msl styles
// the row to look like a flat selectable surface. selected-index
// remains the public kebab-case slot; the expression uses
// selectedIndex because Mosaic emitters expose kebab slots as
// backend-safe camel/Pascal identifiers in predicates.
//
// Why HostButton per row instead of a Box + dispatcher?
// -----------------------------------------------------
// HostButton already provides the click + a11y wiring on every
// backend natively (button role, focus ring, Enter activation,
// etc.). Re-implementing those affordances on a Box would defeat
// the kernel's point. The .msl flat-styles the chrome away.

layout ListGroup {
  Column [ list-group ] {
    For ( each: slot: items , as: item , index: i ) {
      If ( when: i == selectedIndex ) {
        HostButton [ list-group-item-selected ] (
          label : item ,
          onClick : emit: onSelect
        )
      }
      Else {
        HostButton [ list-group-item ] (
          label : item ,
          onClick : emit: onSelect
        )
      }
    }
  }
}
