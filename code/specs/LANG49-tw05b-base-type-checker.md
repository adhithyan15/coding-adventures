# LANG49 — TW05-B: Base Static Type Checker for Twig

## Status

**Proposed → Implemented (2026-05-14)**

Depends on: LANG48 (TW05-A typed syntax — parser, formatter, LSP packages)

---

## Motivation

LANG48 (TW05-A) taught the Twig parser to understand type annotations
(`TypeAnnotation`), type aliases (`TypeAlias`), record definitions
(`RecordDef`), union definitions (`UnionDef`), and pattern-matching
expressions (`Expr::Match`).  The AST now *carries* type information, but
nothing *checks* it — annotations are stored and erased to `Any` during IIR
lowering.

TW05-B adds the first enforcement layer: a static type checker that walks a
parsed `Program`, builds a type environment from all top-level declarations,
infers a base kind for every `Expr`, and reports violations as
`TypeErrorDiagnostic` spans.  The output is a `TypedProgram` pairing the
original AST with the populated `TypeEnv`, usable by TW05-C (refinement
checking) without re-traversal.

---

## Non-Goals (TW05-C territory)

- **Refinement solver**: range / membership predicates (`RangeInt`, `MembershipInt`)
  are recognised and their base kind extracted, but the constraint engine is not
  invoked.  Opaque predicates remain `Any`.
- **Type-level computation**: dependent-type expressions like `(fn (len) (Int 0 len))`
  are stored as `Opaque` — TW05-B resolves them to `Any`.
- **Cross-module type checking**: imports are recorded in `ModuleInfo`; they are
  validated against the declared export list in the same compilation unit but
  cross-unit resolution is deferred.

---

## Base Kinds

TW05-B introduces the `TwigKind` enum:

| Kind | Twig types that map here | Notes |
|------|--------------------------|-------|
| `Int` | `int`, `(Int lo hi)`, `(Member int …)` | All integer annotations |
| `Bool` | `bool`, `#t`, `#f` | Boolean annotation |
| `Nil` | `nil` | Nil / empty list |
| `Symbol` | `Symbol`, `'foo` | Quoted symbol atoms |
| `Str` | `String` | String heap objects (LANG47) |
| `List` | `(List T)`, bare `List` | Homogeneous list |
| `Record(name)` | A declared `RecordDef` | Named product type |
| `Union(name)` | A declared `UnionDef` | Named sum type |
| `Function { arity }` | `(lambda …)`, `(define (f …) …)` | Callable with known arity |
| `Any` | Unannotated, `any`, unresolved | Top / escape hatch |

`Any` is the widened fallback — it propagates through operations when the
checker cannot determine a more specific kind.  In `Strict` mode, `Any` at
a checked boundary is an error; in `Lenient` mode it is a warning.

---

## Typed Modes

| Module declaration | `TypedMode` variant | Checker behaviour |
|-------------------|---------------------|-------------------|
| No `(module …)` | `None` (→ `Off`) | Skip entirely |
| `(typed off)` | `Off` | Skip entirely |
| `(typed lenient)` | `Lenient` | Run checker; errors are warnings; `ok: true` |
| `(typed strict)` | `Strict` | Run checker; any error → `ok: false` |

The compiler integration hook in `twig-ir-compiler` reads `ok` and either
passes through (lenient), or returns a `TwigCompileError` wrapping the
first type error (strict).

---

## Two-Pass Algorithm

### Pass 1 — Declaration Collection

Walk `program.forms` once, populating the `TypeEnv`:

1. `Form::TypeAlias(t)` → register `t.name → t.expr` in `env.aliases`.
2. `Form::RecordDef(r)` → register `r.name → [field_names]` in `env.records`;
   also register `r.name → TwigKind::Record(r.name)` in `env.globals`.
3. `Form::UnionDef(u)` → register `u.name → [variant_names]` in `env.unions`;
   also register each `variant.name → TwigKind::Function { arity: variant.fields.len() }` in `env.globals`.
4. `Form::Define(d)`:
   - If `d.expr` is `Expr::Lambda(lam)` →
     `env.globals[d.name] = TwigKind::Function { arity: lam.params.len() }`.
   - If `d.type_annotation` is `Some(ann)` →
     `env.globals[d.name] = type_annotation_to_kind(ann, env)`.
   - Otherwise → `env.globals[d.name] = TwigKind::Any`.

Pass 1 ensures mutual recursion and forward references are handled: all
top-level names are in scope before any body is type-checked.

