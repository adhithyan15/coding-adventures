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
- Expanded the built-in HTML named-character-reference table through the
  classic Latin-1 entity set used by early web content.
