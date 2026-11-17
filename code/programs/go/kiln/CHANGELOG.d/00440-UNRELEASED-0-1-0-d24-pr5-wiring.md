## [0.1.0] - UNRELEASED

### D24 PR 5: the grammar path, side by side (KILN04 section 4.9)

- `go.mod` requires and replaces the in-repo modules `strace-lexer`,
  `strace-parser`, `lexer`, `parser`, `grammar-tools` (and their two
  dependencies). `TestNoThirdPartyDependencies` enforces KILN00 section 17:
  only repo modules in `require`, only relative in-repo `replace` targets, and
  `go list -deps` holds nothing but the standard library and repo packages.
- `internal/trace`: `handleLineG` reads a line with the grammar and builds the
  same records as `handleLine`, with explicit limits (64 bracket levels, 65,536
  tokens, a bounded memo, strict quoted strings, the strict result parser on
  every closed call). The old scanner stays the default.
- `crossCheck` (set by the package's tests, and by `KILN_TRACE_DIFF=1`): every
  `Parse` also runs the other reader and panics on a disagreement outside the
  spec's stricter-only table. New accepting where the old refused is always a
  failure. Every existing test is now a differential.
- A seeded line differential, `FuzzLineDiff`, `FuzzParseDiff` and a small
  committed seed corpus; assertions that the strace grammars are case-sensitive
  and that the lexer has no regexp-engine rules.
- The kiln-sandbox job sets `KILN_TRACE_DIFF=1` and logs `PROBE: trace` lines
  with real trace sizes and the throughput of both readers (`trace.Measure`).
- New table rows (KILN04 section 4.9): a glued split call over the byte limit,
  an annotation after the pid-namespace note, a Unicode space in an exit line,
  a malformed resumed half with no pending call, arity then a later error.
