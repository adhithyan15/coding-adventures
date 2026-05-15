# Changelog — twig-ir-compiler

## [0.9.0] — 2026-05-14

### Added (LANG55 — Higher-Order List Operations)

- **BUILTINS expansion** — `"map"`, `"filter"`, `"fold-left"`, `"fold-right"` added
  to the `BUILTINS` constant.  The compiler now emits `call_builtin "map" fn_reg list_reg`
  (etc.) for these names rather than treating them as user-defined functions.
  The actual execution is handled by the new special-cased `exec_hof_*` handlers in
  `twig-vm` which can recurse into `dispatch` to call the supplied closure.

---

## [0.8.0] — 2026-05-14

### Added (LANG52 — stdlib completeness + LANG51 string literals)

#### LANG51: string literal lowering

- **`Expr::StrLit` → `const(Operand::Str(value)) : "str"`** — string literals compile to a
  `const` instruction with `Operand::Str` payload and `type_hint = "str"`.  The VM's
  `exec_const` handler (introduced in LANG47) materialises this as a `LangString` heap object.
- `Expr::StrLit` added to the leaf-atom arm of `free_vars.rs` (never a free variable).

#### LANG52: `let*` sequential bindings

- **`Expr::LetStar` → `compile_let_star`** — sequential bindings: each RHS is compiled in a
  scope extended by all prior names.  Each binding gets a fresh register allocated via
  `_move`; the body is compiled after all bindings are live.
- **`free_vars.rs` Expr::LetStar walk** — incremental bound-set extension mirrors the
  compiler's sequential scoping exactly (each name bound before the next RHS).

#### LANG52: `and` / `or` special forms (short-circuit)

- **`(and e₁ e₂ …)`** — intercepted in `compile_apply` before the builtin-resolution path.
  Lowered to: evaluate `e₁`, branch on `jmp_if_false`, evaluate tail with
  recursive `compile_and`, merge into shared result register via `_move`.
  `(and)` → `#t`; `(and e)` → `e`.
- **`(or e₁ e₂ …)`** — similar pattern.  `(or)` → `#f`; `(or e)` → `e`.
- Neither `and` nor `or` is in the `BUILTINS` constant — they never reach the
  `resolve_builtin` path.

#### LANG52: expanded BUILTINS constant

Added to the `BUILTINS: &[&str]` array (used for higher-order closure wrapping):
`<=`, `>=`, `modulo`, `remainder`, `quotient`, `not`, `boolean?`, `equal?`,
`list`, `list?`, `length`, `append`, `reverse`, `list-ref`, `assoc`,
`symbol-append`, `host/write_string`, `host/read_line`, `host/read_file`.

## [0.7.0] — 2026-05-14 (LANG51 — string literal lowering, included here)

*Note: 0.7.0 was the planned standalone LANG51 release; changes are rolled into 0.8.0
above since LANG52 depends on LANG51 and both land together.*

## [0.6.0] — 2026-05-14

### Added (LANG50 — Annotation-aware IIR emission)

- `compile_typed_source(source, module_name) -> Result<IIRModule, TwigCompileError>`
  — new compilation entry point that runs the LANG50 grammar-type-checker pass
  first and post-processes the resulting IIR to propagate concrete `type_hint`
  values (`"i64"`, `"bool"`, `"str"`, `"closure"`) on instructions whose source
  positions map to concretely-typed `AnnotatedNode`s.
- `build_hint_map` — traverses the `AnnotatedNode` tree to build a
  `HashMap<(line, col), &'static str>` of concrete hints.
- `apply_hints` — post-processes an `IIRFunction`'s instructions using the
  hint map and the function's `source_map` for position correlation.
- `set_function_type_status` — sets `IIRFunction::type_status` to
  `FullyTyped` / `PartiallyTyped` / `Untyped` based on the fraction of
  non-void instructions carrying a concrete type hint.
- 7 new unit tests in `tests` module:
  `typed_source_int_literal_hint`, `typed_source_bool_literal_hint`,
  `typed_source_nil_literal_hint`, `typed_source_untyped_fallback`,
  `typed_source_function_status_fully_typed`,
  `typed_source_strict_mode_type_error_returns_err`,
  `typed_source_off_mode_no_errors`.

### Dependencies added

- `type-declarations = { path = "../type-declarations" }`

### Backward compatibility

- `compile_source` and `compile_program` are **unchanged**.
- `FunctionTypeStatus` set by `set_function_type_status` only affects
  functions compiled via `compile_typed_source`; the existing path still
  emits `Untyped` everywhere.

---

## [0.5.0] — 2026-05-14

### Added (LANG49 — TW05-B type-check pre-pass)

