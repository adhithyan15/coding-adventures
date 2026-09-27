### Added - native accessible dynamic tables

Canonical UI31 tables now lower dynamic header, row, and cell `For` loops to
Flutter's native `DataTable`, `DataColumn`, `DataRow`, and `DataCell` widgets.
This restores the platform table semantics that the former nested
`Column`/`Row` visual fallback could not expose while preserving editable cell
content, event dispatch, stable row keys, authored cell styling, and RTL.
Unsupported table shapes deliberately retain the visual fallback so strict
capability analysis continues to report them rather than making a false
accessibility claim. The generated Flutter floor is now 3.32, the first stable
release with explicit table/row/cell semantic roles; generated runtime helpers
are also analyzer-clean on Dart 3.12.

