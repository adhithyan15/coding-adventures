# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-07-13

### Fixed — recursion-depth guard against native stack overflow (DoS)

`create_s_parser`/`try_parse_s` built their `GrammarParser` with no
recursion-depth cap, even though `s-repl` feeds this parser arbitrary,
untrusted source at an interactive prompt. Deeply-nested input
(`((((...))))`) would recurse until it overflowed the native thread stack —
an uncatchable process abort — before this crate's own `Result`-returning
entry points ever got a chance to report anything.

Rather than reuse `r-parser`'s bespoke `MAX_RULE_DEPTH` unmeasured — the two
grammars share rule names but are not byte-identical in compiled shape, and
a sibling's measured floor is a prior, not a substitute for measuring this
grammar's own native-stack behaviour — this crate's floor was measured
independently the same way: binary-searching an *uncapped* parser against
increasing real nesting depth on a default-stack worker thread (crashes at
24 levels, safe at 23; in rule-frame terms, safe through at least 298 on the
same 5000-level adversarial input, slightly higher than `r-parser`'s
measured floor). `r-parser`'s value (200) was then confirmed safe here too,
so both crates share the same cap now that it's independently verified for
each.

- Added `MAX_RULE_DEPTH: usize = 200` and wired it into both
  `create_s_parser` and `try_parse_s` via `.with_max_depth(...)`.
- 3 new regression tests, mirroring `r-parser`'s own:
  `test_deeply_nested_input_returns_error_not_overflow` (5000 levels on a
  32 MiB worker thread returns a clean `Err`, never crashes),
  `test_nesting_up_to_cap_still_parses` (15 levels parse, 16 trips),
  `test_opt_in_cap_trips_before_overflow_on_default_stack` (5000 levels on
  a **default**-stack thread still returns `Err` cleanly, proving the cap
  trips before the native stack would).

No change to behaviour for any input that nests below the cap — every real
S program and every existing test.

## [0.3.0] - 2026-06-19

### Changed

- **R-19 grammar**: the `arg` rule now allows an **empty named-argument value** —
  `arg = NAME EQ [expr] | expr`. A named argument may omit its value (`a = ,` /
  `a = )`), which `switch`'s empty-arm fall-through relies on
  (`switch("a", a = , b = "hit")` → `"hit"`). The optional `expr` is only ever
  followed by `COMMA` or `RPAREN`, so the rule stays LL(1). This is a shared
  S/R grammar change; `r-parser` carries the mirror. Regenerated the embedded
  `src/_grammar.rs` (single-line functional change: `expr` → `Optional(expr)`).

## [0.2.0] - 2026-06-15

### Added

- `special` rule for `%op%` infix operators (left-associative, between `* /`
  and `:`).
- `dollar_suffix` (`df$name`) and `dindex_suffix` (`x[[k]]`) postfixes; 2-D
  `index_suffix` (`df[i, j]`) is now accepted.

### Changed

- Corrected the operator-precedence cascade so `:` binds tighter than `+ - * /`
  (matching R). Regenerated the embedded `_grammar.rs`.

## [0.1.0] - 2026-06-15

### Added

- Initial release of the historical Bell Labs S parser crate.
- `parse_s()` and `try_parse_s()` entry points returning a `GrammarASTNode`
  rooted at the `program` rule.
- `create_s_parser()` factory returning a configured `GrammarParser`.
- Embedded `s.grammar` (`src/_grammar.rs`), generated ahead of time.
- Full S v1 expression grammar with a precedence cascade: assignment
  (`<- _ <<- ->`, right-associative), comparison, the `:` sequence operator,
  additive/multiplicative arithmetic, prefix unary minus, right-associative
  `^`, and left-associative call/index postfixes.
- Function definitions with positional and default parameters; calls with
  positional and named (`name = expr`) arguments; `if`/`else`, `for`, `while`,
  and `repeat` as expressions; `{ }` blocks; and `( )` grouping.
- Statements separated by newlines or semicolons; calls and indices may span
  multiple physical lines.
