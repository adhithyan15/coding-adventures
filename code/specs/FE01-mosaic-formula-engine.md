# FE01 — mosaic-formula-engine

**Track:** Formula Engine (FE)
**Status:** Accepted
**Created:** 2026-05-16

---

## 1. Purpose

`mosaic-formula-engine` is a standalone Rust crate that provides a VisiCalc-style formula
evaluator for spreadsheet-like cells. It has **no UI dependencies** and no dependencies on
any other Mosaic crates. Its only dependency is the Rust standard library.

The crate gives callers three capabilities:

1. **Cell storage** — store raw strings (numbers, text, or `=formula`) in an addressable
   grid of up to 26 columns (A–Z) × 99 rows (1–99).
2. **Formula evaluation** — parse and evaluate arithmetic expressions, cell references,
   ranges, and built-in functions.
3. **Dependency tracking** — maintain a directed dependency graph and recalculate dirty
   cells in topological order, detecting circular dependencies.

---

## 2. Public API

### 2.1 CellAddr

```rust
/// A spreadsheet cell address. Column A-Z (0-25), row 1-99.
pub struct CellAddr { col: u8, row: u8 }

impl CellAddr {
    /// Parse "A1", "B12", "Z99". Column must be a single A-Z letter.
    /// Row must be 1-99. Returns Err(FormulaError::Parse) on invalid input.
    pub fn parse(s: &str) -> Result<Self, FormulaError>;

    /// Format as "A1", "B12", etc.
    pub fn to_string(&self) -> String;
}
```

### 2.2 CellValue

```rust
pub enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Bool(bool),
    Error(FormulaError),
}
```

### 2.3 FormulaError

VisiCalc-compatible error codes:

```rust
pub enum FormulaError {
    DivZero,  // #DIV/0! — division by zero
    Ref,      // #REF!   — invalid cell reference
    Name,     // #NAME?  — unknown function name
    Value,    // #VALUE! — wrong type for operation
    Circ,     // #CIRC   — circular dependency
    Parse,    // #PARSE  — formula syntax error
}
```

Display strings: `#DIV/0!`, `#REF!`, `#NAME?`, `#VALUE!`, `#CIRC`, `#PARSE`.

### 2.4 FormulaEngine

```rust
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    /// Create an empty engine.
    pub fn new() -> Self;

    /// Set a cell to a raw string.
    /// If `raw` starts with '=', it is treated as a formula.
    /// Otherwise it is a literal: a parseable f64 becomes Number, else Text.
    pub fn set_raw(&mut self, addr: CellAddr, raw: String);

    /// Get the display string for a cell (what the user sees in the cell).
    /// Empty cells return "".  Numbers are formatted without trailing ".0"
    /// when integral (e.g. 6.0 → "6", 2.5 → "2.5").
    pub fn get_display(&self, addr: &CellAddr) -> String;

    /// Get the formula string (what appears in the formula bar).
    /// For literal cells this is the original raw string.
    /// For formula cells this is the original "=..." string.
    pub fn get_formula(&self, addr: &CellAddr) -> String;

    /// Evaluate all dirty cells in dependency order.
    /// Must be called after set_raw before get_display reflects the new value.
    pub fn recalculate(&mut self);

    /// Convenience: parse a cell address string.
    pub fn cell_addr(s: &str) -> Result<CellAddr, FormulaError>;
}
```

---

## 3. Expression grammar

Formula strings begin with `=`. The grammar is:

```ebnf
formula   = "=" expr
expr      = term { ("+" | "-") term }
term      = factor { ("*" | "/") factor }
factor    = NUMBER
           | STRING
           | BOOL
           | cell_ref
           | range_ref       (* inside function args only *)
           | func_call
           | "(" expr ")"
           | "-" factor

cell_ref  = LETTER DIGIT+    (* e.g. A1, B12 *)
range_ref = cell_ref ":" cell_ref   (* e.g. A1:C3 *)
func_call = IDENT "(" [arg { "," arg }] ")"
arg       = range_ref | expr

NUMBER    = ["-"] DIGIT+ ["." DIGIT+]
STRING    = '"' [^"]* '"'
BOOL      = "TRUE" | "FALSE"
IDENT     = LETTER { LETTER | DIGIT | "_" }
LETTER    = "A"-"Z" | "a"-"z"
DIGIT     = "0"-"9"
```

The unary minus (`-factor`) has the highest precedence. Multiplication and division bind
tighter than addition and subtraction.

### 3.1 Built-in functions

| Function | Signature | Semantics |
|----------|-----------|-----------|
| `SUM`    | `SUM(val_or_range…)` | Sum of all numeric values. Text/empty → 0. |
| `AVG`    | `AVG(val_or_range…)` | Arithmetic mean of all numeric values. |
| `COUNT`  | `COUNT(val_or_range…)` | Count of cells that are non-empty. |
| `MAX`    | `MAX(val_or_range…)` | Maximum numeric value. |
| `MIN`    | `MIN(val_or_range…)` | Minimum numeric value. |
| `IF`     | `IF(cond, true_val, false_val)` | 0/FALSE → false_val, else true_val. |

Function names are case-insensitive in parsing.

### 3.2 Range expansion

`A1:C3` expands to all cells in the rectangle with top-left `A1` and bottom-right `C3`. The
expanded cell list is iterated column-major then row-major (A1, A2, A3, B1, B2, B3, C1, C2, C3).
Ranges may only appear as function arguments.

---

