# Changelog

## 0.17.0 — `INSPECT` statement (first rung)

- Added `inspect_stmt` to the `statement` alternation:
  `inspect_stmt = "INSPECT" operand ( inspect_tallying [ inspect_replacing ] |
  inspect_replacing ) [ "END-INSPECT" ]`, with the supporting rules
  `inspect_tallying = "TALLYING" tally_for { tally_for }`,
  `tally_for = NAME "FOR" tally_item { tally_item }`,
  `tally_item = ( "ALL" | "LEADING" ) operand { inspect_region } | "CHARACTERS"
  { inspect_region }`,
  `inspect_replacing = "REPLACING" replace_item { replace_item }`,
  `replace_item = "CHARACTERS" "BY" operand { inspect_region } | ( "ALL" |
  "LEADING" ) operand "BY" operand { inspect_region }`, and
  `inspect_region = ( "BEFORE" | "AFTER" ) operand`.
- The source is the first (and only top-level) `operand`; the counter is the
  `NAME` before `FOR`; the delimiter is the `operand` under the matched
  `tally_item`. As with `string_stmt`/`unstring_stmt`, the grammar deliberately
  *accepts* the fuller surface (`LEADING`/`CHARACTERS` tallies, `BEFORE`/`AFTER`
  regions, several counters or `FOR` phrases, and every `REPLACING` form) so the
  reader/compiler reject them with a friendly "later rung" error rather than a
  bare parse failure. Uses the new `INSPECT` keywords (see `cobol-lexer` 0.9.0).
  `_grammar.rs` regenerated via `grammar-tools compile-grammar`.

## 0.16.0 — `UNSTRING` statement (first rung)

- Added `unstring_stmt` to the `statement` alternation:
  `unstring_stmt = "UNSTRING" operand "DELIMITED" "BY" operand "INTO" NAME { NAME }
  [ "WITH" "POINTER" NAME ] [ "ON" "OVERFLOW" { statement } ]
  [ "NOT" "ON" "OVERFLOW" { statement } ] [ "END-UNSTRING" ]`. The source and the
  delimiter are the two `operand` children (in order); the receivers are the `NAME`
  tokens after `INTO`. As with `string_stmt`, the grammar deliberately *accepts*
  the later-rung options (`WITH POINTER`, `ON`/`NOT ON OVERFLOW`) so the
  reader/compiler can reject them with a friendly "later rung" error rather than a
  bare parse failure. Uses the new `UNSTRING` / `END-UNSTRING` keywords (see
  `cobol-lexer` 0.8.0). `_grammar.rs` regenerated via
  `grammar-tools compile-grammar`.

## 0.15.0 — `STRING` statement (first rung)

- Added `string_stmt` to the `statement` alternation:
  `string_stmt = "STRING" operand { operand } "DELIMITED" "BY" string_delim
  "INTO" NAME [ "WITH" "POINTER" NAME ] [ "ON" "OVERFLOW" { statement } ]
  [ "NOT" "ON" "OVERFLOW" { statement } ] [ "END-STRING" ]` with
  `string_delim = "SIZE" | operand`. The sending fields are `operand` children;
  the delimiter operand is nested under `string_delim`, so it does not collide.
  The grammar deliberately *accepts* the later-rung options (a real
  identifier/literal delimiter, `WITH POINTER`, `ON`/`NOT ON OVERFLOW`) so the
  reader can reject them with a friendly "later rung" error rather than a bare
  parse failure. Uses the new `STRING`/`DELIMITED`/`WITH`/`POINTER`/`OVERFLOW`/
  `END-STRING` keywords (see `cobol-lexer` 0.7.0). `_grammar.rs` regenerated via
  `grammar-tools compile-grammar`.

## 0.14.0 — reference-modification suffix on an operand

- The `operand` rule gains an optional reference-modification suffix:
  `operand = NAME [ LPAREN operand COLON [ operand ] RPAREN ] | literal ;`.
  A `NAME` may now be followed by `(start:len)` or `(start:)` (omitted length),
  selecting a substring of an alphanumeric item. A bare `NAME` still parses
  exactly as before. The inner start/length are themselves `operand`s (so an
  integer NUMBER literal parses); the readers reject non-literal start/length as
  a later rung. Uses the new `COLON` token (see `cobol-lexer` 0.6.0).
  `_grammar.rs` regenerated via `grammar-tools compile-grammar`.

## 0.13.0 — `EVALUATE` multiple values and `THRU` ranges per `WHEN`

