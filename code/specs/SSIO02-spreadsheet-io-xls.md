# SSIO02 — `spreadsheet-io`: legacy `.xls` (BIFF8) load & save

Extends [SSIO01](SSIO01-spreadsheet-io.md) (which unified `.xlsx` onto
`spreadsheet-core`) with the **legacy `.xls`** format, so VisiCalc can open and
save the binary Excel 97–2003 files too. Same hub, same adapter crate — two new
functions.

## Public API (added)

```rust
/// Load a legacy .xls (BIFF8) file into a live engine workbook.
pub fn load_xls(bytes: &[u8]) -> Result<Workbook, IoError>;

/// Serialize an engine workbook to legacy .xls bytes.
pub fn save_xls(wb: &Workbook) -> Vec<u8>;
```

`IoError` gains an `Xls(String)` variant alongside `Xlsx`.

## Load: `.xls` → engine

```
bytes ─▶ xls::open_xls   (OLE2/CFB → BIFF8 records → typed cells, 0-based)
      ─▶ spreadsheet-core::Workbook   (addresses shifted 0→1-based)
```

Per cell, the reader's `xls::CellValue` maps to the engine's `CellValue`:

| `xls::CellValue` | engine `CellValue` |
|------------------|--------------------|
| `Number(f64)`    | `Number` |
| `Text(String)`   | `Text` |
| `Bool(bool)`     | `Boolean` |
| `Error(u8)`      | `Error(SpreadsheetError)` via `biff_error_to_core` |
| `Formula{cached}`| the **cached value** as a literal (see below) |
| `Blank`          | skipped |

**BIFF error codes → engine errors** (`biff_error_to_core`): `0x00→#NULL!`,
`0x07→#DIV/0!`, `0x0F→#VALUE!`, `0x17→#REF!`, `0x1D→#NAME?`, `0x24→#NUM!`,
`0x2A→#N/A`; anything else → `#VALUE!` (never dropped).

### Fidelity limit — formulas do not survive load

The `.xls` reader decodes a formula cell's **cached result** but not its
**expression**. So a `.xls` formula loads as a plain value, and — unlike the
`.xlsx` path — there is nothing to recompute from. Worse, some producers (e.g.
**xlwt**) write formulas with *no* cached value at all, so those cells arrive
empty. This is a limitation of the legacy format's reader, documented and pinned
in tests, not a bridge choice. (The `.xlsx` path preserves live formulas; prefer
it when fidelity matters.)

## Save: engine → `.xls`

Walks each sheet's populated cells **sparsely** (`populated_cells`, same
DoS-safe pattern as `save_xlsx`). The `.xls` writer's value model is smaller —
**numbers and strings only** — so every cell is written as its computed value:

| Cell in the engine | Written to `.xls` |
|--------------------|-------------------|
| `Number(n)` (literal or formula cache) | numeric cell |
| `Text(s)`          | shared-string cell |
| `Boolean(b)`       | numeric `1`/`0` |
| `Error(e)`         | its display text as a string |
| empty              | omitted |

**Limits:** formulas are **not** stored (their computed result is written);
BIFF cell addresses are `u16`, so a cell beyond row/col 65535 is skipped by the
writer. Engine addresses are 1-based, the writer 0-based (`row-1`, `col-1`;
`populated_cells` never yields 0, so no underflow).

## Round-trip contract

Numbers and text round-trip through `.xls` exactly; formulas round-trip as their
computed value (not as formulas). The write side is idempotent. Tests assert all
of this, plus:

- **Third-party interop both ways:** our output is re-read by **xlrd**; a
  committed **xlwt-authored** `.xls` fixture is read by `load_xls` (values exact;
  its uncached formula cells documented as empty).
- `.xls` output begins with the OLE2 magic `D0 CF 11 E0 …`.

## Non-goals

- Preserving `.xls` formula expressions (the reader doesn't decode them).
- Number formats / styles (as in SSIO01).
- The front-end wiring (SSIO03–05) is format-agnostic and will expose both
  `.xlsx` and `.xls` through the same open/save path.
