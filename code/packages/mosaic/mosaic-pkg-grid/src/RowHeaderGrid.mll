// An opt-in numbered-row Grid. Data coordinates are unchanged by the header.
layout RowHeaderGrid {
  HostTable [sheet] (focusable: true, a11y-label: "Data table", selected-row: slot: selected-row, selected-col: slot: selected-col, onViewportRows: emit: onViewportRows, viewport-offset: slot: viewport-offset, total-rows: slot: total-rows, onViewportShift: emit: onViewportShift) {
    HostTableColGroup {
      Col (width: 48)
      For (each: slot: column-widths, as: w, index: cw) { Col (width: (w)) }
    }
    HostTableHead [column-headings] {
      Row [header-row] {
        // Wrapped for the same reason the header cell below is (#14830): the
        // colgroup's leading `Col (width: 48)` is threaded into the cell's
        // modifier chain, and only a CONTAINER body receives it. As a bare
        // Text this measured `size=0 x 24` at origin (0, 0) -- present in the
        // semantics tree, correctly named, occupying no space, every
        // accessibility gate passing on it.
        Box [row-corner] (table-cell-role: corner) {
          Text [row-corner-text] (content: "")
        }
        For (each: slot: column-headers, as: h, index: ch) {
          // The Box is load-bearing, not decoration (#14829). A HostTable's
          // per-column width is threaded into the For body's modifier chain,
          // and only a CONTAINER body receives it -- a bare Text leaf sizes to
          // its own glyph. With the Text directly here, five headers spanned
          // 45px while their five data columns spanned 338px, so no header sat
          // above its column.
          //
          // This also restores the shape Grid.mll documents for the same row:
          // `Box [ header-cell ] <- <th>` wrapping a Text, matching how
          // `data-cell` below is built.
          Box [header-cell] (table-cell-role: column-header) {
            Text [header-cell-text] (content: (h))
          }
        }
      }
    }
    HostTableBody {
      For (each: slot: viewport-rows, as: row, index: r) {
        Row [data-row] {
          // Same wrap, same reason (#14829): the row-heading column is the
          // one the leading static `Col` describes, and a bare Text sizes to
          // its own glyph rather than to the authored column width.
          Box [row-heading] (table-cell-role: row-header) {
            Text [row-heading-text] (content: (rowHeaders[r]))
          }
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
