// FormulaBar.touch.mll — touch / mobile layout for the formula bar (UI30).
//
// On a phone-sized viewport the desktop's "address-label sits to the LEFT
// of the formula field" arrangement doesn't fit — there's not enough
// horizontal room for both a recognisable cell address and a usable
// formula input. The touch variant stacks them vertically instead:
//
//   Column [bar]
//     Text [address-label] (content: slot: cell-address)
//     HostInput [formula-field] (... full-width, larger tap target via .msl)
//
// The interface (FormulaBar.mil) is unchanged — same slots, same emits.
// Only the spatial arrangement differs. This is the UI30 invariant in
// action: one component, many layouts, identical host contract.
//
// What the touch variant changes vs. .desktop.mll:
//
//   1. Row [bar] → Column [bar]
//      Vertical stack instead of horizontal row. The address label
//      sits ABOVE the formula field, giving the input full width.
//   2. (everything else identical)
//      The slot bindings and emit wiring are byte-for-byte the same.
//      Tap-target sizing (the Apple-HIG 44 px minimum, etc.) belongs
//      to the .msl — the layout just establishes the spatial shape.
//
// Why HostInput (not Input): consistency with the desktop variant
// post-Phase-1 — both variants use the kernel-canonical UI29 §2.1
// primitive so the React/Flutter/etc. emitters lower them identically.

layout FormulaBar {
  Column [bar] {
    Text [address-label] (content: slot: cell-address)
    HostInput [formula-field] (
      value:       slot: formula,
      read-only:   slot: read-only,
      placeholder: "Enter formula",
      onChange:    emit: onFormulaChange,
      onCommit:    emit: onCommit,
      onCancel:    emit: onCancel
    )
  }
}
