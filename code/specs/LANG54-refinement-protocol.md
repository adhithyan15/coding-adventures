# LANG54 — Generic Refinement-Type Checker Protocol

**Status:** planned  
**Branch:** `feat/lang54-refinement-protocol`  
**Depends on:** LANG53 (twig-type-checker v0.5.0), LANG23 (lang-refinement-checker)

---

## Motivation

LANG53 wired `lang-refinement-checker` into `twig-type-checker` with ~150 lines
of Twig-specific glue.  The pattern is valuable — call-site proof obligations,
flow-sensitive narrowing from `if`-guards — but it cannot be reused by other
language frontends (Nib, future Twig-derivative languages, etc.) without copying
and adapting those 150 lines.

LANG54 extracts the reusable core into a new `lang-refinement-protocol` crate
so that Language X gets call-site checking and flow-sensitive narrowing by
implementing three bridge methods.

---

## New crate: `lang-refinement-protocol`

### Public API

#### `RefinementBridge` trait

The single trait a language must implement.  It is generic over the language's
expression (`Expr`) and kind (`Kind`) types.

```rust
pub trait RefinementBridge {
    /// The language's AST expression type at call sites and guard positions.
    type Expr;
    /// The language's kind/type representation (result of type inference).
    type Kind: Clone;

    /// Classify a call-site argument expression as `Evidence` for the solver.
    ///
    /// `expr` — the argument AST node.  
    /// `inferred_kind` — the kind already computed for this expression (avoids
    ///   a second traversal).  `None` when the caller did not pre-compute it.
    fn evidence_for(&self, expr: &Self::Expr, inferred_kind: Option<&Self::Kind>) -> Evidence;

    /// Extract narrowing facts from a guard expression.
    ///
    /// Returns `(variable_name, predicate)` pairs: "if this guard is true,
    /// `variable_name` satisfies `predicate`."  Returns `[]` when the guard
    /// implies nothing useful (conservative: no narrowing).
    fn narrowing_facts(&self, guard: &Self::Expr) -> Vec<(String, Predicate)>;

    /// Narrow a variable's kind by adding a guard predicate.
    ///
    /// - `Int + p` → `RefinedInt(p)`    (add refinement)
    /// - `RefinedInt(q) + p` → `RefinedInt(and(q, p))`  (intersect)
    /// - `Bool / Str / …` → unchanged   (non-numeric kinds)
    fn narrow_kind(&self, base: &Self::Kind, pred: Predicate) -> Self::Kind;
}
```

#### `check_call_site_refinements` — generic call-site checker

```rust
pub fn check_call_site_refinements<B>(
    bridge: &B,
    callee_name: &str,
    call_site_line: usize,
    call_site_column: usize,
    arg_exprs: &[B::Expr],
    arg_kinds: &[B::Kind],
    param_refinements: &[Option<RefinedType>],
    mode: RefinementMode,
) -> Vec<RefinementDiagnostic>
where
    B: RefinementBridge;
```

Drives `Checker::check` for each annotated argument:

| argument | evidence | outcome |
|---|---|---|
| `evidence_for(arg)` = `Concrete(n)` | `Concrete(n)` | exact check |
| `evidence_for(arg)` = `Predicated(ps)` | `Predicated(ps)` | entailment check |
| `evidence_for(arg)` = `Unconstrained` | `Unconstrained` | `Unknown` |
| `ProvenUnsafe(cx)` | → `RefinementDiagnostic` always |
| `Unknown` + `Strict` | → `RefinementDiagnostic` |
| `Unknown` + `Lenient` | → silent |
| `ProvenSafe` | → silent |

#### `NarrowedBindings<K>` + `compute_if_narrowing` — generic narrowing

```rust
pub struct NarrowedBindings<K> {
    /// (variable, narrowed-kind) bindings for the true branch.
    pub true_branch: Vec<(String, K)>,
    /// (variable, narrowed-kind) bindings for the false branch.
    pub false_branch: Vec<(String, K)>,
}

pub fn compute_if_narrowing<B>(
    bridge: &B,
    guard: &B::Expr,
    scope_lookup: impl Fn(&str) -> Option<B::Kind>,
) -> NarrowedBindings<B::Kind>
where
    B: RefinementBridge;
```

Calls `bridge.narrowing_facts(guard)`, looks up each variable in the provided
scope, and delegates to `bridge.narrow_kind` for true-branch narrowing and
`bridge.narrow_kind(base, Predicate::not(pred))` for false-branch narrowing.

#### Supporting types

```rust
/// An error or warning produced by the refinement checker at a specific source location.
pub struct RefinementDiagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Whether `Unknown` outcomes produce diagnostics (Strict) or are silent (Lenient).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefinementMode { Lenient, Strict }
```

#### Re-exports

From `lang-refinement-checker`:
- `Evidence`, `CheckOutcome`, `CounterExample`, `Checker`, `Obligation`, `check_all`

From `lang-refined-types`:
- `Predicate`, `RefinedType`, `Kind` as `RefKind`

This means a language only needs `lang-refinement-protocol` in its Cargo.toml.

---

## Refactored crate: `twig-type-checker` → v0.6.0

### New file: `src/bridge.rs`

`TwigRefinementBridge` implementing `RefinementBridge`:

