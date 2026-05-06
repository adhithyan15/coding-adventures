# Changelog

All notable changes to the `state-machine-tokenizer` crate will be documented in this file.

## [0.1.0] - 2026-04-23

### Added

- Added the initial tokenizer-profile runtime over `EffectfulStateMachine`.
- Added fixed portable action interpretation for text buffering, start/end tag
  construction, EOF emission, diagnostics, and cursor/position tracing.
- Added a statically linked HTML skeleton tokenizer definition and fixture
  coverage for text, tags, chunked input, EOF flushing, diagnostics, and
  arbitrary Unicode text through `$any`.
- Expanded the portable lexer action vocabulary with comment, doctype,
  attribute, temporary-buffer, self-closing, and return-state actions so
  future HTML lexer definitions can stay declarative without host callbacks.
- Added portable DOCTYPE public/system identifier actions and token fields so
  declarative HTML lexer definitions can retain legacy declaration identifiers.
- Added a temporary-buffer conditional state-switch action for tokenizer states
  that need declarative branch decisions after scanning a keyword.
- Expanded the built-in HTML named-character-reference table through the
  classic Latin-1 entity set used by early web content.
- Added `commit_attribute_dedup` for lexer definitions that need HTML-style
  duplicate attribute recovery while keeping plain `commit_attribute`
  available for definitions that preserve duplicates.
- Added `append_attribute_value_replacement` so lexer definitions can recover
  invalid attribute-value code points with U+FFFD without host callbacks.
- Added `append_tag_name_replacement` and `append_attribute_name_replacement`
  so lexer definitions can recover invalid name code points with U+FFFD.
- Added `append_comment_replacement` so lexer definitions can recover invalid
  comment data code points with U+FFFD.
- Added DOCTYPE replacement actions for names, public identifiers, and system
  identifiers so lexer definitions can recover invalid DOCTYPE code points with
  U+FFFD.
- Numeric character-reference actions now report invalid-code-point diagnostics
  and apply HTML replacement/remapping behavior for null, surrogate,
  out-of-range, noncharacter, and Windows-1252 control references.
- Expanded the built-in HTML named-character-reference table with the remaining
  HTML4-era `alefsym` and `oline` math references.
- Added opt-in CRLF/bare-CR newline normalization for wrapper packages that
  need HTML input-stream preprocessing before transition matching.
- Restricted missing-semicolon named-character-reference recovery to the
  WHATWG legacy no-semicolon aliases instead of accepting every known name
  without `;`.
- Added current-comment seeding helpers so wrapper packages can resume
  comment-token continuation states without loading tokenizer definition files.
- Added `DoctypeSeed` and current-DOCTYPE seeding helpers so wrapper packages
  can resume DOCTYPE continuation states while preserving partial name,
  identifier, and force-quirks data.
