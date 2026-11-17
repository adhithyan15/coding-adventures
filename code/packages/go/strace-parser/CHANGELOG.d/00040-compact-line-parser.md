## Unreleased

### Added

- `LineParser` (`compact.go`): a reusable compact path. `ParseLineInto` parses a
  line into a `parser.Tree` with no `*ASTNode` tree; `BodyKind`, `Body`,
  `RawArgs`, `AppendRawArgs`, `TextAfterClose`, `PrefixBeforeUnfinished`,
  `Tokens`, `Tree` and `Reset` are the helpers over NodeIDs. Not safe for
  concurrent use; results valid until the next call (returned strings are
  substrings of the line and outlive it).
- `compact_test.go`: equivalence with the legacy path (trees, tokens, errors and
  their text, every helper) over the corpora, fixtures, a random generator and
  `FuzzCompactEquivalence`, under seven option sets; reuse and allocation tests;
  legacy-vs-compact benchmarks.

### Changed

- `ParseLineTokens` is split into `checkLine` (everything before the parse),
  the parse and `classifyParseError`, shared by both paths. Behavior and error
  values are unchanged (the golden test passes unmodified).
