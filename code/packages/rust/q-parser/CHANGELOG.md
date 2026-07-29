# Changelog

## [0.1.0] - 2026-07-21

### Added

- Initial grammar-driven Rust Q parser (MA11 §6, task MA-11c): consumes
  `q-lexer`'s token stream, drives it through the compiled
  `code/grammars/q/q.grammar`, and produces a `GrammarASTNode` CST rooted at
  `program`. Exposes `create_q_parser`/`parse_q`/`try_parse_q`, matching
  every sibling parser crate's established shape.
- `q.grammar` reuses `apl.grammar`/`j.grammar`'s two-nonterminal
  `noun_expr`/`verb_expr` design UNCHANGED for primitive-verb application
  (MA11 §3: "reused UNCHANGED... this is the easy, mechanical part") — one
  precedence tier, right-to-left, monadic/dyadic dispatch left to a later
  pass. Q has no trains and no `@` compose in this cut, so `verb_expr` is
  flatter than `j.grammar`'s own.
- Two genuinely new productions (MA11 §3 bullets 1 and 3):
  - **Function literals** (`function_literal`): `{[x;y] stmt; stmt; ...}`,
    an optional bracketed semicolon-separated parameter list (`param_list`)
    followed by a semicolon-separated statement sequence (`stmt_seq`).
    Assignable/passable as an ordinary noun value without being applied
    (`f:{x+y}`).
  - **Dual list-literal syntax**: numeric stranding is reused unchanged from
    APL/J; the explicit `(a;b;c)` form (`list_literal`) is a new `term`
    alternative, disambiguated from plain parenthesised grouping purely by
    the presence of a top-level `;` — this falls out of the packrat
    parser's ordered Alternation with **no explicit lookahead needed**:
    `term`'s plain `LPAREN noun_expr RPAREN` alternative is tried first and
    simply fails (backtracking cleanly) whenever a top-level `;` is
    present, falling through to `list_literal`.
- **The one genuinely hard grammar problem**: making a function literal
  "applied with the same juxtaposition/@ mechanism as a primitive verb" (MA11
  §3 bullet 1) without adding a new named application production. Solved by
  extending `verb_expr` with two new alternatives (a bare `NAME` and an
  inline `function_literal`) and widening `noun_expr`'s existing optional
  dyadic continuation from `[ verb_expr noun_expr ]` (APL/J's shape,
  unchanged for primitives) to `[ verb_expr noun_expr | noun_expr ]` — one
  more inner alternative, not a new top-level rule. This correctly parses
  monadic named-function calls (`f 5`), dyadic named-function calls
  (`2 f 3`), and both arities of an inline anonymous lambda (`{x*2} 5`,
  `2 {x+y} 3`), verified directly by dedicated tests. **A disclosed,
  deliberate limitation**: real K/Q resolves a 3+-name juxtaposition chain's
  verb/noun roles (`f g h`) via arity tracking during parsing (a symbol
  table), which this repo's shared context-free `GrammarParser` has no
  mechanism for; this grammar's best-effort resolution (try
  `verb_expr noun_expr` before a bare `noun_expr` fallback) correctly
  handles the required case (`x f y` = dyadic `f(x,y)`) but does not
  attempt to replicate full K/Q valence resolution for longer chains — see
  `q.grammar`'s own header comment and this crate's README for the full
  rationale. `q-runtime` (MA-11d) needs to be aware of this.
- Ships with a recursion-depth cap (`MAX_RULE_DEPTH = 32`) from day one,
  following MA11 §6's explicit instruction to measure the *actual*
  native-stack crash floor for **every** distinct way this grammar can
  recurse deeply — parenthesised nesting, a flat right-recursive dyadic
  chain, and (genuinely new to this grammar family, no sibling crate has
  measured this shape before) nested function-literal bodies — rather than
  assuming any prior crate's floor or ordering transfers. All three measured
  independently (binary search, a throwaway subprocess per data point, a
  `std::thread::spawn` worker on the **default ~2 MiB stack**, an
  *uncapped* `GrammarParser`, **debug** build to match `cargo test`'s own
  profile):

  1. **Parenthesised nesting**, `((((…5…))))`. Safe up to 101 levels,
     crashes the process at 102.
  2. **A flat, unparenthesised dyadic chain**, `1+1+1+…+1`. Safe up to 115
     terms, crashes at 116.
  3. **Nested function-literal bodies**, `{{{…5…}}}` — this grammar's own
     genuinely new recursion shape, exercising
     `function_literal -> stmt_seq -> statement -> assignment -> noun_expr
     -> term -> function_literal` (six named-rule hops per level, versus
     parenthesised nesting's two). Safe up to only 45 levels, crashes at
     46 — **by far the lowest of the three floors**, and the shape this
     cap is actually chosen against.

  `MAX_RULE_DEPTH` is set to `32` — about 29% below the binding
  nested-function-literal floor of 45 (comparable margin to `apl-parser`'s
  own ~26.5% and `j-parser`'s own ~30%), safely below the other two floors
  (101, 115) as well. Measured headroom at `32` (using the *capped* parser,
  so no crash risk at all): parenthesised nesting to 13 levels (14 trips),
  flat chain to 26 terms (27 trips), nested function literals to 4 levels
  (5 trips) — modest for the function-literal shape in absolute terms, but
  not a practical limitation, since MA11 §4 puts nested function-literal
  *definitions* out of this cut's semantic scope entirely (no
  closure/scoping model specified for them); the cap exists to reject a
  pathologically crafted deep input cleanly, not to bound realistic
  programs. See `MAX_RULE_DEPTH`'s doc comment in `src/lib.rs` for the full
  derivation.
- 38 tests + 1 doctest: one per grammar production (literals/stranding,
  chained assignment, monadic/dyadic primitive application, the
  right-to-left dyadic chain, every adverb, every comparison and primitive
  verb parsed monadically, parenthesised grouping, the list-literal-vs-
  grouping disambiguation with a structural check, function-literal
  definition with explicit params, with implicit params, with a
  multi-statement body, assignability without calling, calling a named
  function both monadically and dyadically, calling an inline lambda both
  monadically and dyadically, a function body calling another
  already-defined function, comment/blank lines, multi-line programs,
  three distinct malformed-input rejections), plus 9 depth-cap regression
  tests (3 shapes × {huge-input-on-32MiB-worker, exact-boundary-at-the-cap,
  cap-trips-before-overflow-on-default-stack}).
- `code/packages/rust/Cargo.toml` workspace registration alongside
  `q-lexer` and the other array-language frontend crate groups.

### Notes for `q-runtime` (MA-11d)

- The 3+-name juxtaposition-chain limitation noted above (this grammar
  builds a best-effort shape for `f g h`-style chains, not a faithful K/Q
  valence resolution).
- Per this repo's own "grammar `{ }` repetition width is NOT bounded by the
  parser depth cap" lesson: `term`'s numeric stranding, `param_list`, and
  `stmt_seq` are all flat EBNF repetitions (bounded only by source length,
  not by `MAX_RULE_DEPTH`) — this parser emits them as flat sibling
  children in the CST (no recursion risk in *this* crate), but if
  `q-runtime` ever folds any of them into a recursively-walked or
  recursively-dropped tree shape, it needs its own separate width budget,
  the same way `MAX_RULE_DEPTH` bounds *nesting* depth here but never
  bounds repetition *width*.
- No bug was found in `q-lexer` while building this crate; it is consumed
  exactly as published.
