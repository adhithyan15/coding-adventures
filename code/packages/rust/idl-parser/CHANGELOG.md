# Changelog

## [0.1.0] - 2026-07-23

### Added

- Initial grammar-driven Rust IDL parser (MA12 §6, task MA-12c).
- `code/grammars/idl/idl.grammar` (47 rules), written to IDL's own
  Algol/Fortran-family imperative shape (MA12 §5) — statements,
  `PRO`/`FUNCTION` definitions, `IF`/`FOR`/`WHILE`/`REPEAT` blocks, an infix
  operator-precedence cascade with word operators — rather than forked from
  any array-family grammar (contrast `scilab.grammar`'s fork of
  `matlab.grammar`).
- The two genuinely new disambiguations MA12 §3 fixed as design problems,
  now resolved entirely through grammar structure (no lookahead predicate of
  any kind):
  1. `/BOOLEAN` keyword shorthand vs. division — `arg = keyword_arg |
     bool_keyword_arg | expr ;` with `bool_keyword_arg = SLASH NAME`. Safe
     because `expr`'s own cascade never lets `SLASH` appear except as a
     binary operator consumed after a left operand (IDL has no unary `/`),
     so a leading `SLASH` at an argument position can only ever be the
     boolean shorthand.
  2. `=` as assignment vs. keyword-bind — `assignment_stmt = NAME
     [ index_suffix ] EQUALS expr` (statement-level, reachable only from
     `statement`) vs. `keyword_arg = NAME EQUALS expr` (reachable only from
     `arg`, inside an argument list). Identical token shape, disambiguated
     purely by which production the parser is inside.
  Both productions (`arg_list`/`arg`) are shared, unmodified, between
  `procedure_call_stmt`'s command-style comma list and a function call's
  parenthesised `call_suffix`, so keyword arguments and the `/BOOLEAN`
  shorthand behave identically in both call styles (MA12 §3 item 2).
- `procedure_call_stmt = NAME COMMA arg_list` — the headline new production,
  disambiguated from `assignment_stmt`/`expr_stmt` by ordinary PEG ordered
  choice (COMMA is not used as a statement separator anywhere else in this
  cut's in-scope surface; IDL's own separator is `&`). A disclosed,
  spec-consistent scope note: a zero-argument call is syntactically
  identical, at the CST level, to a bare-variable-reference expression
  statement (both are a lone NAME with no comma) — MA12 §3 itself frames the
  call-statement production as requiring the comma+arg-list shape, so no
  synthetic zero-arg alternative was added; resolving "call or read" for a
  bare NAME is left to a future `idl-runtime`'s symbol table (MA-12d),
  mirroring MA12 §1's own pre-5.0 `fish(5)` finding.
- Full control-flow surface: `IF...THEN...ELSE` (single-statement and
  `BEGIN...ENDIF/ENDELSE/END` block forms), `FOR v=lo,hi[,step] DO` (`DO` in
  the identical relative position to `WHILE`'s own, confirmed by direct
  comparison, not assumed), `WHILE expr DO`, `REPEAT...UNTIL` (with the
  block form's `ENDREP` confirmed, against real IDL documentation, to
  precede `UNTIL` rather than follow it — a regression test
  (`repeat_until_reversed_order_is_a_syntax_error`) proves the reversed
  order is rejected), `BREAK`/`CONTINUE`, a generic `BEGIN...END` block, and
  `RETURN`/`RETURN, expr` (both forms accepted everywhere by this CST-only
  grammar; which is semantically valid in a given routine kind is deferred
  to MA-12d).
