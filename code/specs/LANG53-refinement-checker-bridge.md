# LANG53 — TW05-C: Refinement Checker Bridge

## Status

Implemented.  Changes shipped on branch `feat/lang53-refinement-checker-bridge`.

## Motivation

LANG51 added string literals; LANG52 added let*, boolean logic, list ops, and
host I/O.  The self-hosted Twig compiler (TW05 spec) needs more than language
completeness — it needs to *prove* compiler invariants at compile time:

- cursor positions are always `(Index source-len)`;
- register IDs are always `(RegId frame-size)`;
- token spans satisfy `0 ≤ start ≤ end ≤ source-len`.

The infrastructure to discharge these proof obligations already exists:
`lang-refined-types`, `lang-refinement-checker`, and `constraint-vm` were built
in LANG23.  `iir-refinement-pass` (LANG42) wires the per-binding `Checker` into
the AOT pipeline.  But `twig-type-checker` silently dropped refinement
predicates: `RangeInt { lo, hi }` and `MembershipInt { values }` both collapsed
to the unrefined `TwigKind::Int`.

LANG53 wires the per-binding `Checker` and AST-level guard analysis into
`twig-type-checker`, making refined types a first-class compile-time guarantee.

## Design

### `TwigKind::RefinedInt(Predicate)`

A new variant carries the `lang_refined_types::Predicate` through the
type-checker pass.  `RefinedInt` is a subtype of `Int`:

```
RefinedInt(p) ⊆ Int ⊆ Any
```

`type_annotation_to_kind` is updated to produce `RefinedInt` instead of `Int`
for `RangeInt { lo, hi }` and `MembershipInt { values }` annotations:

```rust
TypeAnnotation::RangeInt { lo, hi } =>
    TwigKind::RefinedInt(Predicate::Range {
        lo: Some(*lo), hi: Some(*hi), inclusive_hi: false,
    }),
TypeAnnotation::MembershipInt { values } =>
    TwigKind::RefinedInt(Predicate::Membership {
        values: values.clone()
    }),
```

This means lambda parameters with refined annotations are automatically
bound to `RefinedInt(p)` in scope — no change to `infer_lambda` is required.

### `TypeEnv::fn_param_refinements`

A new field `fn_param_refinements: HashMap<String, Vec<Option<RefinedType>>>`
stores the fully-lowered `RefinedType` per parameter for each top-level
function.  `classify_define` populates this during Pass 1 when the function's
`Lambda` body carries `param_annotations`.

The `RefinedType` here uses `lang_refined_types::Kind::Int` as the base kind
and the same predicate as `TwigKind::RefinedInt`.

### Call-site checking in `infer_apply`

When the callee is a `VarRef` with an entry in `fn_param_refinements`, the
checker runs a per-binding proof obligation for each annotated argument:

```
arg_to_evidence(arg, scope):
  IntLit(n)             → Evidence::Concrete(n)
  VarRef(x) → RefinedInt(p) in scope  → Evidence::Predicated([p])
  VarRef(x) → Int or Any in scope     → Evidence::Unconstrained
  anything else                        → Evidence::Unconstrained
```

Outcomes:
- `ProvenSafe` → silent.
- `ProvenUnsafe(cx)` → `TypeErrorDiagnostic` with counter-example value.
- `Unknown` + `Strict` mode → `TypeErrorDiagnostic`.
- `Unknown` + `Lenient` mode → silent.

### Flow-sensitive narrowing in `infer_if`

`extract_narrowing_facts(guard: &Expr) -> Vec<(String, Predicate)>` analyses a
guard expression and returns per-variable narrowing predicates.

Handled forms:

| Guard | Narrowing fact |
|-------|---------------|
| `(< x k)` | `x: Range { lo: None, hi: Some(k), inclusive_hi: false }` |
| `(<= x k)` | `x: Range { lo: None, hi: Some(k), inclusive_hi: true }` |
| `(> x k)` | `x: Range { lo: Some(k+1), hi: None, inclusive_hi: false }` |
| `(>= x k)` | `x: Range { lo: Some(k), hi: None, inclusive_hi: false }` |
| `(= x k)` | `x: Range { lo: Some(k), hi: Some(k), inclusive_hi: true }` |
| `(and c1 c2 …)` | Combine all child facts; same variable → `Predicate::and` |
| `(not c)` | Negate facts from `c` with `Predicate::not` |
| anything else | Empty (conservative — no narrowing) |

