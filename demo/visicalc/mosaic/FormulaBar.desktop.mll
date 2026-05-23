// FormulaBar.desktop.mll — desktop layout for the formula bar.
//
// Row containing the cell address label and the editable HostInput
// field. Migrated from the legacy `Input` primitive (UI25) to
// `HostInput` (UI29 §2.1, kernel primitive #4) per the VisiCalc
// Phase 1 plan — same value/readOnly/placeholder/event wiring, just
// the kernel-canonical name.
//
// HostInput handles onChange / onCommit / onCancel natively:
// onChange wraps `e.target.value` as the payload, and onCommit +
// onCancel merge into a single onKeyDown handler keyed on Enter /
// Escape — identical generated React shape to the legacy `Input`
// primitive (per the UI29 §2.1 migration note in the React
// emitter docs).

layout FormulaBar {
  Row [bar] {
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