- `pro_def`/`func_def` — both terminated by the generic `END` (no
  `ENDPRO`/`ENDFUNCTION` exists in `idl.tokens`' keyword list). Parameter
  lists mix plain positional names with `KEYWORD=local_var` keyword-
  parameter declarations (MA12 §4's own literal spelling) — the RHS of a
  parameter declaration's `=` is always a bare NAME (the internal binding
  variable), never an arbitrary expression, since real IDL default values
  are assigned inside the body, not declared in the header.
- The full expression precedence cascade, verified against the official
  NV5/L3Harris *Operator Precedence* reference (and a verbatim IRYA/UNAM
  mirror of the same table) rather than assumed from Scilab's/MATLAB's own
  cascade shape. Two confirmed, non-obvious divergences:
  1. Unary `+`/`-`/`NOT` sit at the SAME documented tier as binary `+`/`-`,
     not tighter than `multiplicative` the way Scilab/MATLAB's own `unary`
     sits — `unary` here recurses *above* `multiplicative`, so `-a*b`
     parses as `-(a*b)`.
  2. `^` is LEFT-associative in IDL (`2^3^2` = `(2^3)^2` = 64), not
     right-associative like Scilab/MATLAB/Python's own `^`/`**` — `power`
     uses left-recursive `{ }` repetition, not the right-recursive
     `[ CARET power ]` shape a right-associative operator would need.
  `NOT` (unary, tier 5) and `AND`/`OR`/`XOR` (binary, tier 7) sit at
  different precedence tiers despite `idl.tokens`' own lexer-layer comment
  grouping all four under one descriptive "logical/bitwise" heading.
  Assignment (`=`) is deliberately NOT part of the expression cascade at
  all — unlike Scilab's/MATLAB's chainable `assignment = logical_or
  [ EQ assignment ]`, real IDL has no assignment-as-expression.
- Both matrix-product operators `#`/`##` at the same precedence tier as
  `*`/`/` (confirmed against the official table's own tier grouping, not
  assumed to be a separate tier).
- The full subscript surface (MA12 §4): plain (`a[i]`), 2-D (`a[i,j]`),
  ranged (`a[s0:s1]`), strided (`a[s0:s1:n]`), `*`-wildcard (`a[*]`,
  `a[s0:*]`, `a[s0:*:n]`), and negative-from-end (`a[-1]`, handled for free
  by the ordinary `unary` MINUS — no dedicated production needed). Array
  literals (`[1, 2, 3]`) as a `primary` alternative, structurally distinct
  from `index_suffix`'s postfix-position `LBRACKET`.
- A deliberate, disclosed scope boundary: `CONTINUATION` (`$`) is not
  referenced anywhere in `idl.grammar` — `idl-lexer` emits it unconditionally
  and does not suppress the following NEWLINE, and MA12 §5 assigns
  continuation-tracking to `idl-repl`'s own raw-text-level scanner (MA-12d),
  not to this parser. `grammar-tools cross_validate` reports the resulting
  "Token 'CONTINUATION' defined but never used" warning — expected and
  documented, not a bug.
- A bespoke `MAX_RULE_DEPTH = 148` recursion-depth cap — the FIRST IDL crate
  with actual recursive descent (`idl-lexer` only tokenizes; no cap existed
  before this crate). Six structurally distinct self-referential shapes were
  measured independently (binary search, uncapped parser, default-stack
  worker thread, debug build, one fresh subprocess per data point):
  parenthesised nesting (27 safe / 28 crash nesting-count, 291/292
  rule-frame), nested `IF`/`ENDIF` (47/48 nesting, 249/250 rule-frame —
  `FOR`/`WHILE`/`REPEAT`/generic `BEGIN` share the identical
  `statement -> ... -> block_body -> statement_line -> statement`
  reachability, confirmed by rule-graph inspection, so not separately
  measured), nested function-call arguments (22/23 nesting, 266/267
  rule-frame), nested subscript indexing (21/22 nesting, 273/274
  rule-frame — measured independently despite an apparently-identical
  three-wrapper-frame shape to call-argument nesting, to confirm the
  rule-graph symmetry actually holds at the native-stack level), a unary
  prefix chain (199/200 nesting, 212/213 rule-frame), and nested array
  literals (24/25 nesting, 282/283 rule-frame). Every "flat chain of one
  operator" production written with EBNF `{ x }` repetition costs zero
  native stack regardless of width, confirmed by reading
  `parser::grammar_parser`'s own `Repetition`/`SeparatedRepetition`
  implementation directly.
- The genuine surprise (mirroring `scilab-parser`'s own): the unary prefix
  chain tolerates by far the *most* nesting levels (199) of any measured
  shape, yet has the *lowest* rule-frame floor (212) — its persisting
  per-level cost is exactly one rule-frame (`unary` itself, confirmed by the
  near-1:1 nesting-to-frame ratio, 212/199 ≈ 1.07), cheap enough per level to
  reach 199 levels, yet its own call path costs more native-stack bytes per
  crossing than the other shapes' higher per-level rule-frame counts would
  suggest. `148` sits about 30.2% below the binding 212 floor (comparable to
  `reduce-parser`'s own ~28.5%, `apl-parser`'s ~26.5%, `j-parser`'s ~30%,
  `derive-parser`'s ~33%, `maple-parser`'s ~31.2%, `scilab-parser`'s
  ~30.2%), and therefore safely below all five other rule-frame floors (291,
  249, 266, 273, 282) too. Full measurement tables and reasoning in
  `MAX_RULE_DEPTH`'s own doc comment (`src/lib.rs`).
- 59 tests + 1 doctest covering: every statement/control-flow production
  (one test per construct, both body forms where applicable); the two
  headline disambiguations in both directions (`/BOOLEAN` shorthand vs.
  division in an assignment RHS vs. division as a positional call argument;
  `=` as keyword-bind vs. assignment); the procedure-call-statement's
  zero-arg scope note; `REPEAT`'s `ENDREP`-before-`UNTIL` order (plus a
  regression proving the reversed order is rejected); every subscript form;
  `PRO`/`FUNCTION` definitions (positional and keyword parameters) and
  `RETURN`'s two forms; the full precedence cascade (one test per tier
  boundary, including the two confirmed divergences — unary binding looser
  than multiplicative, and left-associative `^`); both matrix-product
  operators; array literals (including immediate re-subscripting); and 4
  depth-guard tests exercising all six measured shapes at once.
- `code/grammars/idl/idl.grammar` validated with
  `grammar_tools::parser_grammar::validate_parser_grammar` (47 rules, clean)
  and cross-validated against `code/grammars/idl/idl.tokens` with
  `grammar_tools::cross_validator::cross_validate` (one expected, documented
  warning: `CONTINUATION` defined but unused — see above).
- Registered `idl-parser` in `code/packages/rust/Cargo.toml`'s workspace
  `members`.

### Not touched

- `idl-lexer` (MA-12b, already merged) — `code/grammars/idl/idl.tokens` and
  `code/packages/rust/idl-lexer/` are untouched; this crate consumes
  `idl-lexer`'s public `tokenize_idl`/`try_tokenize_idl` API as-is.
- `idl-runtime`/`idl-repl` (MA-12d) and `idl-to-semantic-ir` (MA-12e) — not
  started; out of scope for this item (MA-12c is parser-only).
