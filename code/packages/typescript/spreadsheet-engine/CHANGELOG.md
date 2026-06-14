# Changelog

All notable changes to `@coding-adventures/spreadsheet-engine` are documented
here. The format follows [Keep a Changelog](https://keepachangelog.com/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Security / resilience

Cell content is **untrusted host input**, so a malicious or accidental formula
must never be able to exhaust memory or crash the host. Two availability gaps a
security review found are now closed:

- **Capped range expansion (OOM fix).** `expandRange` previously materialized one
  `CellAddress` object per covered cell with no upper bound, so a formula like
  `=SUM(A1:ZZ1000000)` would try to allocate ~700 million objects and run the
  process out of memory — and it triggered on `setCell` (during the dependency
  scan), before evaluation. `address.ts` now exports `MAX_RANGE_CELLS`
  (1 048 576, one Excel column) and a typed `RangeTooLargeError`; `expandRange`
  checks the corner-derived cell count (`rangeCellCount`, O(1)) and throws
  *before* allocating anything. The Excel/CAS adapter catches it: `dependencies()`
  registers no edges, and the range-aggregation path returns `#REF!`. A huge
  range now degrades to a `#REF!` value in milliseconds instead of OOMing.
- **No uncaught `RangeError` (stack-overflow fix).** A pathologically deep
  formula (`=1+1+1+…` with thousands of terms) overflows the recursive
  evaluator's call stack with a `RangeError`. The adapter previously caught only
  its own `FormulaError` and re-threw everything else, so the `RangeError`
  escaped `setCell` and crashed the host. `excel-cas.ts` `evaluate` now degrades
  any non-`FormulaError` throw to `#VALUE!` (and `RangeTooLargeError` to
  `#REF!`), honouring the adapter contract that it "never throws for ordinary
  spreadsheet errors." As belt-and-suspenders, `Workbook.recalcFrom` wraps the
  `adapter.evaluate` call in try/catch and stamps `#VALUE!`, so even a
  *misbehaving third-party adapter* can never crash the engine or leave the
  workbook half-recalculated.

### Tests
- +12 resilience tests (`tests/resilience.test.ts`): the cap constant and
  `rangeCellCount`, `expandRange` rejecting an oversized range fast / accepting
  one exactly at the cap, `=SUM(A1:ZZ1000000)` → `#REF!` quickly with no
  OOM/hang through both the adapter and `setCell`, a 7000-term deep formula
  returning an error without throwing out of `evaluate`/`setCell`, the engine
  surviving a bad cell and still recalculating others, and a deliberately
  throwing adapter being absorbed by the workbook. 109 tests total, all green.

## [0.1.0] — 2026-06-13

Initial release: a generic, headless spreadsheet computation core with a
pluggable formula adapter, plus a default Excel/CAS adapter.

### Added

#### Generic engine core (domain-agnostic)
- **`CellValue`** discriminated union (`empty` | `number` | `text` | `boolean` |
  `error`) with the six error codes `#DIV/0!`, `#REF!`, `#NAME?`, `#VALUE!`,
  `#CIRC!`, `#NA`. Excel-style coercions (`toNumber`/`toText`/`toBoolean`):
  empty → `0`/`""`/`false`; numeric text auto-parses; errors are sticky.
- **`CellAddress` / `CellRange`** with bidirectional A1 notation
  (`parseA1`/`printA1`), bijective base-26 column letters
  (`columnToLetters`/`lettersToColumn`, `A`↔0 … `AA`↔26), optional `$` absolute
  flags, and `expandRange` (row-major).
- **`Workbook`** engine: `setCell`, `getValue`, `getValues`, `getRaw`,
  `setCells` (bulk), `recalcAll`, `setMode`. Holds cells, the dependency graph,
  an epoch counter, and a recalc mode (`auto` | `manual`).
- **`Cell`** model: literal cells (parsed value) vs. formula cells (raw source +
  cached value + last-eval epoch for incremental recalc).
- **`DependencyGraph`**: dual `edgesOut`/`edgesIn` adjacency maps, transitive
  `dirtySet`, and a subgraph Kahn `topoOrderSubset` that returns both the
  evaluation order and the cells caught in cycles (so they can be flagged
  `#CIRC!` without aborting the whole recalc). Deterministic ordering.
- Incremental, topological recalc: on edit, recompute only the edited cell and
  its transitive downstream; cycles → `#CIRC!`.

#### The pluggability seam
- **`FormulaAdapter`** interface (`isFormula` / `dependencies` / `evaluate`) and
  the `CellResolver` callback. The engine core never imports any specific
  adapter — the formula language is entirely external.

#### Default Excel/CAS adapter (`src/adapters/excel-cas.ts`)
- Parses `=…` formulas via `@coding-adventures/excel-parser` and walks the real
  concrete syntax tree (rule names + token types verified against the live
  parser, not guessed).
- Arithmetic lowered to `@coding-adventures/symbolic-ir` nodes and folded with
  `@coding-adventures/cas-simplify`'s `numericFold` (exact integer/rational),
  with a float fallback evaluator for non-foldable trees. Division by zero is
  guarded *before* folding (numericFold throws on a zero denominator) and mapped
  to `#DIV/0!`.
- Operator support: `+ - * / ^` (correct precedence; `^` right-associative),
  unary `+`/`-`, postfix `%`, text concat `&`, comparisons
  `= <> < > <= >=` → booleans.
- Standard function library: `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT`, `PRODUCT`
  over ranges/args; empty cells inside a range are skipped.
- Error handling: div-by-zero → `#DIV/0!`, unknown function → `#NAME?`,
  unparseable → `#NAME?`, non-numeric where a number is needed → `#VALUE!`,
  empty cell → `0` in arithmetic.
- **`createSpreadsheet()`** convenience that wires the default adapter.

### Tests
- 97 tests across four suites: address model, value coercions, the generic
  engine driven by a **toy non-Excel adapter** (proving genericity), and the
  default Excel/CAS adapter end-to-end through the `Workbook`. Line coverage
  93.5% overall (core `src/` files ~100%).

### Notes / scope
- Single-sheet workbook in v1 (structured for multi-sheet later).
- Arrays/dynamic-array spilling, named ranges, structured table refs, volatile
  functions, and iterative-calculation mode (all in the Rust spec) are deferred.
- `@coding-adventures/directed-graph` is a transitive dependency but the recalc
  graph is implemented internally because the shared graph's `topologicalSort`
  can't order a subset or recover from cycles (see README "Design note").
