// Grid.touch.mll — touch / mobile layout for the VisiCalc grid (UI30).
//
// UI28-1 / U29-D1 rewrite — see Grid.desktop.mll's top comment for the
// full composition rationale.  The touch layout shares the same shape
// as the desktop layout in v0.2.0: HostTable + HostTableColGroup +
// HostTableHead + HostTableBody + nested For + inline cell with
// HostInput / Text under an If(when: predicate).
//
// Why identical to desktop?
// -------------------------
//
// UI30 §1: "different layouts ... keeping the interface mostly the
// same."  The desktop and touch layouts can DIFFER (touch might want
// larger cells, different scroll behaviour, etc.) but neither
// requires diverging primitives.  v0.2.0 establishes the kernel-
// only composition; layout-specific tweaks (cell padding, touch-
// target sizing) belong in the .msl files, not here.
//
// When the touch layout actually needs to render differently from
// desktop — for example, swiping a row to dismiss, or a long-press-
// to-edit gesture instead of double-click — this file diverges.
// Until then, the layouts are isomorphic and the difference shows up
// in Grid.dark.msl + a future Grid.touch.msl.

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
            Box [ cell ] (
              // Task #35 — same state-when toggles as desktop. See
              // Grid.desktop.mll for the rationale.
              state-when-selected: ( r == selectedRow && c == selectedCol ) ,
              state-when-editing:  ( r == editRow && c == editCol )
            ) {
              If ( when: ( r == editRow && c == editCol ) ) {
                // Mirrors Grid.desktop.mll — see that file for the
                // edit-content/onFormulaChange rationale.
                HostInput (
                  value:    slot: edit-content ,
                  onChange: emit: onFormulaChange ,
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
