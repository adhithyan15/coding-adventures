# Changelog

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
