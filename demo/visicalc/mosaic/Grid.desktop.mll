// Grid.desktop.mll — desktop layout for the spreadsheet grid.
//
// Pass-through layout: the only primitive used is the built-in Grid
// (per UI26 §3.1). Every slot is forwarded by reference and the
// onNavigate event is wired straight through to the mosmodel emit.

layout Grid {
  Grid [sheet] (
    headers:      slot: column-headers,
    rows:         slot: viewport-rows,
    selected-row: slot: selected-row,
    selected-col: slot: selected-col,
    edit-row:     slot: edit-row,
    edit-col:     slot: edit-col,
    onNavigate:   emit: onNavigate
  )
}