### Pass 2 — Expression Walking

For each `Form::Define(d)` and `Form::Expr(e)`, call `infer_expr` with an
empty `ScopeStack`.  `infer_expr` returns the inferred `TwigKind` and
may append to `errors`.

| Expr variant | Inferred kind | Side effects |
|-------------|---------------|-------------|
| `IntLit` | `Int` | — |
| `BoolLit` | `Bool` | — |
| `NilLit` | `Nil` | — |
| `SymLit` | `Symbol` | — |
| `VarRef(v)` | Scope lookup → globals lookup → `Any` | Error if unresolved (strict) |
| `Lambda { params, … }` | `Function { arity: params.len() }` | Params bound in child frame |
| `Apply { fn_expr, args }` | `Any` | Arity checked if fn resolves |
| `If { cond, then, else }` | `then_kind` if `then_kind == else_kind` else `Any` | — |
| `Let { bindings, body }` | Last body expr kind | Bindings in child frame (Scheme `let`) |
| `Begin { exprs }` | Last expr kind | — |
| `Match { scrutinee, arms }` | `Any` | Exhaustiveness checked for unions |

### Match Exhaustiveness

When `infer_expr` visits `Expr::Match(m)`:
1. Infer the scrutinee kind.
2. If kind is `TwigKind::Union(union_name)`, look up `env.unions[union_name]`
   for the complete variant list.
3. Walk arms:
   - `MatchPat::Wildcard | MatchPat::Binding(_)` → exhaustive; stop.
   - `MatchPat::Variant { name, bindings }` → mark variant covered; bind
     `bindings[i]` to `TwigKind::Any` in arm's child scope (field types not
     yet threaded through).
4. If scrutinee kind is `Union(name)` and no wildcard/binding arm is present
   and some variants are uncovered → emit one diagnostic listing the missing
   names.

---

## API

### `twig-type-checker` crate

```rust
/// Parse and type-check a Twig source string in one call.
pub fn type_check(source: &str)
    -> Result<TypeCheckResult<TypedProgram>, TwigTypeCheckError>

/// Type-check an already-parsed Program.
/// `mode_override` overrides the (typed …) directive.
pub fn check_program(program: &Program, mode_override: Option<TypedMode>)
    -> TypeCheckResult<TypedProgram>
```

`TypedProgram` is the original `Program` plus the populated `TypeEnv`.

### Error type

```rust
pub enum TwigTypeCheckError {
    Parse(TwigParseError),
}
```

Parse failures are `Err(TwigTypeCheckError)`.  Type errors live inside
`TypeCheckResult::errors: Vec<TypeErrorDiagnostic>` (from `type-checker-protocol`).

---

## Integration with `twig-ir-compiler`

`twig_ir_compiler::compile_program` gains a pre-pass:

```rust
// If the module declares (typed lenient|strict), run TW05-B.
if let Some(mode) = program.module_info.as_ref().and_then(|m| m.typed_mode.as_ref()) {
    let result = twig_type_checker::check_program(program, None);
    match mode {
        TypedMode::Strict if !result.ok => return Err(first_error_as_compile_err(&result)),
        TypedMode::Lenient => { /* warnings to stderr */ }
        _ => {}
    }
}
```

`TypedMode::Off` and programs without `module_info` are untouched — zero overhead.

---

## Acceptance Criteria

- [x] `TwigKind` covers: Int, Bool, Nil, Symbol, Str, List, Record, Union, Function, Any.
- [x] Call arity is checked when the function name resolves to `Function { arity }`.
- [x] Match exhaustiveness is checked for `Union(name)` scrutinees.
- [x] Module imports/exports are validated (exports must be declared in `env.globals`).
- [x] Type errors carry source spans (line, column).
- [x] `TypedMode::Off` / no module info → `ok: true`, zero errors, zero overhead.
- [x] `TypedMode::Lenient` → errors are warnings, `ok: true`.
- [x] `TypedMode::Strict` → any error → `ok: false`.
- [x] `TypedProgram.env` exposes `globals`, `records`, `unions`, `aliases` for TW05-C.
- [x] ≥ 28 unit tests (28 committed).

---

## Follow-ups (TW05-C)

- Invoke `lang-refinement-checker` on `RangeInt` / `MembershipInt` obligations.
- Thread record/union field types through to `MatchPat::Variant` bindings.
- Resolve dependent-type `Opaque` expressions instead of widening to `Any`.
- Cross-module import type checking.
