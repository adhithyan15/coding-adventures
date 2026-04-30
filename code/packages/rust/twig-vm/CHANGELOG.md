# Changelog — twig-vm

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

### Added (in support: `grammar-tools`)

- New `grammar_tools::codegen` module — emits Rust source from a
  parsed `TokenGrammar` / `ParserGrammar`.  Functions:
  - `token_grammar_to_rust_source(&TokenGrammar, fn_name) -> String`
  - `parser_grammar_to_rust_source(&ParserGrammar, fn_name) -> String`
  Both produce a `pub fn <fn_name>() -> &'static …` declaration
  with `OnceLock`-cached initialisation.  Fully-qualified paths
  (`::grammar_tools::…`) so generated code works regardless of
  consumer-crate `use` statements.  Deterministic output:
  HashMap iteration is sorted before emit so reproducible builds
  hash to the same `.rs` file.
  - 8 new unit tests in `grammar_tools::codegen::tests` covering
    string escaping (incl. control chars), empty-field handling,
    recursive `GrammarElement` types, deterministic ordering, and
    output-shape sanity.

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
