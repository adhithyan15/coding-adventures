# Changelog — twig-ir-compiler

## [0.16.0] — 2026-05-22 (Path A — typed binary arithmetic + comparison)

### Added — Typed CIR mnemonics for binary arithmetic / comparison

Increment 2 of the Twig → IIR-to-* end-to-end story.  Builds on
0.15.0's typed-literals work to lower **binary arithmetic** (`+ - * /`)
and **comparisons** (`= < > <= >=`) on i64 arguments to typed CIR
mnemonics (`add`, `sub`, `mul`, `div`, `cmp_eq`, `cmp_lt`, `cmp_gt`,
`cmp_le`, `cmp_ge`) instead of the legacy `call_builtin "<op>"`
dispatch.

Mirrors the same pattern PR #3903 used for Nib
(`compile_binary_chain` → typed CIR mnemonics).

#### What changed

- New `typed_arith_op_for(name) -> Option<&'static str>` table maps
  Twig builtin names to the typed-CIR mnemonic.  9 entries:
  `+ - * /` → `add sub mul div`; `= < > <= >=` → `cmp_eq cmp_lt
  cmp_gt cmp_le cmp_ge`.
- `compile_apply`'s `is_builtin` branch now:
  1. Resolves all argument expressions first (existing behaviour).
  2. For binary forms (n=2) where both args have statically-known
     `i64` type, emits the typed mnemonic with `type_hint = "i64"`
     (arithmetic) or `"bool"` (comparison), records the dest's type,
     and short-circuits the legacy `call_builtin` path.
  3. Otherwise falls back to the existing `call_builtin "<op>"`
     dispatch.
- Result types:
  - `add` / `sub` / `mul` / `div` over `i64` → `i64`.  Recorded so a
    chained expression like `(+ (* 2 3) 4)` flows through the typed
    path for the outer `+` too (the `*` dest is `i64`).
  - `cmp_*` → `bool`.

#### What this unlocks

| Program             | wasm | jvm | clr | beam |
|---------------------|------|-----|-----|------|
| `(+ 1 2)`           | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(< 1 2)`           | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(+ (* 2 3) 4)`     | ✅ (typed chain) | ✅ | ✅ | ✅ |
| `(+ (car (cons 1 2)) 3)` | ❌ still rejected (left arg is `any`) | ❌ | ❌ | ❌ |

Variadic forms (`(+ a b c)`, n>2) and arithmetic over dynamically-typed
sources (results of `car` / `length` / user-defined functions) still
flow through `call_builtin`.  Subsequent increments will lower
variadic folds and inject runtime type guards.

#### Tests

- Existing `builtin_call_uses_call_builtin_directly` test renamed
  → `builtin_call_uses_typed_add_for_i64_args` and updated to assert
  the typed path.
- New `builtin_call_falls_back_to_call_builtin_for_dynamic_args`
  asserts the fallback path still fires when an arg is `any`.
- `builtins_recognised` narrowed to non-typed builtins (cons / car /
  cdr / predicates / print) — typed arithmetic moved to its own
  dedicated test.
- 2 new e2e tests in `tests/backend_compat.rs`:
  - `twig_typed_arithmetic_accepted_by_every_backend` — `(+ 1 2)`
  - `twig_typed_comparison_accepted_by_every_backend` — `(< 1 2)`
- The "still rejected" boundary marker from increment 1 has flipped
  to `twig_arithmetic_over_dynamic_args_still_rejected`, pinning the
  current boundary one step further along.
- 73 lib + 5 backend e2e tests pass.

## [0.15.0] — 2026-05-22 (Path A — typed literals + typed return)

### Added — Local type inference for integer / boolean literals

Increment 1 of the Twig → IIR-to-* end-to-end story (the LANG VM
"any frontend, any backend" promise).  A probe against
`iir-to-{wasm,jvm,clr,beam}` validators on the simplest possible Twig
program (`42`) showed every backend rejected it — every instruction
carried `type_hint = "any"`, which the validators all reject with
`UntypedInstruction`.

This release narrows the gap by stamping concrete `type_hint`s on
integer / boolean literals and propagating those types through `ret`
emission sites.

#### What changed

- New `var_types: HashMap<String, String>` on `FnCtx`, populated only
  at sites where the type is statically obvious — literal-defining
  expressions (`IntLit`, `BoolLit`).  Dynamic / `call_builtin`
  destinations are intentionally not recorded; absence means
  "genuinely `any`".
- `Expr::IntLit` now emits `const Int(n)` with `type_hint = "i64"`
  (was `"any"`) and records `var_types[dest] = "i64"`.
- `Expr::BoolLit` now emits `const Bool(b)` with `type_hint = "bool"`.
- `ret` emission sites propagate the source var's inferred type via
  `FnCtx::type_of`.  Dynamic returns still emit `"any"` correctly.
- `main`'s `return_type` is now derived from the last `ret`
  instruction's `type_hint` rather than hard-coded to `"any"`.

#### What this unlocks

The simplest Twig programs flow through every IIR-to-* backend:

| Program   | wasm | jvm | clr | beam |
|-----------|------|-----|-----|------|
| `42`      | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `#t`      | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) | ✅ (was ❌) |
| `(+ 1 2)` | ❌ still rejected — `call_builtin "any"` | ❌ | ❌ | ❌ |

Arithmetic / list / closure programs still emit `call_builtin` with
`type_hint = "any"` and stay rejected.  Subsequent path-A increments
will lower `(+ 1 2)` to typed `add_i64`, then `cmp_*`, then non-trivial
control flow.

