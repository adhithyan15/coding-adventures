# Changelog — twig-vm

## [0.2.0] — 2026-04-29

### Added — PR 4 of LANG20: real dispatch loop

- **`dispatch` module** — first version of the VM that actually
  *runs* a Twig program end to end.  Replaces the PR-3
  `evaluate_call_builtin` 1-instruction helper with a tree-walking
  dispatcher covering the IIR opcodes emitted by
  `twig-ir-compiler` for the closure-free / globals-free /
  symbols-free subset:
  - `const` — bind register ← `Int` / `Bool` immediate
  - `call_builtin` — resolve through `LispyBinding`, materialise
    args via `operand_to_value`, dispatch
  - `call` — resolve callee in the module's `functions` table,
    recurse into a fresh `Frame`
  - `jmp` / `jmp_if_false` / `label` — control flow with O(1)
    label resolution (label index built once per function on
    entry)
  - `ret` — return the operand value to the caller
- **`TwigVM::run(source)`** — public end-to-end entry point.
  Compiles + dispatches in one shot, returns a `LispyValue`.
- **`TwigRunError`** — combined error type that wraps
  `TwigCompileError` and `RunError` so callers don't need to
  juggle two error families.
- **Resource limits**:
  - `MAX_DISPATCH_DEPTH = 256` — caps recursion depth so
    adversarial input can't blow the host stack.
  - `MAX_INSTRUCTIONS_PER_RUN = 2²⁰` — caps total instructions
    per top-level run as a backstop against infinite loops in
    hand-built malformed IIR.
  - `MAX_REGISTERS_PER_FRAME = 2¹⁶` — caps the up-front
    `HashMap` allocation per `Frame` so a hand-built module
    with `register_count = usize::MAX` cannot abort the
    process at allocation time before any instruction tick
    fires.  Added in response to a security review finding.
- **`_move` and `make_nil` builtins** added to `lispy-runtime`
  (and registered in `LispyBinding::resolve_builtin`).  These are
  infrastructure builtins emitted by `twig-ir-compiler` for
  type-preserving register copies (`if` / `let` lowering) and
  nil materialisation.  They are not part of the Lispy surface
  language but appear in the lowered IIR.
- **End-to-end tests** that prove canonical Twig programs run
  through the full pipeline:
  - arithmetic: `(+ 2 3)` → 5, `(+ (* 2 3) (- 10 4))` → 12
  - control flow: `(if (< 1 2) 100 200)` → 100, with bool literals
  - locals: `(let ((x 5)) (* x x))` → 25, nested `let`
  - sequencing: `(begin 1 2 3)` → 3
  - direct call: `(define (square x) (* x x)) (square 7)` → 49
  - direct call w/ multiple args: `(add3 1 2 3)` → 6
  - recursion: `(fact 5)` → 120, `(fib 10)` → 55
  - mutual recursion: `(is_even 10)` → `#t`
  - cons family: `(car (cons 1 2))` → 1, `(pair? (cons 1 2))` → `#t`
  - Scheme truthiness: `(if 0 1 2)` → 1 (only `#f` and `nil`
    branch — 0 is truthy)

### Removed

- **`evaluate.rs` module** (`evaluate_call_builtin`,
  `EvaluateError`) — the PR-3 1-instruction placeholder is no
  longer needed now that `dispatch` interprets full programs.
  All its tests are subsumed by `dispatch::tests` and
  `tests::run_*`.

### Changed

- **`lispy-runtime::LispyBinding::resolve_builtin`** — extended
  with `_move` and `make_nil` arms.  No source-language users —
  the IR compiler emits these names from `if` / `let` lowering.
- **`lang-runtime-safety` CI workflow** — extended Miri
  coverage to include `twig-vm` so the dispatcher's integration
  with lispy-runtime's tagged-pointer code is exercised under
  Miri on every PR (lispy-runtime's own Miri suite tests the
  binding in isolation; this catches UB in the seam).

### Tests across the diff

- twig-vm: **62 unit + 2 doc tests** — all pass on stable Rust
  and Miri.  35 PR-3 tests retired (subsumed by dispatch tests).
  Two extra tests added for the security-review fixes
  (`frame_caps_register_count`,
  `build_label_index_rejects_duplicate_labels`).
- lispy-runtime: 105 unit tests (4 new for the `_move` /
  `make_nil` builtins).

### Hardening

- Clippy-clean on all touched crates (one pre-existing
  `unused_parens` warning in `operand.rs` cleaned up while the
  file was being edited — independent of PR 4).
- Miri-clean on twig-vm dispatcher tests including:
  - factorial recursion (frames + arg copy)
  - mutual recursion (cross-function frame creation)
  - cons / car / cdr (heap pointer round-trips)
  - infinite-recursion guard (depth cap fires before host stack
    overflow)
