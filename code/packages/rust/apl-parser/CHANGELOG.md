# Changelog

## [0.1.1] - 2026-07-12

### Fixed

- **`MAX_RULE_DEPTH` was silently unsafe for one of two distinct crash
  shapes.** The `0.1.0` value (`150`) was derived only from parenthesised
  nesting (`((((…))))`, crash floor 209 on a default ~2 MiB stack). Found
  while building `apl-runtime` (MA-4e) on top of this crate: a flat,
  *unparenthesised* dyadic chain (`1+1+1+…+1`) also recurses through
  `value_expr`'s own right-recursive continuation — every `+1` costs one
  more `parse_rule` level — but at a **higher native-stack cost per level**
  than a `(...)` wrap, so its crash floor is much lower: 136 terms safe, 137
  crashing. `150` sat *above* that floor, meaning inputs still nominally
  under the configured cap (depth ~137) could crash the process outright —
  not "the guard trips a little late," but "the guard's own configured
  value permitted a crash." `MAX_RULE_DEPTH` is now `100` (~26.5% below the
  binding flat-chain floor of 136, comparable margin to every sibling
  crate's own cap), verified safe for **both** shapes: parens up to 47
  levels, flat chains up to 94 terms, neither crashing a default-stack
  thread even thousands of levels past the cap. See `MAX_RULE_DEPTH`'s doc
  comment in `src/lib.rs` for the full measurement.
- Added 4 new permanent regression tests (flat-chain analogues of the 3
  existing parens-only depth-guard tests, plus an updated exact-boundary
  test for the corrected cap) — without these, a future change to
  `MAX_RULE_DEPTH` could silently re-introduce this exact crash while the
  parens-only tests kept passing. 26 tests total (was 22).
- No grammar, token, or AST-shape change — `program`/`value_expr`/
  `function_expr` and every other rule are byte-for-byte identical to
  `0.1.0`. Any already-written APL source that parsed before still parses
  identically; only pathologically deep unparenthesised chains (>94 terms)
  now correctly fail cleanly instead of crashing the process.

## [0.1.0] - 2026-07-11

### Added

- Initial grammar-driven Rust APL parser (MA05 §6, task MA-4d): consumes
  `apl-lexer`'s token stream, drives it through the compiled
  `code/grammars/apl/apl.grammar` (the two-nonterminal `value_expr`/
  `function_expr` design), and produces a `GrammarASTNode` CST rooted at
  `program`. Exposes `create_apl_parser`/`parse_apl`/`try_parse_apl`,
  matching every sibling parser crate's established shape.
- Ships with a recursion-depth cap (`MAX_RULE_DEPTH = 150`) from day one —
  unlike `macsyma-parser`/`matlab-parser`/`wolfram-parser`, which retrofitted
  their caps in a later release, this crate never had the unguarded gap.
  Derived via the same throwaway-isolated-subprocess binary-search
  methodology as those three crates, not copied from them: measured crash
  floor is 209 safe / 210 crashing on a bare ~2 MiB stack thread — *lower*
  than the ~275-280 measured for the other three grammars, the opposite of
  the "APL's shallower one-precedence-tier grammar should have a
  same-or-higher floor" prediction (a real instance of the DoS-guard
  verification lesson: reasoning from a sibling crate's finding is not a
  substitute for measuring this crate's own grammar). `150` (~28% headroom)
  permits 72 real `(...)` nesting levels — measured directly (72 parses
  cleanly, 73 trips the cap) — comfortably more than
  MACSYMA/MATLAB/Wolfram's ~14, since each APL nesting level costs far fewer
  `parse_rule` frames despite the lower absolute floor.
- 19 tests: one per grammar production (literals/stranding, assignment
  including right-associative chaining, monadic/dyadic application, the
  right-to-left dyadic chain, reduce/scan/outer-product operators,
  parenthesised grouping, every comparison and primitive function glyph,
  comments/blank lines, multi-line programs, malformed-input rejection) plus
  the 3 depth-cap regression tests every sibling parser crate carries.
- Marks MA-4d done in `MA05-apl-language.md` §6.
