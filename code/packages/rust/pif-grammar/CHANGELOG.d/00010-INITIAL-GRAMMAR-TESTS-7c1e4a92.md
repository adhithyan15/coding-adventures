## [0.1.0] - 2026-11-24

### Added

- New crate `pif-grammar` (VER32 PIF-001): embeds `code/grammars/pif/pif.tokens` and `pif.grammar` and exposes `lex`, `parse` and `kind` over the repository's `GrammarLexer` and `GrammarParser`.
- 26 tests: the grammar files validate; the VER32 section 4.6 worked example parses and its fixture is compared with the spec text (drift fails the test); one minimal file per record kind; every lexical rule of VER32 4.1 (single spaces, byte-exact brackets, LF and printable ASCII only, canonical numerals, string escapes, digests); the `\u{0}`-only-in-`estr` rule; the four binder words never being identifiers (the keyword/`FIDENT` tie); the `end` trailer; header and record shapes; and a pinned list of reader-only rules (VER32 5.3) that the grammar deliberately accepts.
