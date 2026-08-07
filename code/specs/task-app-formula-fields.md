# task-app — Formula & Rollup Fields

> Part of the [task-app spec series](task-app-overview.md). Specifies how the typed custom-field
> system from [`task-app-data-model.md`](task-app-data-model.md) computes `Formula` and `Rollup`
> fields, by **reusing** the repo's `symbolic-vm` evaluator — no new formula engine.

## Why `symbolic-vm`, not spreadsheet-core

Computed task fields need **named-variable** formulas — `[work] / [duration]`, `[cost] * 1.1` — not
cell coordinates. `spreadsheet-core`'s formula language is A1-only (`B2*C2`), so it cannot express
field references without an ugly synthetic-address shim. `symbolic-vm` (+ `symbolic-ir`) is the
opposite: a variable *is* a named atom `IRNode::Symbol("work")`. It is Rust, mature (266 tests), has
the arithmetic/logical/conditional operators we need, a pluggable `Backend` trait for extension, and
a walkable AST for dependency extraction. It is the right reuse.

## Confirmed API surface (what we build on)

```rust
// symbolic-ir
enum IRNode { Symbol(String), Integer(i64), Rational(i64,i64), Float(f64), Str(String), Apply(IRApply) }
// heads: "Add" "Sub" "Mul" "Div" "Pow" "Neg" "If" "And" "Or" "Not" comparisons, etc.

// symbolic-vm
trait Backend { fn bind(&mut self, name: &str, value: IRNode); /* … */ }
struct StrictBackend; // "calculator mode": every name must be bound, else it errors — ideal for computed fields
struct VM;
impl VM { fn new(backend: Box<dyn Backend>) -> Self; fn eval(&mut self, node: IRNode) -> IRNode; }
fn substitute(node: IRNode, mapping: &HashMap<String, IRNode>) -> IRNode;
```

## Field kinds we compute

From the data model:

```rust
Formula { source: String }                              // e.g. "[work] / [duration] * 100"
Rollup  { over: RollupScope, field: FieldId, agg: RollupAgg } // aggregate children/descendants/assignments
```

Both are **computed, never stored** — recomputed from inputs on change.

## Pipeline for a `Formula` field

### 1. Parse `[Field]` bracket syntax → `IRNode`

We own a small surface parser (justified new code, ~150 lines) that maps our field-reference syntax
to `symbolic-ir`. This is deliberately *our* syntax, not Wolfram/Maxima, so it matches MS-Project's
`[Field]` custom-field convention and stays closed/safe:

- `[field-name]` → `IRNode::Symbol("field-name")` (resolved to a `FieldId` or a built-in like
  `[work]`, `[duration]`, `[cost]`, `[percent-complete]`, `[start]`, `[finish]`).
- number literals → `Integer`/`Float`/`Rational`; strings → `Str`.
- `+ - * / ^`, comparisons, `and/or/not`, and function calls `NAME(args…)` → the matching
  `IRNode::Apply(sym(head), args)`.
- Anything outside the grammar is a parse error with position — no `eval`, no injection (mirrors the
  Mosaic/closed-grammar security posture).

Built-in field names resolve to the task's scheduling/rollup values; user field names resolve to
their `FieldId`. The parser output is cached per `FieldDef` (parse once, evaluate many).

### 2. Extract the dependency set from the AST

