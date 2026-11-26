# Changelog

## Unreleased

- Feature: `GrammarParserOptions.maxDepth` recursion guard, mirroring
  the reference opt-in cap. The check sits outside the memo lookup
  (memoized hits count too); refusal sets a sticky flag so `parse()`
  reports `input nests deeper than the supported limit (N)` instead
  of a generic syntax error. `null` (the default) leaves the parser
  uncapped, exactly like the reference.

## 0.1.0

- Initial release.
- Added grammar-driven parsing support with generic `ASTNode` helpers.
