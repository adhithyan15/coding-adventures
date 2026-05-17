# mosaic-formula-engine

A standalone VisiCalc-style formula evaluator for spreadsheet cells, written
in pure safe Rust with no external dependencies.

## What it does

`mosaic-formula-engine` provides an in-memory spreadsheet grid where each cell
can hold a literal value or a formula that references other cells.  It handles:

- **Arithmetic** — `+`, `-`, `*`, `/`, unary negation, parentheses
- **Cell references** — `A1`, `B12`, `Z99` (columns A–Z, rows 1–99)
- **Range references** — `A1:C3` (expands to all cells in the rectangle)
- **Built-in functions** — `SUM`, `AVG`, `COUNT`, `MAX`, `MIN`, `IF`
- **Dependency tracking** — automatically re-evaluates dependent cells
- **Cycle detection** — circular references produce `#CIRC` errors
- **Error propagation** — errors flow downstream to dependent cells

## How it fits in the stack

This crate is completely independent of all other Mosaic crates.  It has no UI
dependencies and can be embedded in any context that needs spreadsheet
semantics: a table widget, a configuration DSL, a data pipeline, or a
REPL-style calculator.

## Quick start

```rust
use mosaic_formula_engine::{CellAddr, FormulaEngine};

let mut engine = FormulaEngine::new();

// Set literal values.
engine.set_raw(CellAddr::parse("A1").unwrap(), "10".to_string());
engine.set_raw(CellAddr::parse("A2").unwrap(), "20".to_string());
engine.set_raw(CellAddr::parse("A3").unwrap(), "30".to_string());

// Set a formula that sums a range.
engine.set_raw(CellAddr::parse("B1").unwrap(), "=SUM(A1:A3)".to_string());

// Recalculate before reading.
engine.recalculate();

assert_eq!(engine.get_display(&CellAddr::parse("B1").unwrap()), "60");
```

## API reference

### `CellAddr`

```rust
// Parse a cell address string.
let addr = CellAddr::parse("A1")?;

// Access column (0-based) and row (1-based).
assert_eq!(addr.col(), 0);
assert_eq!(addr.row(), 1);

// Format back to string.
assert_eq!(addr.to_string(), "A1");
```

### `FormulaEngine`

| Method | Description |
|--------|-------------|
| `new()` | Create an empty engine |
| `set_raw(addr, raw)` | Store a literal or formula string |
| `recalculate()` | Evaluate all dirty cells in dependency order |
| `get_display(&addr)` | Get the computed display string |
| `get_formula(&addr)` | Get the raw formula string |
| `cell_addr(s)` | Convenience: parse a cell address |

### Formula language

```
=1 + 2               → "3"
=A1 * 2              → depends on A1
=SUM(A1:A3)          → sum of cells A1, A2, A3
=AVG(1, 2, 3)        → "2"
=IF(A1, "yes", "no") → "yes" if A1 is truthy
=(1 + 2) * 3         → "9"
=-5                  → "-5"
```

## Error codes

| Code | Meaning |
|------|---------|
| `#DIV/0!` | Division by zero |
| `#REF!` | Invalid cell reference |
| `#NAME?` | Unknown function name |
| `#VALUE!` | Wrong type for operation |
| `#CIRC` | Circular dependency |
| `#PARSE` | Formula syntax error |

## Constraints

- **Columns**: A–Z (26 columns)
- **Rows**: 1–99
- **No external dependencies** — only `std`
- **No unsafe** — zero `unsafe` blocks

## Spec

[`code/specs/FE01-mosaic-formula-engine.md`](../../../specs/FE01-mosaic-formula-engine.md)
