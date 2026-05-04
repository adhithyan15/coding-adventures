# `lang-refinement-checker`

**LANG23 PRs 23-C + 23-D + 23-F** — the refinement proof-obligation checker.

Takes `RefinedType` annotations from the IIR, lowers their predicates to
`ConstraintInstructions`, runs them through `constraint-vm`, and classifies
the solver's answer into one of the three LANG23 outcomes.

Three APIs are provided:

| API | Module | PR | Scope |
|-----|--------|----|-------|
| `Checker` | (crate root) | 23-C | Per-binding: checks one proof obligation given concrete/predicated/unconstrained evidence. |
| `FunctionChecker` | `function_checker` | 23-D | Function-scope: walks a CFG, accumulates guard predicates path-by-path, checks each return site. |
| `ModuleChecker` | `module_checker` | 23-F | Module-scope: extends function-scope checking with cross-function call-site reasoning; uses a `ModuleScope` registry for per-symbol opt-out. |

---

## Architecture

```
lang-refined-types          (RefinedType, Predicate, Kind)
        │
lang-refinement-checker     (this crate)
        │  PR 23-C: Checker — per-binding proof obligations
        │  PR 23-D: FunctionChecker — CFG-based path-sensitive analysis
        │  PR 23-F: ModuleChecker — cross-function call-site reasoning
        │  all lower via ProgramBuilder → constraint-vm
        │
constraint-vm ──► constraint-engine ──► SAT / LIA tactics
```

## The three outcomes

```text
PROVEN_SAFE    → strip the runtime check; narrow the downstream type
PROVEN_UNSAFE  → compile error with concrete counter-example value
UNKNOWN        → emit a runtime check; warn; proceed in lenient mode
```

## PR 23-C: Per-binding checker

For each annotated binding, the checker runs a *refutation query*:

> Does there exist a value `x` consistent with the evidence that violates
> the annotation predicate?

Formally: `check_sat(E(x) ∧ ¬P(x))`.

| Solver | LANG23 outcome |
|--------|----------------|
| UNSAT  | `PROVEN_SAFE` |
| SAT(m) | `PROVEN_UNSAFE` (m contains the counter-example) |
| UNKNOWN | `UNKNOWN` |

### Evidence

| Variant | When to use | Example |
|---------|-------------|---------|
| `Concrete(v)` | Literal at call site | `(define x : (Int 1 256) 25)` |
| `Predicated(preds)` | Guard/annotation narrows value | `if n < 128 then ascii-info(n)` |
| `Unconstrained` | Source unknown at compile time | `(define x : (Int 1 256) (read-int))` |

### Usage

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

### Batch checking

```rust
use lang_refinement_checker::{Obligation, Evidence, check_all};
# use lang_refined_types::{RefinedType, Kind, Predicate};
# let annotation = RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false });

let obligations = vec![
    Obligation::new("ascii-index", annotation.clone(), Evidence::Concrete(64)),
    Obligation::new("read-int",    annotation.clone(), Evidence::Unconstrained),
];

for (label, outcome) in check_all(&obligations) {
    println!("{label}: {outcome:?}");
}
```

---

## PR 23-D: Function-scope checker

Extends `Checker` to handle entire function bodies.  Walks a tree-structured
CFG, accumulates guard predicates along each path (the same machine TypeScript
uses for control-flow narrowing), and checks each return site.

### The CFG model

```
CfgNode::Branch { guard, then_node, else_node }   — a conditional branch
CfgNode::Return(ReturnValue)                       — a return statement

ReturnValue::Literal(i128)   — compile-time constant
ReturnValue::Variable(name)  — a function parameter or local
```

### clamp-byte example

```scheme
(define (clamp-byte (x : int) -> (Int 0 256))
  (cond ((< x 0)   0)
        ((> x 255) 255)
        (else      x)))
```

```rust
use lang_refined_types::{Kind, Predicate, RefinedType};
use lang_refinement_checker::function_checker::{
    BranchGuard, CfgNode, FunctionChecker, FunctionSignature, ReturnValue,
};

let sig = FunctionSignature {
    params: vec![("x".to_string(), RefinedType::unrefined(Kind::Int))],
    return_type: RefinedType::refined(
        Kind::Int,
        Predicate::Range { lo: Some(0), hi: Some(256), inclusive_hi: false },
    ),
};

//  x < 0  ≡  Range { hi: Some(0), inclusive_hi: false }
//  x ≥ 256 ≡  Range { lo: Some(256) }
let cfg = CfgNode::Branch {
    guard: BranchGuard {
        var: "x".to_string(),
        predicate: Predicate::Range { lo: None, hi: Some(0), inclusive_hi: false },
    },
    then_node: Box::new(CfgNode::Return(ReturnValue::Literal(0))),
    else_node: Box::new(CfgNode::Branch {
        guard: BranchGuard {
            var: "x".to_string(),
            predicate: Predicate::Range { lo: Some(256), hi: None, inclusive_hi: false },
        },
        then_node: Box::new(CfgNode::Return(ReturnValue::Literal(255))),
        else_node: Box::new(CfgNode::Return(ReturnValue::Variable("x".to_string()))),
    }),
};

let mut fc = FunctionChecker::new();
let result = fc.check_function(&sig, &cfg);

assert!(result.all_proven_safe());
// Three return sites, all proven safe — no runtime checks needed.
assert_eq!(result.return_sites.len(), 3);
assert_eq!(result.runtime_check_count(), 0);
```