```rust
pub struct TwigRefinementBridge;

impl RefinementBridge for TwigRefinementBridge {
    type Expr = twig_parser::Expr;
    type Kind = TwigKind;

    fn evidence_for(&self, expr: &Expr, inferred_kind: Option<&TwigKind>) -> Evidence {
        match expr {
            Expr::IntLit(lit) => Evidence::Concrete(lit.value as i128),
            Expr::VarRef(_) => match inferred_kind {
                Some(TwigKind::RefinedInt(p)) => Evidence::Predicated(vec![p.clone()]),
                _ => Evidence::Unconstrained,
            },
            _ => Evidence::Unconstrained,
        }
    }

    fn narrowing_facts(&self, guard: &Expr) -> Vec<(String, Predicate)> {
        extract_narrowing_facts(guard)   // existing narrowing.rs function
    }

    fn narrow_kind(&self, base: &TwigKind, pred: Predicate) -> TwigKind {
        merge_kind_with_predicate(base, pred)  // existing narrowing.rs function
    }
}
```

### Updated `src/check.rs`

`infer_apply` — replace the 40-line manual loop with:
```rust
let diags = check_call_site_refinements(
    &TwigRefinementBridge,
    callee_name,
    app.line,
    app.column,
    &app.args,
    &arg_kinds,
    param_refinements,
    mode.into(),
);
for d in diags {
    errors.push(TypeErrorDiagnostic { message: d.message, line: d.line, column: d.column });
}
```

`infer_if` — replace the 30-line manual narrowing block with:
```rust
let narrowed = compute_if_narrowing(
    &TwigRefinementBridge,
    &if_expr.cond,
    |var| scope.lookup(var).cloned().or_else(|| env.lookup_global(var).cloned()),
);
scope.push_frame();
for (var, kind) in narrowed.true_branch { scope.bind(&var, kind); }
let then_kind = infer_expr(&if_expr.then_branch, env, scope, mode, errors);
scope.pop_frame();

scope.push_frame();
for (var, kind) in narrowed.false_branch { scope.bind(&var, kind); }
let else_kind = infer_expr(&if_expr.else_branch, env, scope, mode, errors);
scope.pop_frame();
```

### Dependency changes

`twig-type-checker/Cargo.toml`:
- **Add**: `lang-refinement-protocol = { path = "../lang-refinement-protocol" }`
- **Remove**: `lang-refinement-checker` (re-exported through protocol)
- **Keep**: `lang-refined-types` (for `RefinedType`/`Predicate` in TwigKind + bridge)

---

## Adoption guide for Language X

To add refinement checking to Language X:

1. **Add dep**: `lang-refinement-protocol = { path = "../lang-refinement-protocol" }`

2. **Implement bridge** (~50 lines):
   ```rust
   pub struct LangXBridge;
   impl RefinementBridge for LangXBridge {
       type Expr = LangXExpr;
       type Kind = LangXKind;
       fn evidence_for(&self, expr: &LangXExpr, kind: Option<&LangXKind>) -> Evidence { ... }
       fn narrowing_facts(&self, guard: &LangXExpr) -> Vec<(String, Predicate)> { ... }
       fn narrow_kind(&self, base: &LangXKind, pred: Predicate) -> LangXKind { ... }
   }
   ```

3. **Call-site checking** (~10 lines in your apply/call handler):
   ```rust
   let diags = check_call_site_refinements(&LangXBridge, callee, line, col,
       args, kinds, param_refinements, mode);
   ```

4. **Flow narrowing** (~10 lines in your if/cond handler):
   ```rust
   let narrowed = compute_if_narrowing(&LangXBridge, guard, |v| scope.lookup(v));
   // apply narrowed.true_branch / narrowed.false_branch to your scope frames
   ```

Total: ~70 lines of language-specific code for full refinement checking.

---

## Tests

### `lang-refinement-protocol` (≥ 10 unit tests, mock bridge)

1. `check_call_site_concrete_in_range` → no diagnostics
2. `check_call_site_concrete_out_of_range` → diagnostic
3. `check_call_site_unconstrained_lenient` → no diagnostics
4. `check_call_site_unconstrained_strict` → diagnostic
5. `check_call_site_no_refinements` → no diagnostics (empty param_refinements)
6. `check_call_site_predicated_proven_safe` → no diagnostics
7. `check_call_site_predicated_proven_unsafe` → diagnostic
8. `compute_if_narrowing_extracts_true_and_false_branches`
9. `compute_if_narrowing_no_facts_yields_empty`
10. `compute_if_narrowing_variable_not_in_scope_skipped`
11. `narrowed_bindings_false_branch_is_negated`

### `twig-type-checker` regression (all 74 existing tests must still pass)

---

## Version bumps

| Crate | Old | New | Reason |
|---|---|---|---|
| `lang-refinement-protocol` | — | 0.1.0 | New crate |
| `twig-type-checker` | 0.5.0 | 0.6.0 | Refactored to use protocol; behaviour identical |

---

## Commit sequence

1. `docs(specs)`: `LANG54-refinement-protocol.md`
2. `feat(lang-refinement-protocol)`: `RefinementBridge` trait + supporting types
3. `feat(lang-refinement-protocol)`: `check_call_site_refinements` free function
4. `feat(lang-refinement-protocol)`: `compute_if_narrowing` + `NarrowedBindings`
5. `test(lang-refinement-protocol)`: 11 unit tests with mock bridge
6. `feat(twig-type-checker)`: `TwigRefinementBridge` + refactored check.rs
7. `docs`: CHANGELOG + README updates
