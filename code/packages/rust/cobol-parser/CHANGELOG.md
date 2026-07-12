# Changelog

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
