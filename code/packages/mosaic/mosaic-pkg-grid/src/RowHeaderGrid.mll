// An opt-in numbered-row Grid. Data coordinates are unchanged by the header.
layout RowHeaderGrid {
  HostTable [sheet] (selected-row: slot: selected-row, selected-col: slot: selected-col, onViewportRows: emit: onViewportRows, viewport-offset: slot: viewport-offset, total-rows: slot: total-rows, onViewportShift: emit: onViewportShift) {
    HostTableColGroup {
      Col (width: 48)
      For (each: slot: column-widths, as: w, index: cw) { Col (width: (w)) }
    }
    HostTableHead [column-headings] {
      Row [header-row] {
        Text [row-corner] (content: "", table-cell-role: corner)
        For (each: slot: column-headers, as: h, index: ch) {
          Text [header-cell] (content: (h), table-cell-role: column-header)
        }
      }
    }
    HostTableBody {
      For (each: slot: viewport-rows, as: row, index: r) {
        Row [data-row] {
          Text [row-heading] (content: (rowHeaders[r]), table-cell-role: row-header)
          For (each: row, as: v, index: c) {
            Box [data-cell] (table-cell-role: data) {
              Cell (
                value: (v), row: (r), col: (c), edit-content: slot: edit-content,
                is-editing: (r == editRow && c == editCol),
                is-selected: (r == selectedRow && c == selectedCol),
                editable: true, alignment: "left", cell-type: "text",
                onClick: emit: onNavigate, onChange: emit: onFormulaChange,
                onCommit: emit: onEditCommit, onCancel: emit: onEditCancel
              )
            }
          }
        }
      }
    }
  }
}
