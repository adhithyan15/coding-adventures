# Changelog

All notable changes to the C++ `lisp-lexer` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `lisp-lexer` crate
  (namespace `ca::lisp_lexer`) — a hand-written Lisp tokenizer.
- `tokenize(source)` → `std::vector<Token>` ending with one `Eof` token; throws
  `LexerError` (a `std::runtime_error` carrying a byte `position`) on an
  unrecognised construct. Handles the 7 Lisp token types, skips whitespace and
  `;` comments, and resolves the `-42`-vs-`-` ambiguity by lookahead.
- `TokenType` enum with `token_type_name`, and `Token` with value equality.
- Byte-based scanning (position is a byte offset) — identical to the Rust
  code-point scanner on all ASCII input.
- 58 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
