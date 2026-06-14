# Changelog

## 0.2.1 — 2026-06-13 (LANG-FULL N1)

- `src/_grammar.rs` regenerated after `nib.tokens` doc updates (the `STAR`/`SLASH`
  tokens are now multiplicative operators, not reserved-for-v2). Token definitions
  are unchanged; only embedded `line_number` metadata shifted.

## 0.2.0 — 2026-05-20 (NIB04 step 3)

- `while` added to the keyword set so `while_stmt` parses as a
  control-flow statement rather than an identifier.

## 0.1.0

- Initial Rust port of the Nib lexer with compiled grammar support.