### How it works — the three paths

| Path | Guards for x | Return | Evidence | Outcome |
|------|-------------|--------|----------|---------|
| then₁ | `x < 0` | `Literal(0)` | `Concrete(0)` | `0 ∈ [0,256)` → Safe |
| then₂ | `¬(x<0)`, `x≥256` | `Literal(255)` | `Concrete(255)` | `255 ∈ [0,256)` → Safe |
| else₂ | `¬(x<0)`, `¬(x≥256)` | `Variable("x")` | `Predicated{__v≥0, __v<256}` | UNSAT refutation → Safe |

For `Variable("x")`, the accumulated scope predicates for `"x"` are remapped to
`"__v"` via `substitute_var` before passing to `Checker::check_predicated`.

### Violation detection

If the return type were `(Int 0 200)` instead, `return 255` would produce
`ProvenUnsafe(CounterExample { value: 255, ... })`.

---

## PR 23-F: Module-scope checker

Extends `FunctionChecker` to reason across function call boundaries within a
module.  A `ModuleScope` registry maps function names to their `FunctionSignature`s;
any function absent from the scope is silently skipped (per-symbol opt-out via
`: any`).

### The `latin1-decode` / `decode` example

```scheme
;; module: text/ascii.twig
(define (decode (codepoint : (Int 0 128))) ...)

(define (latin1-decode (cp : (Int 0 256)))
  (if (< cp 128)
      (decode cp)              ; solver: cp narrowed to [0, 128) by guard ✓
      (latin1-fallback cp)))  ; latin1-fallback not in scope → skipped
```

```rust
use lang_refined_types::{Kind, Predicate, RefinedType};
use lang_refinement_checker::function_checker::{BranchGuard, FunctionSignature, ReturnValue};
use lang_refinement_checker::module_checker::{CallArg, ModuleCfgNode, ModuleChecker, ModuleScope};

let decode_sig = FunctionSignature {
    params: vec![(
        "codepoint".into(),
        RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false }),
    )],
    return_type: RefinedType::unrefined(Kind::Int),
};

let mut scope = ModuleScope::new();
scope.register("decode", decode_sig);

let latin1_sig = FunctionSignature {
    params: vec![(
        "cp".into(),
        RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(0), hi: Some(256), inclusive_hi: false }),
    )],
    return_type: RefinedType::unrefined(Kind::Int),
};

let cfg = ModuleCfgNode::Branch {
    guard: BranchGuard { var: "cp".into(), predicate: Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false } },
    then_node: Box::new(ModuleCfgNode::Call {
        callee: "decode".into(),
        args: vec![CallArg::Variable("cp".into())],
        next: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".into()))),
    }),
    else_node: Box::new(ModuleCfgNode::Return(ReturnValue::Variable("cp".into()))),
};

let mut checker = ModuleChecker::new(scope);
let result = checker.check_function(&latin1_sig, &cfg);

// The call to `decode cp` in the then-branch is proven safe by the guard.
assert!(result.all_call_sites_proven_safe());
```

### How it works

| Path | Scope for `cp` | Call arg evidence | Callee annotation | Outcome |
|------|----------------|-------------------|-------------------|---------|
| then (`cp < 128`) | `[0,256) ∩ cp<128` | `Predicated([Range{0,256}, Range{None,128}])` | `(Int 0 128)` | UNSAT refutation → **ProvenSafe** |
| else (`cp ≥ 128`) | `(cp ≥ 128)` | — | `latin1-fallback` not in scope | *skipped* |

---

## Mode integration

This crate is **mode-agnostic** — it always returns one of the three outcomes.
The caller decides what to do with `UNKNOWN`:

- `--refinement-mode=lenient` (default): emit runtime check, warn.
- `--refinement-mode=strict`: treat `UNKNOWN` as a compile error.
- `--refinement-mode=off`: skip the checker entirely.
