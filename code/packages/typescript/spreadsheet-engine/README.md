# @coding-adventures/spreadsheet-engine

A **headless, adapter-pluggable spreadsheet computation core**. It owns the
*essential machinery* of a spreadsheet — a grid of named cells, a dependency
graph between them, and an incremental topological recalculation engine — while
keeping the **formula language itself pluggable**. The engine never hard-codes
Excel; you supply a `FormulaAdapter` and it drives any table of inter-related
values.

A default **Excel/CAS adapter** ships with the package, so it computes real
spreadsheet formulas (`=SUM(A1:A5)`, `=A1+B2*3`, `=AVERAGE(A1:A3)/2`, …) out of
the box.

> This package is a TypeScript port of the model in
> `code/specs/spreadsheet-core.md` (written in Rust-ish pseudocode). VisiCalc —
> and any other spreadsheet UI — is meant to be built *on top* of this core in a
> later pass.

---

## The two halves

```
┌──────────────────────────────────────────────┐
│  GENERIC ENGINE  (domain-agnostic)            │
│                                               │
│   Workbook                                    │
│    ├─ cells          (address → Cell)         │
│    ├─ DependencyGraph (edgesOut / edgesIn)    │
│    ├─ epoch          (incremental freshness)  │
│    └─ recalc         (dirty set → topo sort)  │
│                                               │
│   knows NOTHING about "=", SUM, or Excel.     │
└───────────────────┬──────────────────────────┘
                    │ FormulaAdapter (the seam)
        ┌───────────┴───────────┐
        ▼                       ▼
  excelCasAdapter         your own adapter
  (= formulas, CAS         (any formula language,
   arithmetic, SUM/…)       or any non-spreadsheet
                            table of related values)
```

The generic core (`Workbook`, `DependencyGraph`, the value/address model) is in
`src/`. The default adapter is in `src/adapters/excel-cas.ts` and is **never
imported by the core** — delete it and the engine still works with any other
adapter.

---

## Install

This is a `file:`-linked package inside the monorepo. See `BUILD` for the
leaf-to-root install chain; in short:

```bash
npm install   # after the dependency chain in BUILD is installed
npm test
```

---

## Quick start (default Excel/CAS adapter)

```ts
import { createSpreadsheet, formatValue } from "@coding-adventures/spreadsheet-engine";

const wb = createSpreadsheet();          // wired with excelCasAdapter
wb.setCell("A1", "10");
wb.setCell("A2", "20");
wb.setCell("A3", "=SUM(A1:A2)");

formatValue(wb.getValue("A3"));          // "30"
wb.setCell("A1", "100");                 // auto-recalc
formatValue(wb.getValue("A3"));          // "120"
```

Supported by the default adapter:

| Feature                | Example                       | Result            |
|------------------------|-------------------------------|-------------------|
| Arithmetic + precedence| `=A1+B2*3`                     | `2 + 3*3 = 11`    |
| Parentheses            | `=(A1+B2)*3`                  | `15`              |
| Exponent (right-assoc) | `=2^3^2`                      | `512`             |
| Unary minus / percent  | `=-A1`, `=A1%`                | `-2`, `0.02`      |
| Text concat            | `="foo"&"bar"`               | `"foobar"`        |
| Comparison             | `=A1<A2`                      | `TRUE`/`FALSE`    |
| `SUM AVERAGE MIN MAX COUNT PRODUCT` | `=AVERAGE(A1:A3)`| reduces a range   |
| Division by zero       | `=1/0`                        | `#DIV/0!`         |
| Unknown function       | `=BOGUS(A1)`                  | `#NAME?`          |
| Empty cell in math     | `=Z9+5`                       | `5` (blank = 0)   |

