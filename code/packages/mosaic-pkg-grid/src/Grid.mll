// Grid.mll — layout for the spreadsheet-style data grid.
//
// Composition (kernel primitives only):
//
//   HostTable [sheet]                  ← semantic data table (UI29 §2.1)
//     HostTableHead
//       Row                            ← header row (intentionally empty
//                                        in v0.1.0 — see note below)
//     HostTableBody
//       For (each: slot: viewport-rows, as: row, index: r)
//         Row [data-row]
//           Cell [body] ( value: row, ... )
//
// Why this shape?  HostTable carries the screen-reader-accessible
// `<table role="grid">` semantics that "div-soup" cannot provide.
// HostTableHead / HostTableBody select the `<thead>` / `<tbody>`
// children.  Row maps to `<tr>` (or SwiftUI's row builder / Qt's row
// delegate).  For iterates the data slot; Cell is the userland Cell
// component from this same package.
//
// v0.1.0 caveats (intentional — full fix in v0.2.0)
// -------------------------------------------------
//   1. Header row is left empty.  Rendering one `<th>` per declared
//      Column requires Grid's interface to accept a `columns` list slot
//      so a `For (each: slot: columns, ...)` can drive the header — out
//      of scope for v0.1.0.
//   2. Body emits ONE Cell per row, displaying `row` (which here
//      resolves to a NAME-valued prop — Keyword("row") — picked up by
//      the resolver as the For-bound iteration variable).  Full
//      per-column iteration also needs the `columns` slot.
//
// IMPORTANT (UI29-P1, v0.1.0): `For`, `HostTable`, `HostTableHead`,
// `HostTableBody`, `Row`, and `Cell` are all NAME tokens in the current
// moslayout grammar, so this file parses today.  Backend emitters lower
// them as their respective U29-K-* PRs land; until then the artifact
// builder may refuse Grid.  That is expected.

layout Grid {
  HostTable [ sheet ] {
    HostTableHead {
      Row { }
    }
    HostTableBody {
      For ( each: slot: viewport-rows , as: row , index: r ) {
        Row [ data-row ] {
          Cell [ body ] (
            value:      row ,
            editable:   true ,
            is-editing: slot: edit-row ,
            onCommit:   emit: onEditCommit ,
            onCancel:   emit: onEditCancel ,
            onClick:    emit: onNavigate
          )
        }
      }
    }
  }
}
