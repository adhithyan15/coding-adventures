layout VisiCalc {
  Column [workbook] {
    Row [toolbar] {
      Column [brand] {
        Text [eyebrow] (font-size: slot: font-eyebrow, content: "THE WORKBOOK")
        Text [title] (font-size: slot: font-title, content: "VisiCalc", a11y-role: heading)
      }
      Text [subtitle] (font-size: slot: font-body, content: "A little room for big ideas.")
    }
    Row [file-toolbar] {
      HostButton [new-button] (font-size: slot: font-body, label: "New workbook", onClick: emit: onNewWorkbook)
      HostButton [open-button] (font-size: slot: font-body, label: "Open", onClick: emit: onOpenWorkbook)
      HostButton [save-button] (font-size: slot: font-body, label: "Save", onClick: emit: onSaveWorkbook)
      Text [file-status] (font-size: slot: font-caption, content: slot: file-status)
    }
    Row [text-toolbar] {
      HostButton [smaller-text] (label: "A−", a11y-label: "Smaller text", font-size: slot: font-body, disabled: slot: smaller-text-disabled, onClick: emit: onSmallerText)
      Text [text-scale-label] (content: slot: text-scale-label, font-size: slot: font-caption)
      HostButton [larger-text] (label: "A+", a11y-label: "Larger text", font-size: slot: font-body, disabled: slot: larger-text-disabled, onClick: emit: onLargerText)
    }
    Row [formula-bar] {
      Text [address-label] (font-size: slot: font-input, content: slot: cell-address)
      Text [formula-symbol] (font-size: slot: font-symbol, content: "fx")
      HostInput [formula-field] (font-size: slot: font-input,
        value: slot: formula, read-only: slot: read-only,
        placeholder: "Enter a value or formula",
        onChange: emit: onFormulaChange,
        onCommit: emit: onCommit, onCancel: emit: onCancel
      )
    }
    Row [sheet-toolbar] {
      Text [sheet-name] (font-size: slot: font-body, content: "Sheet 1")
      Text [sheet-hint] (font-size: slot: font-caption, content: "Enter to apply · Esc to cancel")
    }
    If (when: slot: workbook-empty) {
      Column [empty-introduction] {
        Text [empty-heading] (font-size: slot: font-introduction, content: "Room for your next idea", a11y-role: heading)
        Text [empty-hint] (font-size: slot: font-body, content: "Choose a cell and type a number, a note, or a formula like =2+3. Press Enter to keep it.")
      }
    }
    HostScroll [sheet-frame] ( axis: both ) {
      pkg::mosaic-pkg-grid::RowHeaderGrid (row-height: slot: row-height,
      font-size: slot: font-body, row-font-size: slot: font-row,
      viewport-offset: slot: viewport-offset,
      total-rows: slot: total-rows,
      onViewportShift: emit: onViewportShift,
      row-headers: slot: row-headers,
      viewport-rows: slot: viewport-rows,
      column-headers: slot: column-headers,
      column-widths: slot: column-widths,
      selected-row: slot: grid-selected-row,
      selected-col: slot: selected-col,
      edit-row: slot: grid-edit-row,
      edit-col: slot: edit-col,
      edit-content: slot: edit-content,
      onNavigate: emit: onGridNavigate,
      onViewportRows: emit: onViewportRows,
      onFormulaChange: emit: onFormulaChange,
      onEditCommit: emit: onEditCommit,
      onEditCancel: emit: onEditCancel
      )
    }
    Text [selection-summary] (font-size: slot: font-caption, content: slot: selection-summary)
  }
}