- Resource limits (`MAX_DISPATCH_DEPTH`, `MAX_INSTRUCTIONS_PER_RUN`,
  `MAX_REGISTERS_PER_FRAME`) unit-tested directly so a future
  change can't silently raise them.
- `build_label_index` errors on duplicate label names (instead of
  silently shadowing), so a hand-built module that violates the
  fresh-label invariant fails fast instead of mis-routing
  `jmp` instructions.  Added in response to a security review
  finding.

## [0.1.0] — 2026-04-29

### Added

- **PR 3 of [LANG20](../../../specs/LANG20-multilang-runtime.md)
  §"Migration path"** — runtime wiring between the Twig frontend and
  the LANG-runtime substrate.
- `TwigVM` facade — currently stateless; PR 4 will add per-VM
  state (frame stack, register file scratch, JIT promotion
  thresholds).  Provides:
  - `TwigVM::new()` — zero-cost constructor.
  - `TwigVM::compile(source)` — calls `twig_ir_compiler::compile_source`
    with `module_name = "twig"`.
  - `TwigVM::compile_with_name(source, name)` — explicit module name.
  - `TwigVM::resolve_builtin(name)` — proxies through
    `<LispyBinding as LangBinding>::resolve_builtin`.
- `operand::operand_to_value` — converts IIR `Operand` →
  `LispyValue`.  The per-language seam between the language-
  agnostic IIR and the Lispy runtime's tagged-i64 representation.
  Handles Int (with range-check against the 61-bit tagged-int
  range), Bool, Var (via caller-supplied frame_lookup callback),
  and the special-cased `nil` name.  Float operands return a
  `RuntimeError::TypeError("flonum")` since Lispy doesn't yet
  have flonums.
- `evaluate::evaluate_call_builtin` — 1-instruction evaluator
  proving the substrate composes.  Takes a `call_builtin` IIR
  instruction, resolves the builtin via `LispyBinding`,
  materialises argument operands into `LispyValue`s, and
  dispatches.  Real interpretation lands in PR 4 (vm-core
  wiring); this evaluator is the integration test surface.
  - `EvaluateError` enum with variants for unsupported opcode,
    missing/non-Var builtin name, unknown builtin, operand
    conversion failure, builtin runtime error.
- 35 unit + 2 doc tests.  Coverage includes the full Twig source
  → IIR → LispyValue evaluation pipeline for arithmetic, cons,
  comparisons, and predicates.

### Changed

- **`twig-lexer` and `twig-parser` now compile their grammars at
  build time.**  Earlier drafts called `std::fs::read_to_string`
  on `code/grammars/twig.tokens` / `twig.grammar` at every lexer/
  parser construction.  A new `build.rs` in each crate calls
  `grammar_tools::token_grammar::parse_token_grammar` (or the
  parser-grammar equivalent) and `grammar_tools::codegen::*` to
  emit Rust source code that reconstructs the parsed grammar as a
  `OnceLock<…>` static.  `lib.rs` `include!`s the generated code
  and exposes the grammar via `pub fn twig_token_grammar()` /
  `pub fn twig_parser_grammar()`.
  Result: zero runtime file I/O, Miri-compatible without
  `-Zmiri-disable-isolation`, builds catch malformed grammars
  early.

### Uses (existing): `grammar_tools::compiler`

- The build scripts call `grammar_tools::compiler::compile_token_grammar`
  / `compile_parser_grammar` — the canonical grammar-to-Rust
  compiler that already lived in the workspace.  An earlier draft
  of this PR added a duplicate `codegen` module; that has been
  removed in favour of using the existing one.  The lib.rs of
  twig-lexer / twig-parser wraps the compiler-generated
  `token_grammar()` / `parser_grammar()` constructors in a
  `OnceLock<…>` so the struct is materialised exactly once per
  process — the generated code constructs eagerly each call, so
  the OnceLock ensures we don't redo it on every
  `create_twig_lexer` / `create_twig_parser` invocation.

### Tests across the diff

- twig-vm: 35 unit + 2 doc — all pass on stable Rust **and Miri**.
- grammar-tools: 144 unit + 3 doc (codegen module adds 8 of those).
- twig-lexer: 18 unit + 1 doc, identical surface — grammar source
  is now compiled at build time but `tokenize_twig` /
  `create_twig_lexer` API unchanged.
- twig-parser: 33 unit + 2 doc, identical surface.
- twig-ir-compiler: 45 unit + 2 doc, identical (compiles against
  the build-time grammar via twig-parser).

### Hardening

- Clippy-clean across all touched crates.
- Miri runs cleanly on `lang-runtime-core`, `lispy-runtime`, and
  `twig-vm` — no UB, no isolation-disable required.