#### Tests

- 3 new e2e tests in `tests/backend_compat.rs`:
  - `twig_int_literal_accepted_by_every_backend` — `42` validates on
    all four backends; `main.return_type == "i64"`.
  - `twig_bool_literal_accepted_by_every_backend` — same for `#t`.
  - `twig_arithmetic_still_rejected_in_increment_1` — pins down the
    current boundary; a future increment that types arithmetic must
    explicitly update this test.
- The existing `every_instruction_has_any_or_void_type_hint` test
  renamed to `every_instruction_has_known_type_hint` and updated to
  accept `"i64"` / `"bool"` / `"str"` in addition to `"any"` / `"void"`.
- 72 unit tests pass (was 71).

#### Compatibility

- Non-Twig callers unaffected.
- Downstream Twig tooling (twig-aot, twig-vm) all continue to pass —
  the new type hints are *stricter* than the old `"any"`, never
  broader.
- Pre-existing twig-module-driver `tw05*` self-compile tests fail on
  this branch *and* on `main` (a Windows file-path issue in the test
  fixture, unrelated to this PR).

## [0.14.0] — 2026-05-17

### Added (LANG72 — TW05-Q cross-module strict type checking)

- **`compile_program_with_externs_and_globals`** — new public function that
  accepts a `&HashMap<String, TwigKind>` of cross-module globals in addition
  to the existing extern-fn list.  When a `(typed strict)` module is compiled
  this function forwards the globals map to `check_program_with_globals` so
  that imported names from peer modules are visible during the type-check pass.

  This fixes a pre-existing regression where `compile_program_with_externs`
  called `check_program(program, None)` internally, which caused strict modules
  that imported names from other modules to always fail type checking with
  "unresolved variable" errors — even when the imports were correct.

  Called by `twig-module-driver` Phase 4 instead of `compile_program_with_externs`;
  the driver now passes each module's accumulated export globals from Phase 3.5.

## [0.13.0] — 2026-05-15

### Added (LANG58 — TW05-E string/char builtins)

- **13 string and character builtins added to `BUILTINS`** — these operations
  have been in `lispy-runtime` since LANG47 but were missing from the
  compiler's `BUILTINS` constant, so calls from Twig source were treated as
  user-function calls and failed with "unbound name" at runtime:
  - `string-length`, `string-ref`, `substring`, `string-append`
  - `string->number`, `string=?`, `string<?`, `string>?`
  - `char->integer`, `integer->char`
  - `char-alphabetic?`, `char-numeric?`, `char-whitespace?`

  These are required by `compiler/lexer.tw` (TW05-E) for scanning source text
  character-by-character using ASCII integer comparison.

## [0.12.0] — 2026-05-15

### Added (LANG57 — TW05-D string/symbol conversion builtins)

- **`"number->string"`, `"string->symbol"`, `"symbol->string"` added to
  `BUILTINS`** — these three conversions have been in `lispy-runtime` since
  LANG47 but were accidentally omitted from the compiler's `BUILTINS`
  constant.  Without this entry the compiler treated calls like
  `(number->string 42)` as user-function calls, which then failed with
  "unbound name" when no top-level define existed.  Adding them makes
  string↔number↔symbol conversions usable from any Twig source file,
  including the new `code/twig/compiler/` data model modules.

- **`extern_fns` in `compile_module_tree` now covers record/union generated
  names** — the `twig-module-driver` Phase 3 pre-pass was extended to
  collect constructor, predicate, and accessor names from `Form::RecordDef`
  and `Form::UnionDef`.  This fixes "unbound name" errors when one Twig
  module calls record/union functions defined in another module.

---

## [0.11.0] — 2026-05-15

### Fixed (LANG57 — TW05-D prerequisite)

- **`compile_match`: `jmpif` → `jmp_if_false`** — The variant-arm lowering in
  `compile_match` previously emitted a non-standard three-operand opcode `"jmpif"`
  (`jmpif cond arm_label skip_label`).  This opcode was never registered in the VM
  dispatch table, causing every `(match …)` expression to fail at runtime with
  `UnsupportedOpcode("jmpif")`.
  
  Fixed by replacing it with the standard two-operand `jmp_if_false` pattern
  (identical to `compile_if`): when the tag comparison is false, jump to
  `skip_label`; when true, fall through to the arm body.  The now-redundant
  `label arm_label` instruction is also removed.

---

## [0.10.0] — 2026-05-14

### Added (LANG56 — Multi-File Module Driver)

- **`Compiler::with_extern_fns(&[&str]) -> Self`** — builder method that pre-registers
  extern function names in `fn_globals` before the compiler's own pre-pass runs.  Allows
  cross-module calls (`(double 21)` calling `double` from another `.tw` file) to compile
  to `call` instructions rather than failing with "unbound name".  The linker resolves the
  actual call targets.

- **`compile_program_with_externs(program, module_name, extern_fns)`** — public entry point
  for the module driver.  Equivalent to `compile_program` but pre-populates `fn_globals`
  with `extern_fns` before compiling.  Applies the same LANG49 type-check pre-pass as
  `compile_program`.

- **IIRExport population from `module_info`** — when a program carries a
  `(module name (export f1 f2 ...))` clause, the compiler now populates `IIRModule.exports`
  from `info.exports`, filtered to names that were actually compiled as top-level functions.
  Previously `exports` was always `vec![]`.

---

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
