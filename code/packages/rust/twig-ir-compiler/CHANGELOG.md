# Changelog — twig-ir-compiler

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
