# Changelog

## [0.1.0] - 2026-07-13

### Added

- Initial grammar-driven Rust J parser (MA06 §6, task MA-6c): consumes
  `j-lexer`'s token stream, drives it through the compiled
  `code/grammars/j/j.grammar` (the two-nonterminal `noun_expr`/`verb_expr`
  design, reused from `apl.grammar` almost verbatim, plus the one genuinely
  new `verb_train` production), and produces a `GrammarASTNode` CST rooted
  at `program`. Exposes `create_j_parser`/`parse_j`/`try_parse_j`, matching
  every sibling parser crate's established shape.
- Ships with a recursion-depth cap (`MAX_RULE_DEPTH = 70`) from day one,
  following MA06 §6's explicit instruction to measure the *actual*
  native-stack crash floor for **every** distinct way this grammar can
  recurse deeply, rather than assuming `apl-parser`'s own (twice-corrected)
  cap or floor ordering transfers unchanged. `j.grammar` has three such
  shapes, all measured independently:

  1. **Parenthesised nesting**, `((((…5…))))`. Measured (binary search, a
     `std::thread::spawn` worker on the **default ~2 MiB stack**, uncapped
     `GrammarParser`, **debug** build to match `cargo test`'s own profile):
     safe up to 100 levels, crashes the process at 101.
  2. **A flat, unparenthesised dyadic chain**, `1+1+1+…+1` — the exact shape
     that bit `apl-parser` originally (see that crate's own `0.1.1`
     changelog entry). Same methodology: safe up to 135 terms, crashes at
     136 — close to `apl-parser`'s own measured flat-chain floor (136 safe /
     137 crashing), since the grammar shape and per-level native-stack cost
     are nearly identical between the two crates.
  3. **A long train**, `(+ + + … +) 5` (N `+` teeth in one paren pair,
     applied monadically) — this grammar's own genuinely new recursion
     shape, with no `apl-parser` precedent, exercising `verb_train`'s flat
     `train_tooth { train_tooth }` repetition. Same methodology: safe up to
     200 teeth, crashes at 201.

  **The binding constraint is a genuine surprise**: parenthesised nesting
  (100) is the *lowest* of the three floors here, ahead of the flat chain
  (135) and the train (200) — the **opposite** ranking from `apl-parser`,
  where the flat chain was binding. This is precisely the failure mode MA06
  §6 warns against: a shape's measured floor (even from this same grammar's
  *other* shapes, not just a sibling crate) does not predict another
  shape's floor. `MAX_RULE_DEPTH` is set to `70` — about 30% below the
  binding 100 floor (comparable margin to `apl-parser`'s own ~26.5%), and
  therefore safely below the other two floors as well. Measured headroom at
  `70` (using the *capped* parser, so no crash risk at all): parenthesised
  nesting parses cleanly to 32 levels (33 trips the cap), a flat chain to 63
  terms (64 trips), and a train to 61 teeth (62 trips) — all three far
  beyond any hand-written J expression's needs. See `MAX_RULE_DEPTH`'s doc
  comment in `src/lib.rs` for the full derivation.
- 33 tests: one per grammar production (bare number, numeric stranding,
  local/global/chained assignment, monadic/dyadic application, the
  right-to-left dyadic chain, reduce, scan, `@` compose both monadically and
  dyadically, parenthesised grouping, every comparison verb, every primitive
  verb parsed monadically, comment/blank lines, multi-line programs), six
  dedicated train tests (2-tooth hook, 3-tooth fork, 4+-tooth fork, a
  leading-noun fork, dyadic train application, a train nested inside a
  train), a dedicated `/`-is-reduce-not-division structural regression test,
  malformed-input rejection, plus the 9 depth-cap regression tests (3 shapes
  × {huge-input-on-32MiB-worker, exact-boundary-at-the-cap,
  cap-trips-before-overflow-on-default-stack}) — three more than
  `apl-parser`'s own 6, for this grammar's third (train) recursion shape.
- Marks MA-6c done in `MA06-j-language.md` §6.

### Design notes / divergences from the task's suggested test examples

- The task's suggested `@`-compose example, `(+@-) A`, is **not** valid
  syntax under this grammar: `verb_expr`'s `LPAREN verb_train RPAREN`
  alternative requires **2 or more** train teeth, but `+@-` greedily parses
  as **one** whole tooth via `verb_expr`'s own `simple_verb [ AT verb_expr ]`
  alternative (the AT-continuation is tried before the grammar ever
  considers treating `+`/`-` as two separate teeth), leaving nothing for a
  required second tooth — confirmed empirically (a "expected train_tooth,
  got `)`" parse error) while writing the test. A lone compose doesn't need
  parens at all, since `verb_expr` already covers it wherever a `verb_expr`
  is expected; the test instead confirms `+@-A` (monadic) and `A+@-B`
  (dyadic) both parse.
