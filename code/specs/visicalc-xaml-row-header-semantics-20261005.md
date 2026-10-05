# XAML native semantics for numbered-row tables

Related: #14278, #14388, #14276, epic #14267.

RowHeaderGrid currently misses the canonical native table recognizer because
each row has a leading semantic header before its repeated data cells. Recognize
only paired, explicitly marked corner/row-header prefixes, retaining all indexed
loop and source checks. Unmarked/asymmetric extra children remain unsupported.

Reuse native table and cell automation peers without shifting logical data
columns. Emit row-header peers and connect them through ITableProvider.GetRowHeaders
and ITableItemProvider.GetRowHeaderItems; authored row label text supplies names.
Keep normal grids unchanged. Preserve authored widths, click events and state
hosts, and validate generated Windows markup plus actual interaction/UIA output.
This slice does not claim complete keyboard/editing or native-complete acceptance.

## Validation on Windows, 2026-10-05

- Emitter suite: 367 unit/integration tests passed; one ignored doctest.
- Generated VisiCalc from shared Mosaic sources and the existing Rust runtime
  DLL. Native x64 WinUI build succeeded with zero errors, 22 warnings (binding
  notification warnings and the existing native cell reference comparison).
- Actual Windows UI Automation reports `table sheet table` and named column
  header items A through Z. Its bounded tree truncates the body, so this is not
  a claim that every row-header association was inspected through UIA.
- Launch exposed a header-width regression in the newly selected native path;
  sharing the existing ColGroup width projection fixed it. Screenshot showed
  matching 80-pixel header/data spacing and the restored F6 highlight.
- Clicking B2 selected B2, displayed `=6*7`, highlighted that cell, and reported
  `B2, 42, formula =6*7` with runtime status `handled onGridNavigate`.
- A Tab probe retained formula-field focus. Full keyboard navigation and
  screen-reader traversal remain acceptance work under #14278/#14388.

Generated project, fixture, and native build log are retained locally at
`C:/Users/adhit/worktrees/visicalc-table-semantics-20261005-output/`.
