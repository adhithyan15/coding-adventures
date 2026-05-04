# `lang-refinement-checker`

**LANG23 PR 23-C** — the refinement proof-obligation checker.

Takes `RefinedType` annotations from the IIR, lowers their predicates to
`ConstraintInstructions`, runs them through `constraint-vm`, and classifies
the solver's answer into one of the three LANG23 outcomes.

---

## Architecture

```
lang-refined-types          (RefinedType, Predicate, Kind)
        │
lang-refinement-checker     (this crate)
        │  builds constraint programs via ProgramBuilder
        │  drives constraint_vm::check_sat / get_model
        │
constraint-vm ──► constraint-engine ──► SAT / LIA tactics
```

## The three outcomes

```text
PROVEN_SAFE    → strip the runtime check; narrow the downstream type
PROVEN_UNSAFE  → compile error with concrete counter-example value
UNKNOWN        → emit a runtime check; warn; proceed in lenient mode
```

## Proof obligation

For each annotated binding, the checker runs a *refutation query*:

> Does there exist a value `x` consistent with the evidence that violates
> the annotation predicate?

Formally: `check_sat(E(x) ∧ ¬P(x))`.

| Solver | LANG23 outcome |
|--------|----------------|
| UNSAT  | `PROVEN_SAFE` |
| SAT(m) | `PROVEN_UNSAFE` (m contains the counter-example) |
| UNKNOWN | `UNKNOWN` |

## Evidence

| Variant | When to use | Example |
|---------|-------------|---------|
| `Concrete(v)` | Literal at call site | `(define x : (Int 1 256) 25)` |
| `Predicated(preds)` | Guard/annotation narrows value | `if n < 128 then ascii-info(n)` |
| `Unconstrained` | Source unknown at compile time | `(define x : (Int 1 256) (read-int))` |

## Usage

```rust
use lang_refined_types::{RefinedType, Kind, Predicate};
use lang_refinement_checker::{Checker, Evidence, CheckOutcome};

let annotation = RefinedType::refined(
    Kind::Int,
    Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
);

let mut checker = Checker::new();

// Literal 64 is in [0, 128) → proven safe.
assert!(checker.check(&annotation, &Evidence::Concrete(64)).is_safe());

// Literal 200 is NOT in [0, 128) → proven unsafe.
let out = checker.check(&annotation, &Evidence::Concrete(200));
assert!(out.is_unsafe());
assert_eq!(out.counter_example().unwrap().value, 200);

// Guard: evidence says n ∈ [0, 50) — which is ⊆ [0, 128) → proven safe.
let guard = vec![Predicate::Range { lo: Some(0), hi: Some(50), inclusive_hi: false }];
assert!(checker.check(&annotation, &Evidence::Predicated(guard)).is_safe());

// Unconstrained input → unknown → caller emits runtime check.
assert!(checker.check(&annotation, &Evidence::Unconstrained).is_unknown());
```

## Batch checking

```rust
use lang_refinement_checker::{Obligation, Evidence, check_all};

let obligations = vec![
    Obligation::new("ascii-index", annotation.clone(), Evidence::Concrete(64)),
    Obligation::new("read-int",    annotation.clone(), Evidence::Unconstrained),
];

for (label, outcome) in check_all(&obligations) {
    println!("{label}: {outcome:?}");
}
```

## Mode integration

This crate is **mode-agnostic** — it always returns one of the three outcomes.
The caller decides what to do with `UNKNOWN`:

- `--refinement-mode=lenient` (default): emit runtime check, warn.
- `--refinement-mode=strict`: treat `UNKNOWN` as a compile error.
- `--refinement-mode=off`: skip the checker entirely.
