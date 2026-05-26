// Grid.desktop.mll — desktop layout for the VisiCalc spreadsheet grid.
//
// UI28-1 / U29-D1 rewrite — replaces the L10 "degraded HostTable"
// composition with the v0.2.0 cell-and-column shape pioneered by
// mosaic-pkg-grid v0.2.0 (see code/packages/mosaic-pkg-grid).  The
// VisiCalc demo cannot yet `import` Grid from that package cross-
// repo because mosaic-compile's package-resolver isn't wired through
// the demo's build script — so the composition is inlined here.  When
// the package-resolver lands, this whole file collapses to:
//
//   layout Grid {
//     pkg::mosaic-pkg-grid::Grid (
//       viewport-rows:  slot: viewport-rows ,
//       column-headers: slot: column-headers ,
//       column-widths:  slot: column-widths ,
//       selected-row:   slot: selected-row ,
//       selected-col:   slot: selected-col ,
//       edit-row:       slot: edit-row ,
//       edit-col:       slot: edit-col ,
//       edit-content:   slot: edit-content ,
//       onNavigate:     emit: onNavigate ,
//       onEditCommit:   emit: onEditCommit ,
//       onEditCancel:   emit: onEditCancel
//     )
//   }
//
// Features recovered vs the L10 degraded version
// ----------------------------------------------
//
//   1. Per-cell click — every body Cell's HostInput / Text sits inside
//      a Box [cell] in a Row that dispatches `onNavigate(r, c)`.
//   2. Selection highlight — sub-part styling (`sheet/cell:selected`)
//      fires when `r == selectedRow && c == selectedCol`.  The
//      predicate value is computed at the cell call site via
//      expression-in-slot-binding (UI29 §3.3).
//   3. Inline editing — `If (when: r == editRow && c == editCol)`
//      renders a HostInput in place of the Text leaf.  `onCommit` /
//      `onCancel` forward to the App's reducer.
//   4. Per-column widths — `HostTableColGroup` with
//      `For (each: slot: column-widths, as: w)` emits one <col width>
//      per column.
//   5. Stable React keys — every For binds an explicit `index:`.
//      mosaic-emit-react auto-emits `<React.Fragment key={r}>` per
//      iteration (UI28-1 §6.3 / PR #4396), so the diff cost stays
//      O(1) per render for the VisiCalc-scale 100×26 grid.
//
// Still NOT in v0.2.0 (deferred to UI28-2)
// ----------------------------------------
//
//   - Sticky header (per UI28-1 §2 constraint 5).  The header
//     scrolls away with the body.
//
// Composition (kernel primitives + HostInput + If/Else only — no
// per-backend special cases)
// --------------------------------------------------------------
//
//   HostTable [ sheet ]                  ← semantic data table
//     HostTableColGroup                  ← <colgroup>
//       For ( each: slot: column-widths, as: w, index: cw )
//         Col [ col ] ( width: ( w ) )
//     HostTableHead                      ← <thead>
//       Row [ header-row ]               ← <tr>
//         For ( each: slot: column-headers, as: h, index: ch )
//           Box [ header-cell ]          ← <th>
//             Text ( content: ( h ) )
//     HostTableBody                      ← <tbody>
//       For ( each: slot: viewport-rows, as: row, index: r )
//         Row [ data-row ]               ← <tr>
//           For ( each: row, as: v, index: c )    ← UI29 §3.4
//             Box [ cell ]               ← <td>, styled by part `sheet/cell`
//               If ( when: ( r == editRow && c == editCol ) )
//                 HostInput ( value: ( v ),
//                             onCommit: emit: onEditCommit,
//                             onCancel: emit: onEditCancel )
//               Else
//                 Text ( content: ( v ) )

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
            Box [ cell ] {
              If ( when: ( r == editRow && c == editCol ) ) {
                HostInput (
                  value:    ( v ) ,
                  onCommit: emit: onEditCommit ,
                  onCancel: emit: onEditCancel
                )
              }
              Else {
                Text ( content: ( v ) )
              }
            }
          }
        }
      }
    }
  }
}
