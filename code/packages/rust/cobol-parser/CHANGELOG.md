# Changelog

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
