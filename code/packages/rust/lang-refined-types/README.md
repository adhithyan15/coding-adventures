# `lang-refined-types`

**LANG23 PR 23-A** — `RefinedType`, `Predicate`, and `Kind` data types with
predicate algebra.  No solver — pure data.

---

## What is a refinement type?

A refinement type is a base *kind* (`i64`, `bool`, …) plus a *predicate* that
restricts which values of that kind are valid:

```text
i64 where x ∈ [0, 128)     — Int, Range predicate
i64 where x ∈ {1, 2, 3}    — Int, Membership predicate
i64                          — unrefined (no predicate; same as LANG22)
```

## Predicate vocabulary

| Variant | Meaning |
|---------|---------|
| `Range { lo, hi, inclusive_hi }` | `lo ≤ x < hi` (or `≤ hi` if inclusive) |
| `Membership { values }` | `x ∈ {v₁, v₂, …}` |
| `And(preds)` | conjunction |
| `Or(preds)` | disjunction |
| `Not(pred)` | negation |
| `LinearCmp { coefs, op, rhs }` | `Σ aᵢxᵢ ⊙ c` |
| `Opaque { display }` | user-supplied; degrades to runtime check |

## Smart constructors + algebra

```rust
use lang_refined_types::Predicate;

// Smart constructors simplify on construction.
let p = Predicate::and(vec![
    Predicate::Range { lo: Some(0), hi: Some(100), inclusive_hi: false },
    Predicate::Range { lo: Some(50), hi: Some(200), inclusive_hi: false },
]);
// After simplify(): Range { lo: 50, hi: 100, … }

let q = Predicate::not(Predicate::not(p.clone()));
// After not(not(p)): p  (double negation elimination)
```

`Predicate::simplify()` merges adjacent `Range` predicates in `And` nodes and
deduplicates operands.  `Predicate::canonicalise()` produces sorted, deduped
`And`/`Or` operands for stable `Hash`.

## Lowering to `constraint-core`

```rust
let p = Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false };
let core = p.to_constraint_predicate("x").unwrap();
// Produces: And(Ge(x, 0), Lt(x, 128))
```

`Opaque` lowers to `None` — the caller emits a runtime check.

## Usage

```rust
use lang_refined_types::{RefinedType, Kind, Predicate};

// Unrefined — same semantics as LANG22 type hint "i64".
let unrefined = RefinedType::unrefined(Kind::Int);

// Refined — solver fires on proof obligations.
let refined = RefinedType::refined(
    Kind::Int,
    Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
);

println!("{refined}");  // i64 where x ∈ [0, 128)
```