## 4. Dependency graph

- A cell that contains a formula depends on every cell it references.
- When `set_raw` is called, the cell is marked **dirty**.
- When a dependency cell changes, all cells that reference it are also marked dirty.
- `recalculate()` performs a topological sort (Kahn's algorithm) of all dirty cells and
  evaluates each in dependency order (dependencies first).
- **Cycle detection:** if the topological sort cannot complete (cycle exists), all cells
  in the SCC (cycle) are assigned `CellValue::Error(FormulaError::Circ)`.
- After recalculation, all previously-dirty cells are clean.

---

## 5. Error propagation

If a formula references a cell whose value is `CellValue::Error(e)`, the referencing
formula immediately propagates the same error without further evaluation.

---

## 6. Crate structure

```
code/packages/rust/mosaic-formula-engine/
  Cargo.toml
  BUILD
  README.md
  CHANGELOG.md
  src/
    lib.rs        — module declarations, re-exports, top-level crate doc
    addr.rs       — CellAddr: parse(), to_string(), column/row accessors
    value.rs      — CellValue, FormulaError: Display impls
    lexer.rs      — Token enum, tokenize(formula) -> Result<Vec<Token>>
    parser.rs     — Expr AST, parse(tokens) -> Result<Expr>
    eval.rs       — eval(expr, context) -> CellValue
    builtins.rs   — SUM, AVG, COUNT, MAX, MIN, IF implementations
    graph.rs      — DependencyGraph: add_deps, topological_sort, cycle_detection
    engine.rs     — FormulaEngine: set_raw, get_display, get_formula, recalculate
```

---

## 7. Implementation constraints

- **No external crates** — only `std`. No nom, pest, serde, or any third-party crate.
- **No unsafe** — zero `unsafe` blocks.
- **No unwrap() on user input** — all parsing is fallible; return `Err(FormulaError::Parse)`.
- **Literate programming** — every module has a `//!` doc comment explaining what it does,
  with analogies and examples. Complex algorithms have inline comments.
- Column bounds: valid columns are 'A'–'Z' (u8 values 0–25).
- Row bounds: valid rows are 1–99.

---

## 8. Test plan (95%+ coverage target)

Each item maps to a `#[test]` function in the relevant module's `#[cfg(test)]` block.

### Address parsing
- `test_cell_addr_parse_valid` — "A1", "Z99", "B12" parse correctly
- `test_cell_addr_parse_invalid` — "", "11", "AA1", "A0", "A100" all return Err

### Literal cells
- `test_literal_number` — set "42", get_display → "42"
- `test_literal_text` — set "Hello", get_display → "Hello"
- `test_empty_cell_display` — unset cell → ""

### Basic arithmetic formulas
- `test_formula_addition` — "=1+2" → "3"
- `test_formula_subtraction` — "=10-3" → "7"
- `test_formula_multiplication` — "=3*4" → "12"
- `test_formula_division` — "=10/4" → "2.5"
- `test_division_by_zero` — "=1/0" → "#DIV/0!"
- `test_negative_literal` — "=-5" → "-5"
- `test_nested_parens` — "=(1+2)*3" → "9"

### Cell references
- `test_cell_reference` — A1=5, B1="=A1*2" → B1="10"
- `test_chain_reference` — A1=2, B1="=A1+1", C1="=B1+1" → C1="4"
- `test_update_propagation` — A1=5→recalc; A1=10→recalc; B1 updates to "20"

### Built-in functions
- `test_sum_function` — SUM(A1,A2,A3) where A1..A3=1,2,3 → "6"
- `test_sum_range` — SUM(A1:A3) → "6"
- `test_avg_function` — AVG(1,2,3) → "2"
- `test_count_function` — COUNT over mixed numbers/text → "3"
- `test_max_function` — MAX(3,1,4,1,5) → "5"
- `test_min_function` — MIN(3,1,4,1,5) → "1"
- `test_if_true` — IF(1,2,3) → "2"
- `test_if_false` — IF(0,2,3) → "3"

### Error cases
- `test_circular_reference` — A1="=B1", B1="=A1" → both "#CIRC"
- `test_error_propagation` — A1="=1/0", B1="=A1+1" → B1 also errors
- `test_unknown_function` — "=FOO(1)" → "#NAME?"

### Formula bar
- `test_get_formula_returns_raw` — set "=A1+1", get_formula → "=A1+1"

---

## 9. Number formatting

Numbers are formatted as follows:

- If the value is an integer (no fractional part and is finite), format without decimal:
  `6.0` → `"6"`, `-3.0` → `"-3"`.
- Otherwise format with minimal decimal places using Rust's default `f64` Display:
  `2.5` → `"2.5"`.
- Infinity and NaN are not produced by the evaluator; any operation that would produce
  them yields an appropriate `FormulaError` instead.

---

## 10. Divergences from VisiCalc

| Feature | VisiCalc | This crate |
|---------|----------|------------|
| Column range | A–BK (63 cols) | A–Z (26 cols) |
| Row range | 1–254 | 1–99 |
| Functions | @SUM, @AVG, @IF, etc. | SUM, AVG, AVG, IF, etc. (no @ prefix) |
| String literals | Not supported | Supported (double-quoted) |
| Boolean literals | Not supported | TRUE/FALSE |
| Error codes | ERR, NA | #DIV/0!, #REF!, #NAME?, #VALUE!, #CIRC, #PARSE |

These constraints simplify the implementation while preserving the core semantics.
