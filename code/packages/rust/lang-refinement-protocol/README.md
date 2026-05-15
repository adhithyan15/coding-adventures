# lang-refinement-protocol

Generic protocol for wiring the refinement-type solver into any language's
type checker (LANG54).

## What it does

Extracts the 150-line language-specific glue from `twig-type-checker` into two
generic free functions backed by a three-method trait.  Any language type
checker that implements `RefinementBridge` gets call-site proof obligations and
flow-sensitive narrowing for free.

## Quick start

### 1. Implement `RefinementBridge` for your language

```rust
use lang_refinement_protocol::{Evidence, Predicate, RefinementBridge};

struct MyBridge;

impl RefinementBridge for MyBridge {
    type Expr = MyExpr;
    type Kind = MyKind;

    fn evidence_for(&self, expr: &MyExpr, kind: Option<&MyKind>) -> Evidence {
        match expr {
            MyExpr::IntLit(n) => Evidence::Concrete(*n as i128),
            MyExpr::Var(_) => match kind {
                Some(MyKind::RefinedInt(p)) => Evidence::Predicated(vec![p.clone()]),
                _ => Evidence::Unconstrained,
            },
            _ => Evidence::Unconstrained,
        }
    }

    fn narrowing_facts(&self, guard: &MyExpr) -> Vec<(String, Predicate)> {
        // analyse (< x 128), (>= x 0), (and …), (not …) here
        vec![]
    }

    fn narrow_kind(&self, base: &MyKind, pred: Predicate) -> MyKind {
        match base {
            MyKind::Int => MyKind::RefinedInt(pred),
            MyKind::RefinedInt(existing) => {
                MyKind::RefinedInt(Predicate::and(vec![existing.clone(), pred]))
            }
            other => other.clone(),
        }
    }
}
```

### 2. Wire into your function-application handler

```rust
use lang_refinement_protocol::{check_call_site_refinements, RefinementMode};

let diags = check_call_site_refinements(
    &MyBridge,
    callee_name,
    call_line, call_column,
    &arg_exprs,        // &[MyExpr]
    &arg_kinds,        // &[MyKind]
    &param_refinements, // &[Option<RefinedType>]
    RefinementMode::Strict,
);
for d in diags {
    errors.push(MyError { msg: d.message, line: d.line, col: d.column });
}
```

### 3. Wire into your if-expression handler

```rust
use lang_refinement_protocol::compute_if_narrowing;

let narrowed = compute_if_narrowing(
    &MyBridge,
    &guard_expr,
    |var| scope.lookup(var).cloned(),
);

// True branch
scope.push_frame();
for (var, kind) in narrowed.true_branch { scope.bind(&var, kind); }
let then_kind = infer(then_branch);
scope.pop_frame();

// False branch
scope.push_frame();
for (var, kind) in narrowed.false_branch { scope.bind(&var, kind); }
let else_kind = infer(else_branch);
scope.pop_frame();
```

## API

### `RefinementBridge` trait

| Method | Description |
|---|---|
| `evidence_for(expr, kind)` | Classify a call-site arg as `Evidence` |
| `narrowing_facts(guard)` | Extract `(var, predicate)` pairs from a guard expression |
| `narrow_kind(base, pred)` | Merge a base kind with a narrowing predicate |

### `check_call_site_refinements`

Generic free function.  Drives `Checker::check` per annotated argument.
Returns `Vec<RefinementDiagnostic>`.

### `compute_if_narrowing`

Generic free function.  Computes `NarrowedBindings<K>` for the true and
false branches of an `if`-expression.

### `RefinementMode`

```rust
pub enum RefinementMode { Lenient, Strict }
```

`Strict` → `Unknown` outcomes are errors.  `Lenient` → `Unknown` is silent.

## Reference implementation

`twig-type-checker/src/bridge.rs` contains `TwigRefinementBridge` — the
complete, commented reference implementation for the Twig language.

## Pipeline position

```
lang-refined-types
    │
lang-refinement-protocol   ← this crate
    │
language type-checker (Language X implements RefinementBridge)
```
