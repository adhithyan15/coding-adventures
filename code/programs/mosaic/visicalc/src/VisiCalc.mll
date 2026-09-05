layout VisiCalc {
  Column [workbook] {
    Row [toolbar] {
      Text [title] (content: "VisiCalc", a11y-role: heading)
      Text [subtitle] (content: "A little room for big ideas")
      HostButton [new-button] (label: "New workbook", onClick: emit: onNewWorkbook)
    }
    Row [formula-bar] {
      Text [address-label] (content: slot: cell-address)
      Text [formula-symbol] (content: "fx")
      HostInput [formula-field] (
        value: slot: formula, read-only: slot: read-only,
        placeholder: "Enter a value or formula",
        onChange: emit: onFormulaChange,
        onCommit: emit: onCommit, onCancel: emit: onCancel
      )
    }
    pkg::mosaic-pkg-grid::Grid (
      viewport-rows: slot: viewport-rows,
      column-headers: slot: column-headers,
      column-widths: slot: column-widths,
      selected-row: slot: grid-selected-row,
      selected-col: slot: selected-col,
      edit-row: slot: grid-edit-row,
      edit-col: slot: edit-col,
      edit-content: slot: edit-content,
      onNavigate: emit: onGridNavigate,
      onFormulaChange: emit: onFormulaChange,
      onEditCommit: emit: onEditCommit,
      onEditCancel: emit: onEditCancel
    )
  }
}
