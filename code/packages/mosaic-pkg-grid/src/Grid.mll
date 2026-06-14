// Grid.mll — layout for the spreadsheet-style data grid (v0.2.0).
//
// Composition (kernel primitives + this package's Cell only):
//
//   HostTable [sheet]                  ← semantic data table
//     HostTableColGroup                ← <colgroup> + <col> per column
//       For ( each: slot: column-widths, as: w, index: cw )
//         Col [ col ] ( width: ( w ) )
//     HostTableHead                    ← <thead>
//       Row [ header-row ]             ← <tr>
//         For ( each: slot: column-headers, as: h, index: ch )
//           Box [ header-cell ]        ← <th>
//             Text ( content: ( h ) )
//     HostTableBody                    ← <tbody>
//       For ( each: slot: viewport-rows, as: row, index: r )
//         Row [ data-row ]             ← <tr>
//           For ( each: row, as: v, index: c )    ← UI29 §3.4
//             Cell ( value: ( v ), is-editing: ( r == editRow && c == editCol ),
//                    is-selected: ( r == selectedRow && c == selectedCol ),
//                    editable: true, alignment: "left", cell-type: "text",
//                    onClick: emit: onNavigate, onCommit: emit: onEditCommit,
//                    onCancel: emit: onEditCancel )
//
// Why this shape
// --------------
//
// HostTable carries the screen-reader-accessible `<table role="grid">`
// semantics that "div-soup" cannot provide.  HostTableColGroup /
// HostTableHead / HostTableBody select the matching `<colgroup>` /
// `<thead>` / `<tbody>` children.  Col is the cell-definition sub-tag
// inside `<colgroup>` (UI31 §3.2).  Row maps to `<tr>` (or SwiftUI's
// row builder, Qt's row delegate, Flutter's DataRow).  For iterates
// each data slot; Cell is the userland Cell component from this same
// package; HostInput inside Cell provides inline edit.
//
// What v0.2.0 finishes that v0.1.0 deferred
// ------------------------------------------
//
//   1. Header row now renders — `For (each: slot: column-headers, ...)`
//      drives one `<th>` per column.
//   2. Body now renders ALL cells per row via NESTED For — the inner
//      `For ( each: row, as: v, index: c )` uses UI29 §3.4 (For-binding
//      as the inner loop's iterable) which landed in PR #4398.  Each
//      Cell receives its (r, c)-determined `value`, plus the predicate-
//      computed `is-editing` / `is-selected` booleans.
//   3. Column widths reach the `<colgroup>` — the outer
//      `For (each: slot: column-widths, ...)` emits one `<col width>`
//      per column.  When omitted at the HTML / WebComponent layer, the
//      table auto-sizes; explicit widths give stable column widths
//      independent of content.
//
// Per-cell predicates: where does the comparison live?
// ----------------------------------------------------
//
// The host pushes the *coordinate* slots only (`edit-row`, `edit-col`,
// `selected-row`, `selected-col`).  Grid is the encapsulation boundary
// that turns those into per-Cell booleans.  The comparison is done
// using **expression-in-slot-binding** at the Cell call site:
//
//   is-editing:  ( r == editRow && c == editCol )
//   is-selected: ( r == selectedRow && c == selectedCol )
//
// The `(...)` grouping triggers the moslayout-compiler's Expr branch
// (UI29 §3.3), so the expression text passes verbatim into the target
// language.  At runtime:
//
//   - `r` and `c` are the .map / ForEach / Repeater loop variables
//     (the camelCased `as:` / `index:` bindings — single letters here
//     so the Expr text is target-language-clean without further
//     transformation).
//   - `editRow`, `editCol`, `selectedRow`, `selectedCol` are the
//     camelCased slot names — every backend's slot-reference lowering
//     produces these identifiers, so the Expr resolves correctly in
//     the target component's scope.
//
// Stable iteration keys (performance)
// -----------------------------------
//
// Every For binds an explicit `index:` (`cw`, `ch`, `r`, `c`) so each
// backend's For lowering threads it into the framework-native list
// key:
//
//   - React           — `key={r}` / `key={c}` on each <React.Fragment>
//                       (auto-emitted by mosaic-emit-react via UI28-1
//                       §6.3 / PR #4396).
//   - SwiftUI         — `ForEach(..., id: \.offset)` keyed by index.
//   - Flutter         — `KeyedSubtree(key: ValueKey(r), ...)`
//                       (UI28-1 §6.2 / PR #4393).
//   - Qt              — Repeater delegate's `property int index`.
//   - HTML / WebComp  — static-rendered, no key needed.
//   - XAML            — `ItemsRepeater` consumes the index via
//                       `x:Bind` on the row-VM property.
//
// This is the UI28-1 §5 performance property.  For VisiCalc-scale
// data (100 rows × 26 cols = 2,600 keyed cells) the React diff cost
// drops from O(n) to O(1) per render.
//
// What is NOT in v0.2.0 (deferred to UI28-2)
// ------------------------------------------
//
//   - Sticky header.  Per UI28-1 §2 constraint 5.  Authors compose
//     `HostScroll { Grid { ... } }` themselves or wait.
//   - Custom cell renderers (image, button, checkbox, sparkline).
//     v0.3.0 extends Cell's `cell-type` to switch.
//   - List virtualization INSIDE Grid (renders only visible rows of
//     a large viewport-rows).  Today, the host slices to viewport
//     before pushing — `viewport-rows` IS the visible window.
//   - Mosmodel record type so `columns: list<column-meta>` replaces
//     parallel-array `column-headers` + `column-widths`.

layout Grid {
  HostTable [ sheet ] {
    HostTableColGroup {
      For ( each: slot: column-widths , as: w , index: cw ) {
        Col [ col ] ( width: ( w ) )
      }
    }
    HostTableHead {
      Row [ header-row ] {
        For ( each: slot: column-headers , as: h , index: ch ) {
          Box [ header-cell ] {
            Text ( content: ( h ) )
          }
        }
      }
    }
    HostTableBody {
      For ( each: slot: viewport-rows , as: row , index: r ) {
        Row [ data-row ] {
          For ( each: row , as: v , index: c ) {
            // No call-site part name here.  Earlier drafts used
            // `Cell [body]` to give the call site its own
            // addressable part, but Grid.dark.msl never targets
            // `body` and the consumer-side part-name override
            // (UI34 §5.1) would then shadow Cell's own `cell` part,
            // breaking any consumer .msl that styles `cell`.
            // Dropping the label lets Cell.mll's root `Box [cell]`
            // flow through after resolution.
            Cell (
              value:        ( v ) ,
              // edit-content forwards the host's live edit buffer
              // (the same buffer the FormulaBar drives) into every
              // cell.  Only the cell with `is-editing: true` will
              // actually render a HostInput pointed at it; the
              // others receive the buffer but never display it.
              edit-content: slot: edit-content ,
              is-editing:   ( r == editRow && c == editCol ) ,
              is-selected:  ( r == selectedRow && c == selectedCol ) ,
              editable:     true ,
              alignment:    "left" ,
              cell-type:    "text" ,
              onClick:      emit: onNavigate ,
              // onChange propagates per-keystroke updates to the
              // host as `onFormulaChange` so the host's reducer can
              // update edit-content.  Without this round-trip the
              // controlled HostInput in Cell freezes — see Cell.mll
              // for the rationale.
              onChange:     emit: onFormulaChange ,
              onCommit:     emit: onEditCommit ,
              onCancel:     emit: onEditCancel
            )
          }
        }
      }
    }
  }
}