Walk the parsed `IRNode` and collect every `Symbol` (there is no packaged `free_symbols` in the
cas-* crates — a ~10-line walker, templated on `symbolic-vm`'s `substitute` at `vm.rs:238`). The set
of referenced fields *is* the formula's dependencies. This feeds the recalc graph (below). Rollup
fields depend on the rolled-up field across the scope's tasks.

### 3. Bind values and evaluate with `StrictBackend`

For a given task, bind each referenced field to its current `IRNode` value and evaluate:

```rust
let mut vm = VM::new(Box::new(StrictBackend::new()));
for dep in formula.deps { vm.backend.bind(dep.name, value_as_irnode(task, dep)); }
let result = vm.eval(formula.ast.clone());     // Integer/Float/Rational/Str out → FieldValue
```

`StrictBackend` is chosen precisely because it **errors on any unbound name** — a formula that
references a deleted field surfaces as a field error, never a silent wrong number. The result
`IRNode` is converted back to a `FieldValue` (Number/Money/Duration per the field's declared kind).

## Pipeline for a `Rollup` field

A rollup aggregates a field over a scope (children / descendants / assignments). We express it as an
`Apply` over the collected child values and evaluate with the same VM, so rollups and formulas share
one evaluator:

```
Sum      → Apply(sym("Add"),  child_values)
Min/Max  → Apply(sym("Min"|"Max"), child_values)
Average  → Apply(sym("Div"), [Apply(sym("Add"), child_values), Integer(n)])
Count    → Integer(n)
```

`Min`/`Max`/aggregation heads that `StrictBackend` doesn't ship are registered via the `Backend`
trait's handler table (a small extension, not a fork) — the same seam we use for date functions.

## Backend extensions (via the trait, not a fork)

Two capability gaps in the stock backend, both closed by registering handlers on our own backend
built atop `StrictBackend`:

1. **Aggregation heads** — `Min`, `Max`, `Total`/`Sum`, `Average`, `Count` over a `List`/vararg.
2. **Date/duration functions** — `WorkdaysBetween([start],[finish])`, `AddWorkdays([date], n)`,
   `Year/Month/Day`, wired to `task-core`'s calendar module (which itself is built on
   `datetime-core`). Dates are carried as `Integer` (days since epoch) inside the IR.

No change to `symbolic-vm` itself — we compose a `TaskBackend { inner: StrictBackend, … }` that
delegates unknown heads to the extra handler table. If the extensions prove broadly useful they can
later be upstreamed into a `cas-*` crate, but they start folded into `task-core`.

## Incremental recomputation (shared with the scheduler)

Formula/rollup fields form a **dependency graph** (field → fields it reads; rollup → child field
across tasks). We reuse **`directed-graph`** — the *same* crate the CPM scheduler uses:

1. On any field/schedule edit, form the `changed` set (the edited fields + scheduling outputs like
   `[finish]` that formulas may read).
2. `graph.affected_nodes(changed)` → the transitive set of computed fields needing refresh.
3. Recompute them in `graph.topological_sort()` order, so each formula sees fresh inputs.
4. `graph.has_cycle()` guards against a formula that references itself transitively — a validation
   error surfaced to the UI (matching the data-model invariant), never an infinite loop.

The reducer tags each `TaskCommand` with whether it dirties the schedule cache, the formula cache,
or both (see [`task-app-architecture.md`](task-app-architecture.md)), so a rename or a non-computed
edit does the minimal recompute.

## Safety

- Closed grammar; no `eval` of arbitrary strings, no host access — the parser accepts exactly the
  defined operators/functions, and `StrictBackend` cannot reach outside its bound environment.
- Resource-bounded: formula depth and the recalc graph size are capped; a pathological formula fails
  to parse rather than hanging (the `symbolic-ir` depth conventions apply).

## Testing

- Parser: `[field]` refs, literals, every operator, function calls, and rejection of malformed input
  with positions.
- Dependency extraction: the collected `Symbol` set matches the fields a formula reads (incl. nested).
- Evaluation: arithmetic/logical/conditional formulas against hand-computed results; unit conversions
  (Duration/Money) round-trip; unbound-field references error cleanly.
- Rollups: Sum/Min/Max/Average/Count over children, descendants, and assignments; empty-scope edge cases.
- Recalc: editing one input recomputes exactly its transitive dependents in topo order; a formula
  cycle is reported, not looped; large field graphs recompute incrementally, not wholesale.
