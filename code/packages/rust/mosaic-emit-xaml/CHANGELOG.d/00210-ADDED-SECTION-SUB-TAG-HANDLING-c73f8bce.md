### Added — Section sub-tag handling

- **`HostTableColGroup`** — recognised but ignored in PR-4 (the
  column-widths layout question per spec §5.2 needs more design).
- **`HostTableHead`** — emits as `<StackPanel Grid.Row="N" Orientation="Vertical">`
  containing the header row(s). Auto-sized row.
- **`HostTableBody`** — emits inside `<ScrollViewer Grid.Row="N" VerticalScrollBarVisibility="Auto">`
  for vertical overflow. `*`-sized row (fills remaining space).
- **`HostTableFoot`** — same shape as Head but at the last Grid.Row.
  Auto-sized.

Each section's `Row` children become `<StackPanel Orientation="Horizontal">` (via the existing `emit_stack_panel` reused from PR-1).
Sections also accept `For` and `If` children so authors can iterate /
conditionally include rows. Any other child of a section is an
`UnsupportedPrimitive` error.

