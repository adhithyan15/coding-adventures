// FormulaBar.desktop.mll — desktop layout for the formula bar.
//
// Row containing the cell address label and the editable Input field.
// The Input primitive (UI25) handles onChange / onCommit / onCancel
// natively: onChange wraps `e.target.value` as the payload, and
// onCommit + onCancel merge into a single onKeyDown handler keyed on
// Enter / Escape.

layout FormulaBar {
  Row [bar] {
    Text [address-label] (content: slot: cell-address)
    Input [formula-field] (
      value:      slot: formula,
      read-only:  slot: read-only,
      onChange:   emit: onFormulaChange,
      onCommit:   emit: onCommit,
      onCancel:   emit: onCancel
    )
  }
}