**Documented defaults:** empty cells coerce to `0` in arithmetic (Excel's "a
blank cell behaves as zero"), `""` in text concatenation, and `false` in logical
contexts. Empty cells *inside* a `SUM`/`AVERAGE`/`COUNT` range are skipped (a
blank is not counted as a zero). Single-sheet only in v1.

---

## Writing your own adapter (the generic path)

The engine talks to formulas through exactly three methods:

```ts
export interface FormulaAdapter {
  isFormula(raw: string): boolean;                 // is this content a formula?
  dependencies(raw: string): CellAddress[];        // which cells does it read?
  evaluate(raw: string, resolve: CellResolver): CellValue; // compute it
}
```

A complete toy adapter whose formulas are `:A1+A2` (sum of cell refs):

```ts
import { Workbook, parseA1, num, toNumber } from "@coding-adventures/spreadsheet-engine";

const toy = {
  isFormula: (raw) => raw.startsWith(":"),
  dependencies: (raw) => raw.slice(1).split("+").map((s) => parseA1(s.trim())),
  evaluate: (raw, resolve) => {
    let sum = 0;
    for (const ref of raw.slice(1).split("+")) {
      const n = toNumber(resolve(parseA1(ref.trim())));
      if (typeof n !== "number") return n;          // propagate errors
      sum += n;
    }
    return num(sum);
  },
};

const wb = new Workbook({ adapter: toy });
wb.setCell("A1", "10");
wb.setCell("A2", "20");
wb.setCell("A3", ":A1+A2");
wb.getValue("A3"); // { kind: "number", value: 30 }
```

Because the engine only knows about the adapter, the *same* dependency tracking,
topological recalc, incremental update, and cycle detection work unchanged.

---

## The recalc heart

- **Dependency graph** (`edgesOut` = "cells I read", `edgesIn` = "cells that
  read me"). Both directions are kept because both queries are hot.
- On `setCell`, the adapter's `dependencies()` rewrite the cell's out-edges.
- The **dirty set** = the edited cell plus its transitive `edgesIn` closure.
- The dirty set is **topologically ordered** (Kahn's algorithm, restricted to the
  subset, with a deterministic tie-break) and evaluated dependencies-first.
- Cells caught in a **cycle** are stamped `#CIRC!` — the rest of the recalc still
  completes (we don't abort the whole pass).
- **Automatic** mode recomputes after every edit; **Manual** mode waits for
  `recalcAll()`.

### Design note: why a small internal graph instead of `directed-graph`?

We do depend on `@coding-adventures/directed-graph` transitively (via
`excel-parser`), and it has an excellent `topologicalSort()`. But that method
sorts the **whole** graph and throws a `CycleError` the instant *any* cycle
exists anywhere. Incremental recalc needs two things it doesn't offer:

1. Topologically order only a **dirty subset** (one edit must not re-sort the
   whole book), and
2. **recover from cycles** by marking just the cycle's cells `#CIRC!` while
   still evaluating everything else.

Those are spreadsheet-specific recalc concerns, so `DependencyGraph` here is a
small, purpose-built adjacency-map graph (a few dozen lines: `setDependencies`,
`dirtySet`, a subgraph Kahn `topoOrderSubset`). See `src/dependency-graph.ts`.

---

## Public API

- `Workbook` — `setCell(a1, raw)`, `getValue(a1)`, `getValues()`, `getRaw(a1)`,
  `setCells(record)`, `recalcAll()`, `setMode("auto"|"manual")`. Constructed with
  `{ adapter, mode? }`.
- `createSpreadsheet({ mode? })` — a `Workbook` pre-wired with `excelCasAdapter`.
- `FormulaAdapter`, `CellResolver` — the pluggability seam.
- `excelCasAdapter` — the default Excel/CAS adapter.
- Value model: `CellValue`, `CellErrorCode`, and helpers `num/text/bool/err/
  EMPTY/isError/toNumber/toText/toBoolean/formatValue`.
- Addresses: `CellAddress`, `CellRange`, `parseA1/printA1/addressKey/
  columnToLetters/lettersToColumn/parseRange/normalizeRange/expandRange`.
- `Cell` (`LiteralCell` | `FormulaCell`), `DependencyGraph`.

---

## Where it fits in the stack

```
  VisiCalc / any spreadsheet UI            ← future, builds on this
            │
  spreadsheet-engine  (THIS)               ← cells + graph + recalc + adapter seam
            │  excelCasAdapter composes ↓
  excel-parser · symbolic-ir · cas-simplify · symbolic-vm
```

## License

MIT
