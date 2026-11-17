## Unreleased - cigates path-sharded test tracks the match-work ceiling

- `TestPathShardedLanguageDiffsStaySelective/10751-paths` failed with `CI_GATE_MATCH_LIMIT_EXCEEDED` on main. Root cause: the fixed 50,000,000-unit ceiling (normative, shared by every build-tool implementation and the conformance fixtures) is charged per distinct pattern x changed-file pair, and the registry grew to 138 distinct patterns (about 52.2M units at 10,751 paths). The ceiling is correct and unchanged; the test's expectation was stale.
- The strictly-selective large case is now 8,192 paths, and a new `TestHugePathShardedDiffIsSelectiveOrBoundedFailure` asserts that 10,751 paths is either selective or fails with the stable limit error before any matcher call (the case CI answers with a run-all fallback).
