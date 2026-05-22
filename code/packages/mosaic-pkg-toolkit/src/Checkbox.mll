// Checkbox.mll — layout for the Checkbox.
//
//   Row [ checkbox ]
//     If (when: slot: checked) {
//       HostButton [ checkbox-box-checked ]  ← styled square with check
//     } Else {
//       HostButton [ checkbox-box-unchecked ]  ← styled empty square
//     }
//     Text [ checkbox-label ]
//
// The two HostButton instances allow the .msl to style each state
// independently (different background, border, glyph). Both fire the
// same `onChange` emit on click — the host inverts its slot value.
//
// Why two HostButtons via If/Else instead of one with a slot-bound label?
// ----------------------------------------------------------------------
// A single button with `label : "✓" if checked else ""` would work
// IF the kernel supported expressions in label props. It accepts a
// SlotRef OR a literal string, but not a conditional expression
// directly. The If/Else block produces two distinct nodes that the
// backend lowers into a runtime visibility-toggle (XAML's
// BoolToVisibilityConverter, React's `cond ? ... : ...`).

layout Checkbox {
  Row [ checkbox ] {
    If ( when: slot: checked ) {
      HostButton [ checkbox-box-checked ] (
        label : "✓" ,
        disabled : slot: disabled ,
        onClick : emit: onChange
      )
    }
    Else {
      HostButton [ checkbox-box-unchecked ] (
        label : "" ,
        disabled : slot: disabled ,
        onClick : emit: onChange
      )
    }
    Text [ checkbox-label ] (
      content : slot: label
    )
  }
}
