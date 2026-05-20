# mosaic-pkg-grid

> Spreadsheet-style data grid built on UI29 kernel primitives.

This is the first userland Mosaic component package — the headline
deliverable of **UI29 (Primitive Kernel + Userland Component Packages)**.
It proves the architecture: the framework core ships only ~15 kernel
primitives, and useful components like Grid live *outside* the core, in
plain moslayout source files, the same shape as any user component.

## What this package exports

Three components, listed in `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Grid`   | the spreadsheet itself — header row + data rows | `Grid.mil` / `Grid.mll` / `Grid.dark.msl` |
| `Cell`   | one editable cell (read display ↔ edit input) | `Cell.mil` / `Cell.mll` / `Cell.dark.msl` |
| `Column` | declarative column metadata (no rendered output) | `Column.mil` / `Column.mll` |

Column ships no `.msl`: it produces no visible tree, so there is nothing
to style.

## How it fits in the stack

```
              ┌────────────────────────────────────────────┐
              │  Host application (.mll uses `Grid`)       │
              └─────────────────────┬──────────────────────┘
                                    │ component reference
                                    ▼
              ┌────────────────────────────────────────────┐
              │  mosaic-pkg-grid (this package)            │
              │  Grid → HostTable + For + Cell             │
              │  Cell → Box + If + HostInput + Text        │
              │  Column → metadata-only Box marker         │
              └─────────────────────┬──────────────────────┘
                                    │ kernel primitives only
                                    ▼
              ┌────────────────────────────────────────────┐
              │  UI29 kernel (15 primitives)               │
              │  Box / Row / Column / Stack / Text /       │
              │  Image / Spacer / Divider / Icon /         │
              │  If / For / HostInput / HostButton /       │
              │  HostTable / HostScroll                    │
              └─────────────────────┬──────────────────────┘
                                    │ per-backend lowering
                                    ▼
                  React / SwiftUI / Qt / WebComponent / HTML
```

A host that wants a Grid adds `mosaic-pkg-grid` to its package
dependencies; a host that doesn't, pays nothing.  A host that wants a
*richer* Grid can fork this package and publish their own — the same way
React's ecosystem treats data-grid libraries as userland.

## v0.1.0 architecture (what is proven)

* Three components, all manifest-declared.
* Three components, all composed entirely from kernel primitives — no
  bespoke per-backend code.
* Three components, all expressed in plain moslayout source files (the
  same `.mil` / `.mll` / `.msl` trio any user component uses).
* The smoke test at `tests/package_compiles.rs` round-trips every source
  file through its respective compiler crate.

## v0.2.0 caveats (deliberate scope cuts)

* **Header row is empty.**  `Grid.mll`'s `HostTableHead` contains a Row
  with no cells.  Rendering one `<th>` per declared Column requires
  Grid's interface to accept a `columns` list slot so a
  `For (each: slot: columns, as: col)` can drive the header.
* **Body emits ONE Cell per row.**  Each row shows `row` (the For-bound
  iteration variable resolved as a Keyword-valued prop), not `row[0]`,
  `row[1]`, ... — full per-column iteration needs the same `columns`
  slot, plus the expression-grammar `row[c]` field-access syntax that
  UI29 §3.3 defines but UI29-G3 has not yet landed.
* **No theming cascade.**  The package ships `dark.msl` only; light-mode
  styles and host overrides arrive once the multi-theme cascade lands
  in mosstyle.

These are not bugs.  The architecture (manifest-driven, kernel-
primitive-only composition, backend-agnostic) is what v0.1.0 proves;
v0.2.0 fills in the Grid behaviour once the underlying grammar and
resolver PRs land.

## Usage (conceptual; resolver lands in U29-R2)

```moslayout
// In a host component's .mll:
layout SpreadsheetApp {
  Column [ root ] {
    Grid (
      viewport-rows: slot: data-rows ,
      edit-row:      slot: app-edit-row ,
      edit-col:      slot: app-edit-col ,
      onEditCommit:  emit: handleCommit
    ) {
      // Future v0.2.0: Column children declare the columns.
      Column ( key: "name",  header: "Name",  width: 120 , editable: true ,
               cell-type: "text" , default-alignment: "left" )
      Column ( key: "price", header: "Price", width:  80 , editable: true ,
               cell-type: "number" , default-alignment: "right" )
    }
  }
}
```

## Smoke test

```bash
cd code/packages/mosaic-pkg-grid
cargo test
```

The smoke test asserts:

1. `mosaic-package.toml` parses and declares the three expected exports.
2. Each `.mil` compiles via `mosmodel-compiler`.
3. Each `.mll` compiles via `moslayout-compiler` against its `.mil`
   interface descriptor.
4. Each `.dark.msl` compiles via `mosstyle-compiler` against its `.mll`
   part map.

If `Grid.mll` ever stops compiling (because a future resolver PR
tightens its tag-validation to reject unknown primitives), the test
documents the upgrade path: flip Grid's `.mll` assertion to `expect_err`
until U29-R2 + all U29-K-* resolvers cover `HostTable` / `For` / `If` /
userland component references.

## Position in UI29's roadmap

* **U29-P1 (this package)**: ship the source tree, declare the manifest,
  prove the smoke test.
* **U29-D1**: migrate `demo/visicalc` to import this package's `Grid`
  instead of carrying its own bespoke copy.
* **U29-X1**: remove dead `emit_grid_jsx_*` / `emit_input_jsx` /
  `emit_cell_jsx_*` / `emit_column_jsx_*` from every backend now that
  Grid is userland and only kernel-primitive lowering is on the
  critical path.

## License

MIT OR Apache-2.0.
