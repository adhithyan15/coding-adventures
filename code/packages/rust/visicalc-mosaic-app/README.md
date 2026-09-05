# VisiCalc Mosaic application

`VisiCalcMosaicApp` implements the UI38 `MosaicApp` lifecycle and exports the
standard C ABI through `mosaic-app-capi`. It owns selection, the edit buffer,
the rendered row window and appearance. `spreadsheet-core` owns cell coercion,
formula evaluation, dependency recalculation and workbook serialization.

The default workbook uses the shared
[budget fixture](../../../programs/mosaic/visicalc/fixtures/budget-v1.json).
The same 16-step presentation contract runs against the generated React host
and this adapter. These are two implementations during migration; the React
host has not yet been switched to this adapter (#14272).

## Contract

Props use Mosaic slot names in kebab-case. Selection and edit row/column props
are zero-based **absolute workbook coordinates**; `viewport-rows` contains the
slice beginning at `viewport-offset`. Root Mosaic composition must translate
coordinates for a Grid that uses slice-relative indices. An idle edit uses -1
for both edit coordinates. The current reference workbook surface is 100 rows
by 26 columns on one sheet; broader workbook commands remain in #14279.

Supported events (also accept the `onNavigate`/`onFormulaChange` spelling):

| Event | Payload | Behavior |
|---|---|---|
| `navigate` | `row`, `col` | Select and reveal a cell; discard any edit |
| `editStart` | `row`, `col` | Select/reveal and buffer the cell's source |
| `formulaChange` | `value` | Buffer text; begin editing selection if idle |
| `commit` | empty object | Commit through the engine; retain selection |
| `editCommit` | empty object | Commit and move down, bounded at the last row |
| `cancel`, `editCancel` | empty object | Discard uncommitted text |
| `scroll` | `offset` | Set a valid row-window offset; retain selection |
| `resizeViewport` | `rows` | Set a 1–100 row window and reveal selection |
| `newWorkbook` | empty object | Replace contents with an empty sheet |

Indices must be integer numbers within bounds. Invalid events are rejected
before mutation; the runtime may retry the same sequence number. Navigation,
commit, cancel, restore and new-workbook operations emit standard announcements.
The adapter requests no custom host effects. Hosts use the standard lifecycle
snapshot/restore surface for persistence; filesystem access belongs to the host.

Snapshots use schema `visicalc-mosaic-app/state`, version 1, containing the
engine's serialized workbook and presentation cursor. They exclude uncommitted
edit text. Restore validates a temporary cursor/workbook before replacing the
live state. Unsupported versions, invalid cursor bounds, malformed workbooks,
and empty/multi-sheet snapshots fail without changing the current app.

## Validation

```sh
cd code/packages/rust
cargo test -p visicalc-mosaic-app
cargo build -p visicalc-mosaic-app
cargo clippy -p visicalc-mosaic-app --all-targets -- -D warnings
```

Tests cover the shared fixture, committed snapshot round trips, malformed input
atomicity, bounded viewport resizing, runtime sequence retry, and the exported
C ABI's create/dispatch/snapshot/restore/buffer-free/destroy lifecycle. The
VisiCalc Linux/Windows workflow also runs the Rust tests when shared fixtures
change. This is application/ABI validation; native GUI launch, physical
scrolling, accessible focus, polished design and downloaded-release acceptance
remain required by [the delivery backlog](https://github.com/adhithyan15/coding-adventures/issues/14267).
