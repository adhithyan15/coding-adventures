# LANG50 — Generic Grammar Type Checker (Compilation-First)

## Motivation

LANG49 introduced `twig-type-checker`, a Twig-specific type checker that
produces `ok/errors` only — types as ornament.  The stronger requirement is:
**types must feed AOT and JIT compilation**.  When the type checker infers
that an expression produces an `Int`, that knowledge must propagate into
`type_hint = "i64"` on every IIR instruction that computes it.  The existing
JIT/AOT specialisers in `jit-core` and `aot-core` already prioritise
`type_hint` over runtime profiles:

```rust
// jit-core/src/specialise.rs
fn spec_type(instr: &IIRInstr, min_obs: u32) -> String {
    if instr.type_hint != "any" && ALLOWED_TYPES.contains(&instr.type_hint.as_str()) {
        return instr.type_hint.clone();   // ← static hint wins, zero profiling cost
    }
    ...
}
```

A fully-typed IIR module therefore reaches AOT-quality code generation without
any runtime warmup.

LANG50 generalises the type checker away from Twig-specific AST types
(`Program`, `Expr`, `Form`) so that any language compiled through the grammar
pipeline can participate:  the **parser emits `TypeDeclarations`** (analogous
to TypeScript's `.d.ts`) and the **generic checker operates on the raw
`GrammarASTNode`** tree.

---

## New Crates

### `type-declarations` (pure data, no dependencies)

Defines the language-agnostic type declaration format.

```
TypeDeclarations
  language: String              — e.g. "twig", "ruby"
  named_types: {name → NamedTypeDecl}   — records, unions, aliases
  globals: {name → KindDecl}   — top-level bindings
  typed_mode: Option<TypedModeDecl>

KindDecl:
  Int | Bool | Nil | Symbol | Str | List
  Named(String)                — look up in named_types
  Function { arity: usize }
  Any

KindDecl::to_iir_hint() → &'static str
  Int → "i64",  Bool → "bool",  Str → "str",  Function{..} → "closure",  _ → "any"

AnnotatedNode                  — GrammarASTNode + inferred KindDecl at every node
  rule_name, kind, children, start_line, start_column, end_line, end_column
  iir_hint() → kind.to_iir_hint()
```

`AnnotatedNode` is the central compilation artifact that flows from type
checker into IIR emission.

### `grammar-type-checker` (depends on `parser` + `type-declarations`)

Generic type checker.  Core API:

```rust
pub fn check<P: LanguageProfile>(
    root:    &GrammarASTNode,
    decls:   &TypeDeclarations,
    profile: &P,
) -> TypeCheckResult<AnnotatedNode>
```

Annotation is **always-on** — even in `TypedModeDecl::Off` the annotated tree
is built (kinds are `Any` for unresolved nodes).  Type *enforcement*
(unresolved-variable errors, arity errors, exhaustiveness errors) is
mode-gated.

#### `LanguageProfile` trait

Encodes language-specific tree navigation without coupling the checker to any
language's AST types.  Each method takes a raw `GrammarASTNode` and returns
structured info or `None`:

```rust
pub trait LanguageProfile: Send + Sync {
    fn literal_kind(&self, node: &GrammarASTNode) -> Option<KindDecl>;
    fn as_var_ref<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a str>;
    fn as_apply<'a>(&self, node: &'a GrammarASTNode) -> Option<AppInfo<'a>>;
    fn as_binder<'a>(&self, node: &'a GrammarASTNode) -> Option<BinderInfo<'a>>;
    fn as_match<'a>(&self, node: &'a GrammarASTNode) -> Option<MatchInfo<'a>>;
    fn as_begin<'a>(&self, node: &'a GrammarASTNode) -> Option<Vec<&'a GrammarASTNode>>;
    fn child_exprs<'a>(&self, node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode>;
    fn position(&self, node: &GrammarASTNode) -> (usize, usize);
}
```

`BinderInfo` distinguishes let-style from lambda-style bindings:

```rust
pub enum BinderKind<'a> {
    Let    { bindings: Vec<(String, &'a GrammarASTNode)>, body: Vec<&'a GrammarASTNode> },
    Lambda { params: Vec<(String, KindDecl)>,             body: Vec<&'a GrammarASTNode> },
}
```

#### Inference algorithm

1. **Literal?** → `literal_kind()` → return annotated leaf
2. **Variable reference?** → `as_var_ref()` → scope lookup → `globals` lookup → `Any` + error if absent
3. **Function application?** → `as_apply()` → infer callee + args → arity check if callee is `Function{n}`
4. **Let/lambda binder?** → `as_binder()` → push scope, bind names, infer body, pop
5. **Match?** → `as_match()` → infer scrutinee, exhaustiveness check if union, walk arms
6. **Begin?** → `as_begin()` → infer all, return last kind
7. **Fallback** → `child_exprs()` → recurse, return last kind

Depth cap: 256.

---

## Updated Crates

### `twig-parser` (new export)

```rust
pub fn emit_type_declarations(program: &Program) -> TypeDeclarations
```

Converts `Form::TypeAlias/RecordDef/UnionDef/Define` → `TypeDeclarations`.
Maps `TypeAnnotation`/`TypeExpr` → `KindDecl` (same mapping as the existing
`type_annotation_to_kind` in `twig-type-checker`).

New dependency: `type-declarations`.

### `twig-type-checker` (v0.2.0 — thin adapter)

Adds `TwigLanguageProfile` implementing `LanguageProfile` for Twig grammar
rule names (`"atom"`, `"quoted"`, `"apply"`, `"lambda_form"`, `"let_form"`,
`"begin_form"`, `"if_form"`, `"match_form"`, etc.).

Adds `type_check_source` entry point using the GrammarASTNode path.
Keeps `check_program` (legacy typed-AST path) for backward compat with
`twig-ir-compiler`.

### `twig-ir-compiler` (v0.6.0)

Adds `compile_typed_source`:

```rust
pub fn compile_typed_source(source: &str) -> Result<IIRModule, TwigCompileError>
```

1. Calls `twig_type_checker::type_check_source(source)` → `AnnotatedNode` tree
2. On strict-mode failure → `Err`
3. Compiles to IIR using `annotated_node.iir_hint()` for `type_hint` on each instruction
4. Sets `FunctionTypeStatus` from instruction type coverage

Existing `compile_source` / `compile_program` unchanged.

---

## KindDecl → IIR type_hint

| KindDecl | IIR type_hint | JIT/AOT impact |
|----------|---------------|----------------|
| `Int` | `"i64"` | Native 64-bit int ops, no box |
| `Bool` | `"bool"` | Conditional branches skip type guard |
| `Str` | `"str"` | String operations skip type guard |
| `Function{n}` | `"closure"` | Direct closure ops, no apply_closure dispatch |
| All others | `"any"` | Falls back to profiling / runtime check |

---

## Twig Grammar → LanguageProfile Mapping

| Grammar rule_name | Profile method | Action |
|-------------------|----------------|--------|
| `atom` + INTEGER token | `literal_kind` | `KindDecl::Int` |
| `atom` + BOOL_TRUE/FALSE | `literal_kind` | `KindDecl::Bool` |
| `atom` + "nil" keyword | `literal_kind` | `KindDecl::Nil` |
| `quoted` | `literal_kind` | `KindDecl::Symbol` |
| `atom` + NAME token | `as_var_ref` | scope/globals lookup |
| `apply` | `as_apply` | callee + args extracted from `expr` children |
| `lambda_form` | `as_binder` | `BinderKind::Lambda{params, body}` |
| `let_form` | `as_binder` | `BinderKind::Let{bindings, body}` |
| `define` (fn) | `as_binder` | `BinderKind::Lambda{params, body, is_global}` |
| `match_form` | `as_match` | scrutinee + arms |
| `begin_form` | `as_begin` | all `expr` children |
| `if_form` | `child_exprs` | cond + then + else nodes |
| `expr`/`compound`/`form` | `child_exprs` | transparent wrapper — single child |

---

## Tests

### `type-declarations` (≥ 8)
- `kind_to_iir_hint_int` / `_bool` / `_str` / `_closure` / `_any`
- `annotated_node_iir_hint`
- `resolve_alias_chain`, `resolve_alias_cycle_returns_any`
- `union_variants_lookup`

### `grammar-type-checker` (≥ 18)
- `annotated_node_carries_kind`
- `annotated_children_propagate`
- `typed_off_still_annotates`
- `generic_var_ref_resolved` / `_unresolved`
- `generic_apply_arity_correct` / `_wrong`
- `generic_match_exhaustive_all` / `_wildcard` / `_non_exhaustive`
- `generic_typed_off` / `_strict_fails` / `_lenient_ok`
- `kind_decl_resolve_alias` / `_cycle`
- `type_declarations_union_variants`

### `twig-type-checker` adapter (existing 34 + 5 new)
- `emit_type_decls_record` / `_union` / `_alias` / `_globals`
- `type_check_source_path_uses_grammar_ast`

### `twig-ir-compiler` (existing + 4 new)
- `typed_source_int_literal_hint`
- `typed_source_bool_literal_hint`
- `typed_source_untyped_fallback`
- `typed_source_fn_fully_typed`

---

## Acceptance Criteria

1. `cargo test -p type-declarations` — all pass
2. `cargo test -p grammar-type-checker` — all pass
3. `cargo test -p twig-type-checker` — all 34 existing tests pass + 5 new
4. `cargo test -p twig-ir-compiler` — all existing tests pass + 4 new
5. `cargo build --workspace` — clean
6. `(define (f (x : int)) (+ x 1))` compiled via `compile_typed_source` emits `type_hint = "i64"` on the result of `+`
7. A fully-typed function has `FunctionTypeStatus::FullyTyped` in IIR

## Follow-up

- **TW05-C / LANG51**: Refinement solver — `RangeInt { lo, hi }` annotations
  propagate into IIR as `"i64"` + refinement proof obligations in
  `iir-refinement-pass`.
- Unify `check_program` (typed-AST path) and `type_check_source` (GrammarASTNode
  path) now that `twig-ir-compiler` can be refactored to retain the raw AST.
- Extend `TwigLanguageProfile` to `define` (value form) body walking and
  typed parameter kind propagation.
