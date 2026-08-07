# mosaic-pkg-sheet

> Filterable, sortable, editable spreadsheet view wired to task-core's
> `table(view)` projection.

A `mosaic-pkg-grid` `Grid` wrapped in a small toolbar — a filter box and a
sort-field selector (`mosaic-pkg-toolkit`'s `Select`) plus a direction
toggle. No primitives of its own; every cell, row, and column comes
straight from `Grid`.

## What this package exports

One component, per `mosaic-package.toml`'s `[components].exports`:

| Component | Role | File trio |
|---|---|---|
| `Sheet` | filter/sort toolbar + `Grid` | `Sheet.mil` / `Sheet.mll` / `Sheet.{dark,light}.msl` |

## How it fits in the stack

```
          ┌──────────────────────────────────────────┐
          │  Host application (task-app's Sheet view) │
          └─────────────────────┬──────────────────────┘
                                │ component reference
                                ▼
          ┌──────────────────────────────────────────┐
          │  mosaic-pkg-sheet (this package)          │
          │  Sheet → HostInput + Select + Grid        │
          └────────┬─────────────────────┬─────────────┘
                    │                     │
                    ▼                     ▼
     mosaic-pkg-toolkit::Select   mosaic-pkg-grid::Grid
```

## Fat engine, dumb UI

Sheet does not filter, sort, or format anything itself (per
[task-app-super-app.md](../../../specs/task-app-super-app.md) §2.1).
Filtering and sorting are two plain inputs — `filter-text`, `sort-field` +
`sort-ascending` — that the host collects from Sheet's emits and folds
into the `View` it passes to task-core's `table(view)` projection on the
next render. The rows Sheet receives back are already filtered, sorted,
and formatted; Sheet only places them.

## Editing (0.1.1+)

`onNavigate(row, col)` / `onFormulaChange(value)` / `onEditCommit(value)`
carry their full payload — a click identifies the cell, and edits reach
the host. This depends on `mosaic-pkg-grid` 0.2.3 +
[UI37](../../../specs/UI37-generic-payload-dispatch.md); 0.1.0 shipped
these void because `Grid`'s `Cell` (a `Box`, a generic container) couldn't
carry a click payload through `mosaic-emit-react` at all yet. See
CHANGELOG.md.

## Usage

```moslayout
// In a host component's .mll:
pkg::mosaic-pkg-sheet::Sheet (
  viewport-rows:   slot: sheet-viewport-rows ,
  column-headers:  slot: sheet-column-headers ,
  column-widths:   slot: sheet-column-widths ,
  selected-row:    slot: sheet-selected-row ,
  selected-col:    slot: sheet-selected-col ,
  edit-row:        slot: sheet-edit-row ,
  edit-col:        slot: sheet-edit-col ,
  edit-content:    slot: sheet-edit-content ,
  filter-text:     slot: sheet-filter-text ,
  sort-field:      slot: sheet-sort-field ,
  sort-options:    slot: sheet-sort-options ,
  sort-open:       slot: sheet-sort-open ,
  sort-ascending:  slot: sheet-sort-ascending ,
  onFilterChange:        emit: onSheetFilterChange ,
  onSortFieldChange:     emit: onSheetSortFieldChange ,
  onToggleSortOpen:      emit: onSheetToggleSortOpen ,
  onToggleSortDirection: emit: onSheetToggleSortDirection
)
```

The host builds `viewport-rows`/`column-headers`/`column-widths` from a
`table(view)` call whose `View.filter.search`/`View.sort` come from the
host's own `filter-text`/`sort-field`/`sort-ascending` state — see
`code/programs/mosaic/task-app/host/web/src/main.tsx`'s `SHEET_VIEW` /
`SHEET_FIELDS` for a worked example (the reference host consumer).

## Smoke test

```bash
cd code/packages/mosaic/mosaic-pkg-sheet
cargo test
```

Mirrors `mosaic-pkg-grid`'s own smoke test: manifest parses and declares
the expected export + dependencies; `Sheet.mil` compiles via
`mosmodel-compiler`; `Sheet.mll` compiles against that interface via
`moslayout-compiler` (with an explicit pin that `Grid`/`Select` are
referenced via the qualified `pkg::P::C` form, not inlined by hand); both
themes' `.msl` compile against the resulting part map.

Cross-package references stay **unresolved** at this layer (UI34 §5 —
`pkg::P::C` substitution is `mosaic-compile`'s job, not
`moslayout_compiler::compile`'s) — the package's own build script /
`mosaic-compile pkg` invocation is what actually inlines `Grid` and
`Select`, and is exercised end-to-end by task-app's own build
(`scripts/build-web.sh`).

## License

MIT OR Apache-2.0.