Wires the new `twig-type-checker` crate as an optional pre-pass in
`compile_program`.

#### Behaviour

- `TypedMode::Strict`: if `check_program` returns `ok: false`, the first
  `TypeErrorDiagnostic` is wrapped in a `TwigCompileError` and returned
  as `Err` before any IIR is emitted.
- `TypedMode::Lenient`: type errors are printed as warnings to `stderr`
  (prefix `twig type warning (line:col): …`), then compilation proceeds.
- `TypedMode::Off` / no `module_info`: pre-pass skipped entirely —
  zero performance overhead for dynamic Twig programs.

#### Dependency added

- `twig-type-checker = { path = "../twig-type-checker" }` — the new
  TW05-B base type checker crate.

---

## [0.4.0] — 2026-05-14

### Added (LANG48 — TW05-A annotation erasure)

Implements the TW05-A bootstrap stage: typed Twig source compiles to
dynamic IIR by erasing all type annotations.  No type checker yet (that's
TW05-B/C); the compiler accepts typed programs and lowers them faithfully.

#### New `Compiler` field

- `variant_tags: HashMap<String, usize>` — populated during the pre-pass
  from every `Form::UnionDef`; consulted when lowering `Expr::Match` arms
  to determine variant integer tags for dispatch.

#### New form lowering

- `Form::TypeAlias` — erased (no-op, type aliases are compile-time only).
- `Form::RecordDef` — lowered via `emit_record_def`:
  - Constructor function `Name(f0, f1, …)` using a right-fold `cons` chain.
  - Positional accessor `name-field-i(r)` using `car` of `cdr^i`.
  - Type predicate `name?(v)` using `pair?`.
- `Form::UnionDef` — lowered via `emit_union_def`:
  - Per-variant constructor `Variant(f0, …)` — prepends the zero-based
    integer tag via `cons`.
  - Per-variant predicate `Variant?(v)` — checks `(= (car v) tag)`.
  - Per-variant field accessor `variant-field-k(v)` using `car` of
    `cdr^(k+1)` (skip the tag slot).

#### New expression lowering

- `Expr::Match` — lowered via `compile_match` to a `jmpif`/`label`/`jmp`
  chain:
  - Scrutinee evaluated once into a fresh register.
  - `Variant` arm: test `(= (car scrutinee) tag)`, bind fields via
    `car`/`cdr` chains, evaluate body.
  - `Binding` arm: bind scrutinee to name, evaluate body.
  - `Wildcard` arm: evaluate body directly.
  - After all arms: fall through to `nil`.

#### Annotation erasure extension

- `TypeAnnotation::Opaque(_)` → `TypeAnnotation::Any` in the annotation
  map.  Any type expression that isn't a LANG23 shape is silently erased
  to the `Any` (untyped) refinement, preserving backward compat.

### Tests

- Regression tests confirm `alloc_closure` / `call_closure` emission is
  unchanged by LANG48 changes.
- New compiler tests for record def erasure (constructor + accessor +
  predicate IIR shapes), union def erasure (tagged variants), and match
  expression lowering (variant/binding/wildcard dispatch chains).

---

## [0.3.0] — 2026-05-12

### Changed (LANG34 — Emit alloc_closure / call_closure)

Three emission sites updated to use the LANG34 first-class closure opcodes:

#### Lambda allocation (`compile_anonymous_lambda`)

```
BEFORE:
  %s0 = const("__lambda_N")          ← string_arg indirection
  %c0 = call_builtin("make_closure", %s0, caps...) : "any"

AFTER:
  %c0 = alloc_closure(Str("__lambda_N"), caps...) : "closure"
```

No preceding `const` instruction is emitted; `fn_name` is now an inline
`Operand::Str` in `srcs[0]`.

#### Top-level function as value (`compile_var_ref` / fn_globals)

```
BEFORE:
  %s0 = const("fn_name")
  %fnref = call_builtin("make_closure", %s0) : "any"

AFTER:
  %fnref = alloc_closure(Str("fn_name")) : "closure"
```

#### Indirect call (`compile_apply`, indirect path)

```
BEFORE:
  %r = call_builtin("apply_closure", %handle, args...) : "any"

AFTER:
  %r = call_closure(%handle, args...) : "any"
```

The `string_arg` helper is retained for `global_set`/`global_get`/`make_symbol`
which still use the const-via-Var register convention.

#### Tests updated

Three tests renamed/updated to assert the new opcode forms:
- `anonymous_lambda_emits_make_closure` → `anonymous_lambda_emits_alloc_closure`
- `closure_call_uses_apply_closure` → `closure_call_uses_call_closure`
- `fn_globals_can_be_passed_as_values` (assertion updated)

