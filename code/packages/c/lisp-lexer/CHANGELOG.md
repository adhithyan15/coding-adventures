# Changelog

All notable changes to the C `lisp-lexer` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `lisp-lexer` crate — a hand-written
  Lisp tokenizer.
- `ll_tokenize(source, out, err)`: scans source into an owned `LlTokenList`
  ending with one `LL_EOF` token, or fails closed with an `LlError`. Handles the
  7 Lisp token types (number incl. negative, symbol incl. operator characters,
  quote-preserving string with `\` escapes, `(` `)` `'` `.`), skips whitespace
  and `;` comments, and resolves the `-42`-vs-`-` ambiguity by lookahead.
- `ll_token_type_name` (uppercase names) and `ll_token_list_free`.
- Byte-based scanning (position is a byte offset) — identical to the Rust
  code-point scanner on all ASCII input.
- 63 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
