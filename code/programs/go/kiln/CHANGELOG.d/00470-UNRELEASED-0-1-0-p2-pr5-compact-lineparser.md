## [0.1.0] - UNRELEASED

### Changed (D24, P2 PR 5)

- `handleLineG`, the grammar path of the trace reader, now parses each line with
  one reused `straceparser.LineParser` (owned by the per-trace `lineState`)
  instead of building an `*ASTNode` tree: arguments are read with
  `AppendRawArgs` into a reused slice, and the shape, the text after the
  closing parenthesis and the pending prefix come from the compact helpers.
  Behaviour is identical (same records, refusals and reasons); the old
  hand-written scanner is still the default and `crossCheck` is unchanged.
  On the fixture traces the grammar path goes from 13.6 to 17.9 MB/s and from
  1693 to 421 allocations per corpus pass.
- `trace.Measure` also reports each pass's heap allocations
  (`OldAllocs`, `NewAllocs`), and the sandbox `PROBE:` line logs them.

### Added (tests)

- `grammar_legacy_test.go`: the ASTNode `handleLineG`, frozen as a test-only
  reference (`handleLineGLegacy`).
- `compact_equiv_test.go`: the compact path equals the legacy path on the
  fixtures, 300k seeded lines (some under a small line limit), 600k seeded
  sequences and `FuzzCompactEqualsLegacy`; the all-space (including U+00A0)
  argument guard is pinned against the old scanner and both grammar paths; and
  `BenchmarkLineHandler` / `BenchmarkPass1` compare old scanner, grammar on
  ASTNode and grammar on the compact path.
