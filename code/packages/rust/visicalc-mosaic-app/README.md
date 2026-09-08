# VisiCalc Mosaic application

`VisiCalcMosaicApp` implements the UI38 `MosaicApp` lifecycle and exports the
standard C ABI through `mosaic-app-capi`. It owns selection, the edit buffer,
the rendered row window and appearance. `spreadsheet-core` owns cell coercion,
formula evaluation, dependency recalculation and workbook serialization.

The default workbook uses the shared
[budget fixture](../../../programs/mosaic/visicalc/fixtures/budget-v1.json).
The same 16-step presentation contract runs against the generated React host
and this adapter. The generated React root uses this adapter through the standard
WASM lifecycle; native interaction acceptance remains in #14272.

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
| `openWorkbook` | empty object | Request a user-selected VisiCalc file through UI47 |
| `saveWorkbook` | empty object | Capture committed state and request a file destination |

Indices must be integer numbers within bounds. Invalid events are rejected
before mutation; the runtime may retry the same sequence number. Navigation,
commit, cancel, restore and new-workbook operations emit standard announcements.
Protocol-2 hosts execute standard `file.open` and `file.save` Await effects through
the dedicated completion channel. Protocol-1 native hosts receive an explicit
unavailable status and emit no unsupported Await. Filesystem access belongs to
the shared host capability executor.

Snapshots use schema `visicalc-mosaic-app/state`, version 1, containing the
engine's serialized workbook and presentation cursor. They exclude uncommitted
edit text. Restore validates a temporary cursor/workbook before replacing the
live state. Unsupported versions, invalid cursor bounds, malformed workbooks,
and empty/multi-sheet snapshots fail without changing the current app.

`.visicalc` files contain the JSON snapshot envelope; the file capability carries
those bytes as base64, capped at 16 MiB. Save asks the user to apply any pending
cell edit first. It captures the checkpoint before emitting Await and reports
success only after the host closes the write. While a file operation is pending,
workbook/edit commands leave state unchanged; viewport measurements still work.
Open validates the returned snapshot transactionally. Cancelled, denied and
malformed opens preserve the current workbook and edit buffer. Malformed content
settles the operation with a visible error, rather than leaving a pending protocol
result forever. File operation IDs survive same-instance restore.

Generated Open/Save controls and real-WASM interaction tests cover this contract.
OS file-dialog acceptance remains #14548: the current automated desktop session
does not expose the in-app browser's native picker, and Chrome browser-tool
connection is unavailable. A successful boundary test is not a downloaded-file or
native-dialog launch test.

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

Selected-cell descriptions use committed display values rather than pending
keystrokes. Formula cells include the result (including errors) and source;
empty cells say blank. The selection-summary prop and polite announcements share
this Rust-owned description. Commits identify the updated cell and, after inline
commit, the next selection. Cancel/restore/new-workbook messages include current
selection context. Typing, viewport changes and a no-op cancel stay silent.
