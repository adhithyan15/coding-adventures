// Sheet.mll — layout for the filterable/sortable/editable spreadsheet
// view.
//
// Composition (mosaic-pkg-toolkit's Select + kernel primitives +
// mosaic-pkg-grid's Grid — no primitives of Sheet's own):
//
//   Column [ sheet-root ]
//     Row [ sheet-toolbar ]
//       HostInput [ sheet-filter ] (filter box)
//       pkg::mosaic-pkg-toolkit::Select (sort field — no call-site part
//       name, same reasoning as Grid.mll's own Cell call: it would
//       shadow Select's own styled root part)
//       HostButton [ sheet-sort-dir-asc | sheet-sort-dir-desc ] (direction)
//     pkg::mosaic-pkg-grid::Grid ( ...pass-through... )
//
// Emit forwarding across a package boundary
// ------------------------------------------
//
// `pkg::mosaic-pkg-grid::Grid ( onNavigate: emit: onNavigate, ... )`
// forwards Grid's own onNavigate to Sheet's declared onNavigate — the
// identical mechanism mosaic-pkg-grid's own Grid.mll already uses to
// forward its userland Cell's onClick/onChange/onCommit/onCancel up to
// Grid's onNavigate/onFormulaChange/onEditCommit/onEditCancel (see
// Grid.mll). Composition resolves the same way whether the child is a
// same-package sibling or a `pkg::`-qualified cross-package reference.
// This wiring must stay in place even though nothing meaningful flows
// through it yet — omitting it breaks package resolution outright
// (Grid's internal `Cell(onClick: emit: onNavigate)` needs SOMETHING
// to bind to). See Sheet.mil's "Known limitation" note for why these
// are declared void rather than with Grid.mil's own payload shape.
//
// Direction toggle: two branches, not one button with computed content
// -----------------------------------------------------------------
//
// Two uniquely-named HostButtons in an If/Else — one per direction —
// rather than a single button whose label is computed from
// sort-ascending. This is the same "duplicate part per branch" shape
// task-app's own segmented view switch already uses (see
// TaskApp.mll's seg-list-off / seg-list-on): mosstyle scopes each
// button's hover/pressed styling by its OWN part name, so the two
// directions can carry distinct icons/tooltips later without a
// conditional style expression.

layout Sheet {
  Column [ sheet-root ] {
    Row [ sheet-toolbar ] {
      HostInput [ sheet-filter ] (
        value : slot: filter-text ,
        placeholder : "Filter…" ,
        onChange : emit: onFilterChange
      )
      pkg::mosaic-pkg-toolkit::Select (
        value : slot: sort-field ,
        options : slot: sort-options ,
        placeholder : "Sort by…" ,
        open : slot: sort-open ,
        disabled : false ,
        onToggle : emit: onToggleSortOpen ,
        onChange : emit: onSortFieldChange
      )
      If ( when: slot: sort-ascending ) {
        HostButton [ sheet-sort-dir-asc ] ( label : "↑" , onClick : emit: onToggleSortDirection )
      }
      Else {
        HostButton [ sheet-sort-dir-desc ] ( label : "↓" , onClick : emit: onToggleSortDirection )
      }
    }
    pkg::mosaic-pkg-grid::Grid (
      viewport-rows : slot: viewport-rows ,
      column-headers : slot: column-headers ,
      column-widths : slot: column-widths ,
      selected-row : slot: selected-row ,
      selected-col : slot: selected-col ,
      edit-row : slot: edit-row ,
      edit-col : slot: edit-col ,
      edit-content : slot: edit-content ,
      onNavigate : emit: onNavigate ,
      onFormulaChange : emit: onFormulaChange ,
      onEditCommit : emit: onEditCommit ,
      onEditCancel : emit: onEditCancel
    )
  }
}
