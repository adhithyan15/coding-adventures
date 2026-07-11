# Changelog — coding-adventures-wolfram-parser

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.4.1] — 2026-07-11

### Fixed

- **DoS hardening (partial — see "Known limitation" below)**:
  `create_wolfram_parser` / `parse_wolfram` / `try_parse_wolfram` now opt the
  underlying `GrammarParser` into a recursion-depth cap
  (`MAX_RULE_DEPTH = 2000`) via `.with_max_depth(...)`. Previously the parser
  recursed once per nested `(...)`/`f[...]` layer with no limit at all;
  deeply nested input (thousands of levels) could overflow the *native*
  thread stack — an uncatchable process abort — before ever reaching a
  `Result`-returning entry point, on **any** thread regardless of stack size.
- Wolfram's precedence cascade is unusually deep (`assignment` down to
  `group` is a 20-rule chain — the exact example that motivated keeping
  `parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH` (128) as an opt-in,
  per-caller default rather than a global one), so 128 would have allowed
  only ~5 real nesting levels — too easy for ordinary nested function calls
  like `f[g[h[x]]]` to trip. A first pass at `200` (empirically: safe at 275
  raw `parse_rule` frames / crashing at 278 on a bare ~2 MiB default-stack
  thread) broke `wolfram-runtime`'s own existing, legitimate
  `moderate_nesting_still_evaluates` test (40 levels of real nesting) — that
  crate already parses on its own 512 MiB worker thread
  (`EVAL_STACK_SIZE`) gated by a token-count budget rather than a
  parser-level depth cap, because even 40 real nesting levels costs ~840
  `parse_rule` frames, already past the bare-2-MiB-stack crash floor
  regardless of the cap chosen here. `2000` (98 real nesting levels) is
  calibrated for that big-stack deployment instead. See the `MAX_RULE_DEPTH`
  doc comment in `src/lib.rs` for the full derivation.
- **Known limitation, disclosed rather than silently shipped**: `2000` does
  **not** protect a caller that invokes `create_wolfram_parser` / `parse_wolfram`
  / `try_parse_wolfram` on an ordinary default-stack thread — such a caller
  remains exposed to the native-stack-overflow DoS past ~11 real nesting
  levels, exactly as before this change (no `with_max_depth` value can sit
  both below the ~276-frame bare-stack crash floor and above the ~840 frames
  `wolfram-runtime`'s own legitimate 40-level test needs — those two
  constraints are mathematically incompatible for this grammar). Today
  `wolfram-runtime` is this crate's only consumer, and it already mitigates
  correctly (big stack + its own token-count cap checked before parsing). A
  future caller without an equivalent strategy should either follow the same
  pattern or call `.with_max_depth(...)` with an explicit, much lower value
  suited to a bare thread (e.g. `150`) directly on the `GrammarParser`
  `create_wolfram_parser` returns.
- Added 3 regression tests exercising the guard on the real Wolfram grammar,
  run on an enlarged worker thread matching `wolfram-runtime`'s own
  `EVAL_STACK_SIZE` (not a bare thread, since a bare thread cannot validate
  this crate's cap without also tripping over the limitation above): a
  deep-nesting test proving the cap trips instead of overflowing, and a
  boundary test proving legitimate nesting up to 98 levels still parses
  while 99 trips the cap.

## [0.4.0] — 2026-06-25

### Added (W-21 — pattern operator grammar)

- Parser rules for the W-20 pattern constructs' operator sugar (MA04 §23),
  modelled on the existing `replaceall`→`ReplaceAll` and `rule`→`Rule` rules and
  inserted at Wolfram precedence (loosest→tightest):
  - `replaceall = rule { ( REPLACEALL | REPLACEREPEATED ) rule }` — the `//.`
    arm joins `/.` at the same left-associative replace level.
  - `rule = condition [ ( RULE | RULEDELAYED ) rule ]` — `->`/`:>` now recurse
    on the new `condition` rule, so `/;` binds tighter than `->`.
  - `condition = alternatives [ CONDITION condition ]` — the new `/;` level,
    right-associative, looser than `|`.
  - `alternatives = logical_or { ALTERNATIVES logical_or }` — the new `|` level,
    infix/left-associative, folded into one n-ary `Alternatives` by the runtime.
  - `patterntest = postfix { PATTERNTEST postfix }` — the new `?` level inserted
    between `mapapply` and `postfix`, so `?` binds tightest (just above
    application); `mapapply`'s operand changed from `postfix` to `patterntest`.
- The embedded `_grammar.rs` was regenerated via the Rust grammar-tools CLI
  (`compile-grammar`); generated files are never hand-edited.

## [0.3.0] — 2026-06-19

### Added (W-11 — pure-function grammar)

- A `slot` atom (`slot = HASH [ NUMBER ] | SLOTSEQ`) for `#`, `#n`, and `##`.
- A low-binding postfix `amp` level for the `&` pure-function postfix:
  `amp = comparison AMP { AMP } { amp_apply } | comparison`, placed just below
  `logical_not` and ABOVE the whole comparison/arithmetic stack, so `&` binds
  looser than every arithmetic/comparison operator but tighter than `,`. Thus
  `#^2 &`, `# + 1 &`, and `Mod[#,2]==0 &` all wrap the *whole* body. The
  `amp_apply` suffix lets a pure function be applied immediately (`(#^2)&[5]`)
  and chains (`f&[1][2]`, `f&[[i]]`); it appears only after at least one `&`, so
  the no-`&` path falls through to ordinary application (no ambiguity).

The embedded `_grammar.rs` was regenerated via the Rust grammar-tools CLI.

## [0.2.0] — 2026-06-17

### Added (W-6 — operator sugar)

- New grammar rules for the W-6 operator sugar (`code/grammars/wolfram.grammar`,
  regenerated into `_grammar.rs`):
  - `mapapply = postfix { ( MAP | APPLY ) postfix }` — a new infix precedence
    level (between `power` and `postfix`, left-associative) for `f /@ x` and
    `f @@ x`.
  - `postfix` gains the part-sugar arm
    `LDBRACKET arglist RBRACKET RBRACKET` alongside `f[…]` application, so
    `x[[i]]`, `x[[i, j]]`, `x[[1]][[2]]`, and `f[x][[1]]` all parse as postfixes.
- The double-bracket form closes with two single `RBRACKET`s (there is no `]]`
  token), so nested ordinary application `f[g[x]]` is unaffected (regression
  guarded by a parser test).

### Notes

- An empty `x[[]]` is a syntax error (unlike `f[]`), since `[[ … ]]` requires at
  least one index. See `code/specs/MA04-wolfram-language.md` §9.

## [0.1.0] — 2026-06-16

### Added

- Initial release (W-3). A parser for the Wolfram Language M-expression subset,
  a thin wrapper over the generic `GrammarParser` with the committed `_grammar.rs`
  compiled from `code/grammars/wolfram.grammar`. Exposes `parse_wolfram` /
  `try_parse_wolfram` / `create_wolfram_parser`, producing a `GrammarASTNode`
  rooted at `program`.
- The parse tree's rule names (`assignment`, `replaceall`, `rule`, `additive`,
  `multiplicative`, `power`, `postfix`, `atom`, `list`, …) are the surface forms
  the W-4 `wolfram-runtime` will lower into canonical `symbolic-ir` heads.

### Notes

- Sibling of `r-parser` / `macsyma-parser`. See `code/specs/MA04-wolfram-language.md`.
