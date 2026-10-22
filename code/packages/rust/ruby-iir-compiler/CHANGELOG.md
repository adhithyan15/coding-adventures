# Changelog

## Unreleased — grammar-tools adjacency predicates (E6)

### Changed
- `tests/cst_contract.rs`'s `referenced_tokens` covers the new
  `GrammarElement::AdjacencyPredicate` (`@no_space_before` …), which tests
  the trivia between tokens and names no token. The exhaustive `match`
  stopped compiling when grammar-tools gained the variant.

## Unreleased — LANG84 phase 5b, slice 1: release recorded and selectable through lang-aot

### Added
- The compiled module records the release that produced it:
  `IIRModule::release` is `Some("3.3.5" | "3.3.6" | "3.3.7")` from
  `compile_source_at`, `compile_source` (the default, `3.3.6`) and
  `compile_cst` alike. lang-aot's `--lang ruby@<release>` relies on it.

### Changed
- `FrontendError::ReleaseNotAvailable.stage` is self-describing. A row with
  no grammar pair now reads `no grammar pair: grammar_stage none` instead of
  the bare `none`, next to the existing `no IIR profile: …`, so lang-aot
  can show either as `release X cannot compile yet (…)`. ruby-parser's
  regeneration-bug phrases (`token table missing (regenerate ruby-lexer)`)
  already say what is wrong and are kept as they are (security review,
  info).

## Unreleased — LANG84 phase 5a (= LANG83 PR2): the R0 release pairs, LANG83 ops

### Changed (breaking)
- The compiler now walks the tree `ruby-parser`'s **release API** builds
  (`parse_ruby_at`: the generic `GrammarLexer`/`GrammarParser` over
  `ruby<release>.tokens`/`.grammar`). The hand-written `ruby-lexer` is no
  longer on the IIR path; `tests/iir_path.rs` keeps it that way, and
  `ruby-lexer` is only a dev-dependency (for the C8 contract test's token
  tables).
- LANG83 operations replace every Ruby-named builtin: `/` is
  `i64_div_floor`, `%` (new) is `i64_mod_floor`, `puts` writes each value
  with `print_i64`, `puts()` with `putchar 10`. `rb_int_div`,
  `rb_puts_int`, `rb_puts_empty`, `rb_puts_two_ints` and
  `rb_puts_three_ints` are gone, so every lang-aot backend runs the output.
- A zero divisor is no longer a compile error: like Ruby's
  `ZeroDivisionError` it traps at run time (`Trap.DivZero`) after the
  earlier output. Values after a certain trap are not range-checked.
- `main` returns `i64` 0 instead of `ret_void`, the exit status every AOT
  entry point expects (and what keeps the CLR and BEAM launchers' integer
  readers happy).
- Errors are a typed `FrontendError` (LANG84 C6) instead of `String`.
- `puts` takes any number of arguments, bare or parenthesised, as the 3.3
  grammar admits; an empty program compiles. `010` and `0x10` are refused by
  the grammar (they lex as `OCTAL_INTEGER`/`HEX_INTEGER`, not `NUMBER`).
- `puts -7` (bare command whose argument starts with `-`) is now refused:
  the R0 grammar cannot tell unary from binary minus without the space
  before it (LANG84 E6). Write `puts(-7)`. This narrows LANG80's accepted
  set; nothing else LANG80 accepted is lost.

### Security
- `i64::MIN % -1` is folded to `const 0` (Ruby's answer) instead of emitting
  `i64_mod_floor`: it is the one modulus a machine `rem` faults on (x86
  SIGFPE, LLVM UB, a CLR exception, a Rust `%` panic). LANG83's backends
  guard it too; this keeps the edge out of every backend. Regression corpus
  case `min_mod_minus_one` and a unit test.
- Diagnostics escape every caller-supplied text (`rubyvm` prints the path
  with `{:?}`; `UnknownRelease` no longer double-escapes).

### Added
- `compile_source_at(source, release, module_name, &FrontendBudgets)`,
  `compile_cst(ast, release, module_name)` (caller-supplied trees, measured
  against item/depth/text budgets first), `run_source_at`, `run_module`,
  `RunOutput { stdout, trap }`, `RunError`.
- `profiles.rs`: `ReleaseProfile { name, int_div }`, profile `ruby33` for
  3.3.5–3.3.7. 0.49/0.50 have no profile (`puts` is `IO#puts` only and
  `Fixnum#/` truncates there; cited to the tags' `io.c`/`numeric.c`), so
  they are `ReleaseNotAvailable`.
- `DISPATCHED_RULES` / `DISPATCHED_TOKENS` and `tests/cst_contract.rs`
  (LANG84 C8); `tests/release_subsets.rs` (stage I over the new
  `code/grammars/ruby/corpus/rb-int-puts` corpus, IIR identity across
  3.3.5–3.3.7, CSV/profile agreement, host-ruby oracle; the oracle folds
  Windows' `\r\n` stdout back to `\n` before comparing with `.expect`);
  `tests/hostile.rs` (C5 probes in a child process).
- `rubyvm --release <id>`; a trap prints the output so far, then
  `trap: <kind>` on stderr, exit 1.

### Earlier unreleased work (LANG80 pilot)


- Reject forged direct-AST unary/binary arithmetic and grouping tokens by
  validating canonical enum kinds, exact spelling and absence of named
  overrides. Cover bare and every supported parenthesized argument position,
  malformed grouping shape and preserved source arithmetic/IIR behavior.
  Retain the current source subset, range proofs, budgets and output behavior.

- Lower exactly three positional integer arguments in a parenthesized `puts`
  call directly to IIR and Rust `vm-core`. Validate both commas and all argument
  wrappers, preserve source-order expression evaluation, and check combined
  output size before appending three newline-terminated values. Add forged-AST,
  integer-range, third-argument budget, and atomic output-boundary coverage.

- Lower exactly two positional integer arguments in a parenthesized `puts`
  call to a bounded Rust VM builtin, emitting two newline-terminated values
  atomically after both expressions execute. Validate direct-AST comma and
  statement/argument wrapper shapes before lowering.

- Lower an exact zero-argument `puts()` grammar call directly to a bounded
  Rust VM builtin that emits one newline; validate direct-AST delimiters by
  token kind and effective grammar type on both parenthesized call forms.

- Accept one-argument bare `puts expression` calls through the existing Ruby
  parser tree and direct IIR lowering, with the same integer bounds as `puts(...)`.
- Reject forged direct-AST `puts` callees with non-name token classifications
  in both supported call forms.
- Use the fallible Ruby parser constructor so malformed lexical input is
  returned as a compiler error.
- Reject numeric-looking strings and legacy leading-zero octal literals in the
  decimal-only pilot instead of silently compiling them as base-10 integers.

## 0.1.0

- Add a bounded Ruby source-to-IIR numeric compiler and native VM runner.
- Bound directly supplied AST token counts and text fields before lowering.
