# Changelog — coding-adventures-wolfram-parser

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

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
