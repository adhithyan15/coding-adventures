# Changelog — twig-ir-compiler

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
- 30+ unit tests verifying instruction shape, dispatch decisions,
  closure layout, recursion, and error paths.