---

## [0.2.1] — 2026-05-11

### Fixed (LANG33 — Module System)

- Added `exports: Vec::new(), imports: Vec::new()` to the `IIRModule { ... }`
  struct literal in `compiler.rs` (`compile_module`).  Required by the new
  LANG33 fields on `IIRModule`; the workspace `cargo build` enforces this.

---

## [0.2.0] — 2026-05-04

### Added (LANG23 PR 23-E — emit RefinedType annotations into IIR)

- `type_annotation_to_refined_type(ann: &TypeAnnotation) -> RefinedType`:
  conversion function that bridges the parser's `TypeAnnotation` enum to
  `lang-refined-types::RefinedType`.  Matches all five `TypeAnnotation` variants:
  - `UnrefinedInt` → `RefinedType::unrefined(Kind::Int)`
  - `UnrefinedBool` → `RefinedType::unrefined(Kind::Bool)`
  - `Any` → `RefinedType::unrefined(Kind::Any)`
  - `RangeInt { lo, hi }` → `RefinedType::refined(Kind::Int, Predicate::Range { lo, hi, inclusive_hi: false })`
  - `MembershipInt { values }` → `RefinedType::refined(Kind::Int, Predicate::Membership { values })`
- `compile_top_level_lambda` now populates `IIRFunction::param_refinements` and
  `IIRFunction::return_refinement` from the `Lambda` node's annotation fields.
- `lang-refined-types` added as a dependency.
- Round-trip tests in `lib.rs` (PR 23-E section, 7 new tests):
  - `ranged_int_param_annotation_round_trips_to_iir`
  - `unrefined_int_param_annotation_round_trips`
  - `return_annotation_round_trips_to_iir`
  - `multiple_annotated_params_lockstep`
  - `unannotated_function_has_no_refinement_fields`
  - `annotation_does_not_change_existing_type_hints`
  - `source_map_lockstep_holds_for_annotated_functions`

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig → InterpreterIR compiler
  (TW00).  Mirrors the Python reference at
  `code/packages/python/twig/src/twig/compiler.py`.
- `compile_source(source, module_name)` — lex + parse + compile in one
  call.
- `compile_program(program, module_name)` — compile a parsed
  `twig_parser::Program` into an `IIRModule`.
- `Compiler` struct — one-program, mutable lowering driver.
- Pre-pass classification of top-level defines into
  `fn_globals` (lambda RHS) and `value_globals` (non-lambda RHS) so
  the main pass can resolve names before walking any bodies.
- Per-function compilation context (`FnCtx`) tracking accumulated
  instructions, in-scope locals, and fresh-name counters for
  registers and labels.
- Free-variable analysis (`free_vars` module) — Scheme-`let`-aware
  walk that returns captures in stable insertion order.
- Apply-site dispatch decided at compile time:
  - top-level user function → `call <name>, ...args`
  - builtin → `call_builtin <name>, ...args`
  - everything else → `call_builtin "apply_closure", h, ...args`
- Lambda handling: each anonymous lambda becomes a synthesised
  top-level `IIRFunction` named `__lambda_N`; captured variables
  appear as the *leading* parameters in the order produced by
  `free_vars`; the call site emits `call_builtin "make_closure"
  <fn_name> <captures...>`.
- `if` lowering to `jmp_if_false` + two-branch `_move`s + final
  `label`s — preserves value type across branches (booleans are not
  coerced to integers).
- `let` lowering with mutually-independent bindings, copied into
  named registers via `_move`.
- `begin` returns the value of the last expression.
- Top-level value defines lower to `call_builtin "global_set" name
  value`; references to value globals lower to
  `call_builtin "global_get" name`.
- Top-level function names in non-call position wrap in a 0-capture
  `make_closure`; builtin names wrap in `make_builtin_closure`.
- Synthesised `main` function holds top-level value defines and bare
  expressions in source order.  Programs with no bare expression
  return `nil` via `call_builtin "make_nil"`.
- Every emitted instruction carries `type_hint = "any"` (or
  `"void"` for control-flow ops); functions are tagged
  `FunctionTypeStatus::Untyped`.
- `TwigCompileError { message, line, column }` with
  `From<TwigParseError>` so callers handle a single error type at
  the public entry point.
- `MAX_COMPILE_DEPTH = 256` cap in `compile_expr` — defence-in-depth
  against stack overflow on hand-built ASTs (the parser already
  caps source-paren-depth at 64 before reaching the compiler).
- 45 unit tests verifying instruction shape, dispatch decisions,
  closure layout, recursion, and error paths.
