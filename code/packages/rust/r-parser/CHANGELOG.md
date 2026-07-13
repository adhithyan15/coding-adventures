# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-07-13

### Fixed — recursion-depth guard against native stack overflow (DoS)

`create_r_parser`/`try_parse_r` built their `GrammarParser` with no
recursion-depth cap (the shared `parser` crate's default is unbounded), even
though `r-repl` feeds this parser arbitrary, untrusted source at an
interactive prompt. Deeply-nested input (`((((...))))`) would recurse until
it overflowed the native thread stack — an uncatchable process abort — before
this crate's own `Result`-returning entry points ever got a chance to report
anything.

The shared crate's generic `DEFAULT_MAX_RULE_DEPTH` (128) turned out unsafe
*by rejection* for this grammar: measured directly, 128 rule-frames only
covers 8 real parenthesised-nesting levels, well within plausible real R
code. Added a bespoke `MAX_RULE_DEPTH = 200`, empirically measured the same
way `apl-parser`/`j-parser`/`macsyma-parser` measured their own bespoke
values: binary-searching an *uncapped* parser against increasing real nesting
depth on a default-stack worker thread (crashes at 22 levels, safe at 21;
in rule-frame terms, safe through 297, crashes at 298 on the same 5000-level
adversarial input). 200 sits about 33% below that measured floor and still
supports 14 real nesting levels before tripping.

- Added `MAX_RULE_DEPTH: usize = 200` and wired it into both
  `create_r_parser` and `try_parse_r` via `.with_max_depth(...)`.
- 3 new regression tests: `test_deeply_nested_input_returns_error_not_overflow`
  (5000 levels on a 32 MiB worker thread returns a clean `Err`, never
  crashes), `test_nesting_up_to_cap_still_parses` (14 levels parse, 15
  trips), `test_opt_in_cap_trips_before_overflow_on_default_stack` (5000
  levels on a **default**-stack thread still returns `Err` cleanly, proving
  the cap trips before the native stack would).

No change to behaviour for any input that nests below the cap — every real
R program and every existing test.

## [0.3.0] - 2026-06-19

### Changed

- **R-19 grammar**: the `arg` rule now allows an **empty named-argument value** —
  `arg = NAME EQ [expr] | expr`, mirroring the shared S/R grammar change. A named
  argument may omit its value (`a = ,` / `a = )`), enabling `switch`'s empty-arm
  fall-through (`switch("a", a = , b = "hit")` → `"hit"`). The optional `expr` is
  only ever followed by `COMMA` or `RPAREN`, so the rule stays LL(1). Regenerated
  the embedded `src/_grammar.rs` (single-line functional change:
  `expr` → `Optional(expr)`).

## [0.2.0] - 2026-06-16

### Added

- **R-9 grammar**: a `pipe` rule (`x |> f()`) at the special-operator precedence
  level, and a `\(params) body` alternative on `func_def` so the backslash
  lambda produces the same node the evaluator already handles.

## [0.1.0] - 2026-06-16

### Added

- Initial release of the R parser crate — item R-2 of the R frontend.
- `parse_r()` / `try_parse_r()` and the `create_r_parser()` factory, producing a
  `GrammarASTNode` rooted at the `program` rule.
- Embedded `r.grammar` (`src/_grammar.rs`), generated ahead of time.
- `r.grammar` mirrors `s.grammar`'s rule names exactly so the shared `s-runtime`
  tree-walker can evaluate R programs unchanged. The grammar differences from S:
  - `=` and `->>` are assignment operators (alongside `<-`, `<<-`, `->`);
  - the typed-`NA` atoms `NA_integer_` / `NA_real_` / `NA_character_`.
- 11 tests covering R's assignment operators, the `=` named-arg vs assignment
  distinction, typed NAs, the shared precedence cascade, indexing/`[[`/`$`,
  functions, control flow, multi-line input, and error reporting.
