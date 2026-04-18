# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Pure F# two-phase CommonMark parser built on the F# document AST package
- xUnit coverage for headings, paragraphs, lists, blockquotes, code fences, links, inline formatting, and entities
- Parser guardrails that reject oversized markdown inputs and excessively deep nesting before recursive parsing can exhaust the stack
