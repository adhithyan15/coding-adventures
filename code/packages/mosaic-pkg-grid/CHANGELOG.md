# Changelog

All notable changes to `mosaic-pkg-grid` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## 0.1.0 — 2026-05-19

Initial release. Exports Grid, Cell, Column. Built on UI29 kernel
primitives (Box, Row, Text, HostInput, HostTable + sub-tags, If, For).

### Added

- `mosaic-package.toml` manifest declaring `[components].exports =
  ["Grid", "Cell", "Column"]` and targeting UI29 kernel version `"1"`.
- `Cell.mil` / `Cell.mll` / `Cell.dark.msl`: an editable spreadsheet
  cell composed of `Box` + `If`/`Else` + `HostInput` + `Text`.
- `Column.mil` / `Column.mll`: a metadata-only column declaration with
  a single hollow `Box` marker layout (moslayout currently requires a
  single root node per component).
- `Grid.mil` / `Grid.mll` / `Grid.dark.msl`: the data grid itself,
  composed of `HostTable` + `HostTableHead` + `HostTableBody` + `Row` +
  `For` + the package's own `Cell` component.
- `tests/package_compiles.rs`: integration smoke test that round-trips
  every source file through `mosmodel-compiler`, `moslayout-compiler`,
  and `mosstyle-compiler`.

### Known limitations

- **Header row is empty in v0.1.0.**  Rendering one header cell per
  declared Column requires the Grid interface to accept a `columns`
  list slot (v0.2.0).
- **Body rows show `row` only.**  Full per-column iteration needs the
  same `columns` slot plus the expression-grammar `row[c]` field-access
  syntax defined in UI29 §3.3 but not yet landed (UI29-G3).
- **Backend lowerings still landing.**  `For`, `If`, `HostInput`, and
  `HostTable` (with `HostTableHead` / `HostTableBody` / `Row` slots) are
  kernel primitives whose per-backend lowerings are in flight under
  `U29-K-react`, `U29-K-swiftui`, `U29-K-qt`, `U29-K-webcomp`, and
  `U29-K-html`.  The smoke test asserts language-frontend correctness
  (parse + analyze + validate at the .mil/.mll/.msl layer); end-to-end
  backend artifact generation lights up once those PRs land.
- **No theming cascade.**  The package ships `dark.msl` only; light-mode
  styles and host overrides arrive once the multi-theme cascade lands
  in mosstyle.

The architecture (manifest-driven, kernel-primitive-only composition,
backend-agnostic) is what v0.1.0 proves.  v0.2.0 fills in the Grid
behaviour once the supporting grammar / resolver / kernel-coverage PRs
land.