- `when_branch` now takes a value-*list*: `"WHEN" ( "OTHER" | when_value
  { when_value } )` with `when_value = operand [ ( "THRU" | "THROUGH" ) operand ]`.
  So a `WHEN` may list several values and inclusive ranges (`WHEN 1 5 THRU 7 9`).
  A `when_value` stops at the next `WHEN`/`END-EVALUATE` or a statement verb
  (all keywords, so a value operand can't consume them). `THRU`/`THROUGH` were
  already reserved. `_grammar.rs` regenerated via `grammar-tools compile-grammar`.

## 0.12.0 — `EVALUATE` (case statement)

- Added `evaluate_stmt = "EVALUATE" operand { when_branch } "END-EVALUATE"` and
  `when_branch = "WHEN" ( "OTHER" | operand ) { statement }` to the `statement`
  alternation. A WHEN's statement list ends at the next `WHEN` or `END-EVALUATE`
  (both keywords, so a statement can't consume them). Uses the new
  `EVALUATE`/`OTHER`/`END-EVALUATE` keywords (see `cobol-lexer` 0.5.0). This first
  cut is the simple form: one value per `WHEN`. `_grammar.rs` regenerated via
  `grammar-tools compile-grammar`.

## 0.11.0 — `NOT` over a condition

- The condition cascade gains a `negation` layer between `conjunction` and
  `simple_condition`: `conjunction = negation { "AND" negation }`,
  `negation = [ "NOT" ] simple_condition`. `NOT` binds tighter than `AND`/`OR` and
  negates the following relation, condition-name, or parenthesised group. It does
  not collide with a relation's own `IS NOT …`: the negation `NOT` precedes the
  first operand, the relop `NOT` sits between the operands. `NOT` was already a
  keyword, so no lexer change. `_grammar.rs` regenerated via `grammar-tools
  compile-grammar`.

## 0.10.0 — compound conditions (`AND` / `OR` / parentheses)

- `condition` is now a precedence cascade: `disjunction` of `AND`-joined
  `simple_condition`s, where a `simple_condition` is a relation, a level-88
  condition-name, or a parenthesised `condition`. `AND` binds tighter than `OR`
  (`A OR B AND C` = `A OR (B AND C)`); parentheses group. `AND`/`OR`/`NOT` and
  `(`/`)` were already tokens, so no lexer change. Applies to `IF` and both
  `PERFORM … UNTIL` forms (all reference `condition`). `_grammar.rs` regenerated
  via `grammar-tools compile-grammar`.

## 0.9.0 — symbolic relational operators

- The `relop` rule accepts the symbols `>` `<` `=` `>=` `<=` `<>` alongside the
  word forms (`GREATER THAN`, …). `>=`/`<=`/`<>` already encode a negation
  (`>=` ≡ `NOT <`); a leading `NOT` composes with it (the interpreter/compiler
  readers XOR the two). Uses the new `GT`/`LT`/`GE`/`LE`/`NE` tokens (see
  `cobol-lexer` 0.4.0); `EQ` was already present. `_grammar.rs` regenerated via
  `grammar-tools compile-grammar`.

## 0.8.0 — `SET cond-name TO TRUE`

- Added `set_stmt = "SET" NAME "TO" "TRUE"` to the `statement` alternation — the
  verb that assigns a level-88 condition-name (sets its conditional variable to
  the value that makes it hold). `SET`/`TRUE` are new keywords (see `cobol-lexer`
  0.3.0); `TO` was already reserved. `_grammar.rs` regenerated via `grammar-tools
  compile-grammar`.

## 0.7.0 — `VALUE` clause: multiple values and `THRU` ranges

- `value_clause = "VALUE" [ "IS" ] value_item { value_item }` with
  `value_item = literal [ ( "THRU" | "THROUGH" ) literal ]`. A plain item still
  parses a single literal; a level-88 condition-name may now list several values
  and inclusive ranges (`88 OK VALUE 1 5 THRU 7 9`). `THRU`/`THROUGH` were already
  reserved words, so no lexer change. The grammar is permissive — the interpreter
  and compiler reject a multi-value/range `VALUE` on a non-88 item.
- `_grammar.rs` regenerated from `cobol.grammar` via `grammar-tools compile-grammar`.

## 0.6.0 — level-88 condition-names in `IF` / `PERFORM UNTIL`

- `condition` becomes an ordered choice `relation | condition_name`, where
  `relation = operand relop operand` (the former body) and `condition_name = NAME`
  (a bare level-88 condition-name, e.g. `IF IS-OK`). The relation is tried first;
  a bare condition-name has no relop after its NAME, so the parser cleanly falls
  back to `condition_name`. No lexer change — a condition-name is an ordinary NAME.
- Level-88 data entries already parse as a `data_entry` (`88` is a NUMBER, `VALUE`
  is a `value_clause`), so no data-division grammar change was needed — only the
  *reference* site (`condition`) grew the new alternative.
- `_grammar.rs` regenerated from `cobol.grammar` via `grammar-tools compile-grammar`.

## 0.5.0 — ROUNDED / ON SIZE ERROR on the arithmetic verbs

- `add_stmt` / `subtract_stmt` / `multiply_stmt` / `divide_stmt` gain the trailing
  `[ ROUNDED ] [ size_error ]` clauses, sharing the `size_error` rule already used
  by `COMPUTE`. `ROUNDED`/`ON`/`SIZE`/`ERROR` were already reserved words, so the
  lexer is unchanged.

## 0.4.0 — PERFORM … VARYING

- `perform_stmt` gains a third repeat clause, `perform_varying`
  (`VARYING NAME FROM operand BY operand UNTIL condition`), alongside
  `operand TIMES` and `UNTIL condition`. `VARYING`/`FROM`/`BY` were already
  reserved words, so the lexer is unchanged.

## 0.3.0 — PERFORM … UNTIL

- `perform_stmt` gains the `UNTIL condition` clause as an alternative to
  `operand TIMES`: `PERFORM para [THRU …] [ operand TIMES | UNTIL condition ]`.
  Reuses the existing `condition` rule. `UNTIL` was already a reserved word, so
  the lexer is unchanged.

## 0.2.0 — COMPUTE and arithmetic expressions

- `compute_stmt = "COMPUTE" NAME [ "ROUNDED" ] EQ arith_expr [ size_error ]` —
  the first COBOL verb that takes operator symbols instead of prepositions.
- **Precedence-layered arithmetic expressions.** A PEG cannot left-recurse, so
  COBOL's operator precedence is encoded as a rule cascade, loosest binding
  first: `arith_expr` (`+ -`) → `arith_term` (`* /`) → `arith_factor` (`**`) →
  `arith_unary` (leading `+`/`-`, binding tighter than `**` so `-2 ** 2` reads
  as `(-2) ** 2`) → `arith_primary` (`NUMBER | NAME | ( arith_expr )`).
  Parenthesised sub-expressions recurse through `arith_primary`; deep nesting is
  bounded by the recursion-depth cap added in 0.1.1.
- `size_error = "ON" "SIZE" "ERROR" { statement }` recognises the overflow
  handler (its runtime semantics are a later PR). Exponentiation's
  right-associativity is likewise left to the evaluator — the grammar keeps
  operands as flat siblings.
- Tests: operator-precedence nesting, parentheses regrouping, `ROUNDED` +
  `ON SIZE ERROR`, and spaced binary minus vs. negative literal.

## 0.1.1 — depth-cap hardening

- **Security (DoS):** opt into the shared parser's recursion-depth cap
  (`DEFAULT_MAX_RULE_DEPTH`) in both `create_cobol_parser` and `try_parse_cobol`.
  Deeply-nested syntax (e.g. thousands of nested `IF … IF … IF …`) recurses once
  per level through the generic `parse_rule`; without the cap it overflowed the
  *native* stack — an uncatchable `SIGSEGV`/abort that a `Result`-returning entry
  point cannot report. It now surfaces as a recoverable "input nests deeper than
  the supported limit" parse error. Regression test added.

## 0.1.0 — COBOL-60 parser (PL07)

- Grammar-driven parser over `code/grammars/cobol/cobol.grammar`, wrapping
  `parser::GrammarParser`. Public API: `parse_cobol` / `try_parse_cobol` /
  `create_cobol_parser`. CST rooted at `"program"`.
- Grammar covers the demonstrated language: the four divisions (IDENTIFICATION
  and PROCEDURE required; ENVIRONMENT and DATA optional); IDENTIFICATION with
  `PROGRAM-ID` and commentary paragraphs; a minimal ENVIRONMENT (CONFIGURATION
  and INPUT-OUTPUT sections); DATA `WORKING-STORAGE`/`FILE` entries with level
  numbers, `PICTURE`, and `VALUE`; and PROCEDURE paragraphs of sentences over the
  core verbs (`MOVE`, `DISPLAY`, `ACCEPT`, `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE …
  GIVING`, `PERFORM`, `GO TO`, `IF … ELSE`, `STOP RUN`).
- Data entries parse as `NUMBER (NAME | FILLER) { clause } DOT` — the leading
  NUMBER is the level (the lexer keeps no LEVEL token). Sentences and paragraph
  names never collide (verb KEYWORD vs NAME).
- Tests parse the full carded four-division program end to end, plus each
  division and statement kind in isolation.