`infer_if` uses the existing `push_frame`/`pop_frame` mechanism to apply
narrowing scopes:

```
True  branch: push_frame; bind narrowing facts; infer then; pop_frame
False branch: push_frame; bind negated facts;  infer else; pop_frame
```

`merge_kind_with_predicate(base, pred)`:
- `Int` or `RefinedInt(_)` → `RefinedInt(pred)` (guard overrides or refines)
- Anything else → `base` unchanged

### `TwigKind::unify` updates

Two integer-like branches that disagree on their predicate widen to `Int`
(not `Any`), since both branches still produce an integer:

```
unify(RefinedInt(p), RefinedInt(p)) = RefinedInt(p)  (same predicate)
unify(RefinedInt(_), RefinedInt(_)) = Int             (different predicates)
unify(RefinedInt(_), Int)           = Int
unify(Int, RefinedInt(_))           = Int
unify(RefinedInt(_), Any)           = Any
unify(a, b) where a != b            = Any             (existing rule)
```

### Scope of v1 — intentional deferrals

- **Inter-procedural narrowing** — `(if (byte? x) (f x) ...)` where `byte?` is
  a user predicate with a declared refinement effect.  Deferred to TW05-D.
- **CFG-based loop invariants** — `FunctionChecker` in `lang-refinement-checker`
  does path-sensitive CFG checking.  LANG53 uses only the per-binding `Checker`
  at call sites and AST-level guard analysis for `if`.  No loops.
- **Return-type annotation checking** — `iir-refinement-pass` (LANG42) handles
  this at the IIR level.  No duplication.
- **`let`/`let*` binding refinements** — deferred to TW05-C continuation.

## Changes

| File | Change |
|------|--------|
| `twig-type-checker/src/kinds.rs` | Add `TwigKind::RefinedInt(Predicate)`; update `mnemonic`, `unify`, `Display`, `type_annotation_to_kind` |
| `twig-type-checker/src/env.rs` | Add `fn_param_refinements` to `TypeEnv`; add `register_fn_refinements` |
| `twig-type-checker/src/narrowing.rs` | **New** — `extract_narrowing_facts`, `merge_kind_with_predicate` |
| `twig-type-checker/src/check.rs` | Update `classify_define`; update `infer_apply` (call-site checking); update `infer_if` (flow narrowing) |
| `twig-type-checker/src/lib.rs` | Add `pub mod narrowing` |
| `twig-type-checker/Cargo.toml` | Add `lang-refined-types`, `lang-refinement-checker` deps; version → 0.5.0 |
| `twig-type-checker/CHANGELOG.md` | Prepend `## [0.5.0]` |

## Tests

- `refined_kind_from_range_annotation` — `(Int 0 128)` annotation → `TwigKind::RefinedInt`
- `refined_kind_from_membership_annotation` — `(Member int 1 2 5)` → `TwigKind::RefinedInt`
- `unrefined_int_annotation_stays_int` — `int` annotation → `TwigKind::Int` (regression)
- `call_site_literal_in_range_no_error` — `(ascii-info 42)` with `(Int 0 128)` → no error
- `call_site_literal_out_of_range_error` — `(ascii-info 200)` → `TypeErrorDiagnostic`
- `call_site_unconstrained_lenient_silent` — unresolved arg in lenient mode → no error
- `call_site_unconstrained_strict_error` — same in strict mode → error
- `narrowing_lt_proves_call` — `(if (< x 128) (ascii-info x) ...)` → no error in then
- `narrowing_and_both_bounds` — `(if (and (>= x 0) (< x 128)) (ascii-info x) ...)` → no error
- `narrowing_not_in_else` — `(if (< x 128) ... (ascii-info x))` → error in else branch
- `refined_kinds_unify_to_int` — `(if c 42 200)` from different `RefinedInt` → `Int`
- `no_narrowing_for_non_numeric` — bool/string guards do not crash narrowing
