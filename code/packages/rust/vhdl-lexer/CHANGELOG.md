# Changelog

All notable changes to the `coding-adventures-vhdl-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `create_vhdl_lexer(source)` — factory function that loads `vhdl.tokens` and returns a configured `GrammarLexer`.
- `tokenize_vhdl(source)` — convenience function that tokenizes VHDL source, applies case normalization (lowercasing NAME and KEYWORD token values), and returns `Vec<Token>`.
- Case-insensitive identifier support: all NAME and KEYWORD token values are lowercased after tokenization, matching VHDL's case-insensitive semantics.
- Loads grammar from `vhdl.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering entity declarations, architecture bodies, signal declarations, case insensitivity normalization, character literals, bit string literals, based literals, all operators (:=, <=, =>, /=, **, <>), keyword operators (and, or, xor, not, mod, rem), comments, strings, complete VHDL snippets (half adder, D flip-flop), extended identifiers, real numbers, and delimiters.
