# constraint-core

**LANG24 PR 24-A** — predicate AST + sort/logic/theory enums + normalisation
passes for the generic Constraint-VM.

This is the *data-crate foundation* for the Constraint-VM stack — pure data and
algorithms, no I/O, no solver state.  Future PRs (`constraint-instructions`,
`constraint-engine`, `constraint-vm`, every consumer) build against this.

It mirrors the layering of the Logic-VM stack:

| Constraint-VM       | Logic-VM                |
|---------------------|-------------------------|
| `constraint-core`   | `logic-core`            |
| `constraint-instructions` | `logic-instructions` |
| `constraint-engine` | `logic-engine`          |
| `constraint-vm`     | `logic-vm`              |

---

## Position in the stack

```
SMT-LIB / refinement-types / hand-written predicates
        ↓
   constraint-core           ← this crate
        ↓
   constraint-instructions   (PR 24-B)
        ↓
   constraint-engine         (PR 24-C)
        ↓
   constraint-vm             (PR 24-D)
        ↓
  consumers (LANG23 refinement-types, planners, type-state checkers, …)
```

---

## What lives here

| Type | Description |
|------|-------------|
| `Predicate`     | The recursive constraint-language AST (24 variants, `#[non_exhaustive]`) |
| `Sort`          | Type system: `Bool`, `Int`, `Real`, `BitVec(width)`, `Array { idx, val }`, `Uninterpreted(name)` |
| `Logic`         | Declared theory profile of a program: `QF_LIA`, `QF_LRA`, `QF_BV`, `QF_AUFLIA`, `LIA`, `ALL`, … |
| `Theory`        | Single-tactic capability: `Bool`, `LIA`, `LRA`, `Arrays`, `BitVectors`, `EUF`, `Strings`, `NRA`, `FP` |
| `Rational`      | Hand-rolled `num/den` rational for the `Real` literal variant — no `num` dependency |
| `SortError`     | Typed error returned by `infer_sort` (mismatch, unknown var, etc.) |

All public enums are `#[non_exhaustive]` so v2/v3 theories can plug in without
breaking downstream matchers.

---

## Core algorithms

### Smart constructors (simplify on construction)

- `Predicate::and(parts)` — drops `Bool(true)`, short-circuits on `Bool(false)`,
  flattens nested `And`, unwraps singletons, returns `Bool(true)` for empty.
- `Predicate::or(parts)` — mirror of the above.
- `Predicate::not(p)` — folds `Not(Bool(b))` and eliminates `Not(Not(p))`.

### Normalisation passes

- `Predicate::to_nnf()` — **negation normal form**.  Pushes `Not` down to atoms
  via De Morgan, eliminates double negation, and inverts atomic comparisons
  (`Not(Lt(a,b))` → `Ge(a,b)`, etc.).  `Implies` and `Iff` are first
  desugared to `Or`/`And` of their parts.
- `Predicate::to_cnf()` — **conjunctive normal form**.  Runs `to_nnf()` then
  distributes `Or` over `And` (naive — exponential worst case).  Acceptable
  for the small predicates that arise in refinement-type checking; more
  scalable variants live in `constraint-engine`.
- `Predicate::simplify()` — constant folds `Ite` with constant conditions and
  deduplicates `And`/`Or` operands.

### Free-variable extraction

`Predicate::free_vars()` returns a `BTreeSet<String>` of all variable names
referenced by `Var(name)`, *minus* anything bound by an enclosing `Forall` or
`Exists`.  `BTreeSet` for deterministic iteration order.

### Sort inference

`infer_sort(predicate, env)` walks a predicate against a `SortEnv` (a
`HashMap<String, Sort>`) and returns the inferred top-level sort.  Errors (sort
mismatch, unknown variable, arity mismatch on `Apply`) come back as `SortError`.
Quantifiers extend the env for the body's scope.

---

## Display format

Predicates `Display` as Lisp-style s-expressions:

```
(and (>= x 1) (<= x 100))
(forall ((y Int)) (=> (>= y 0) (>= (+ y 1) 1)))
(ite (> x 0) (* x 2) 0)
```

This is a debugging aid and the basis for an eventual SMT-LIB exporter
(LANG24 PR 24-E, separate crate).

---

## Example

```rust
use constraint_core::{Predicate, Rational, Sort, infer_sort};
use std::collections::HashMap;

// Build  1 ≤ x ∧ x ≤ 100
let p = Predicate::and(vec![
    Predicate::Le(Box::new(Predicate::Int(1)), Box::new(Predicate::Var("x".into()))),
    Predicate::Le(Box::new(Predicate::Var("x".into())), Box::new(Predicate::Int(100))),
]);

assert_eq!(p.to_string(), "(and (<= 1 x) (<= x 100))");

// NNF, CNF round-trips
let nnf = p.clone().to_nnf();
let cnf = p.clone().to_cnf();

// Sort inference
let mut env = HashMap::new();
env.insert("x".to_string(), Sort::Int);
assert_eq!(infer_sort(&p, &env).unwrap(), Sort::Bool);
```

---

## Dependencies

**None.**  Pure data + algorithms.  See `required_capabilities.json`.

---

## Tests

39 unit tests covering smart constructors, normalisation passes, free-variable
extraction, sort inference (positive + negative), `Rational` GCD reduction,
and `Display` round-trips.

```sh
cargo test -p constraint-core
```

---

## Roadmap

- **PR 24-B `constraint-instructions`** — declarative tactic & solver-step
  instruction set built on top of `Predicate`.
- **PR 24-C `constraint-engine`** — Nelson-Oppen tactic composition + DPLL(T)
  scaffolding consuming the instructions.
- **PR 24-D `constraint-vm`** — full executable Constraint-VM packaging the
  core/instructions/engine layers.
- **PR 24-E `constraint-smtlib`** — SMT-LIB import/export adapter.
- **LANG23 integration** — refinement-type checker switches from its inline
  predicate AST to `constraint-core::Predicate`.
