### Known limitations carried to later PRs

- **HostTableColGroup column-widths layout** — the spec §5.2 caveat
  about WinUI 3's lack of a native semantic-table control means
  column widths need either explicit `Grid.ColumnDefinitions` or
  per-cell `Width` settings. PR-4 emits the StackPanel-per-row
  structure but doesn't yet propagate column widths; the
  ColGroup sub-tag is recognised and ignored. A follow-up tackles
  this together with the `--use-community-datagrid` flag.
- **`--use-community-datagrid` flag** — exists on `EmitOptions` but
  not yet acted on. When set, future PR will switch the lowering to
  `<controls:DataGrid>` from CommunityToolkit.WinUI for full UIA
  fidelity (spec §5.3 caveat).
- **Component references** still `UnsupportedPrimitive` pending PR-5.

## [Unreleased] — PR-3 — HostInput / HostButton / HostScroll

