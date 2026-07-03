// Select.mll — layout for the Select (v0.11).
//
//   Column [ select ]
//     HostButton [ select-toggle ] (
//       label: slot: value,
//       disabled: slot: disabled,
//       onClick: emit: onToggle
//     )
//     If (when: slot: open) {
//       Column [ select-options ]
//         For (each: slot: options, as: option, index: i)
//           HostButton [ select-option ] (label: option, onClick: emit: onChange)
//     }
//
// The toggle's label is always `value`. When value is empty, the
// host should pass the placeholder text in via `value` to display
// the "Choose…" hint. v0.11 deliberately doesn't branch on value-
// truthiness inside the .mll — mosstyle's part-name scoping treats
// two HostButton[select-toggle] in different If/Else branches as a
// duplicate part name, which the moslayout compiler rejects.
// Cleanest is to push the choice into the host.
//
// Each select-option button's `onClick` routes to the `onChange`
// emit — the chosen `option` text becomes the payload via the
// For-loop's per-iteration variable binding. This mirrors how
// Pagination's per-page button passes the page label.

layout Select {
  Column [ select ] {
    HostButton [ select-toggle ] (
      label : slot: value ,
      disabled : slot: disabled ,
      onClick : emit: onToggle
    )
    If ( when: slot: open ) {
      Column [ select-options ] {
        For ( each: slot: options , as: option , index: i ) {
          HostButton [ select-option ] (
            label : option ,
            onClick : emit: onChange
          )
        }
      }
    }
  }
}
