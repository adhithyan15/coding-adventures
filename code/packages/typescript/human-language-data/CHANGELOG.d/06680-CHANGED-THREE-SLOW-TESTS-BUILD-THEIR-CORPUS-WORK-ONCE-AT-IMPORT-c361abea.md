### Changed — three slow tests build their corpus work once, at import

A full-suite run on the Malayalam A1 corpus had three test files with default-budget tests at about half the 30s per-test budget. That margin shrinks with every content PR:

- `tests/spanish-a1-mock-audit.test.ts`: each level-comparison test built two whole-corpus audits, about 15s. The three audits (pre-A1, A1 and A2) are now built once at import and shared by all five tests that read one.
- `tests/track-progress.test.ts`: two tests each rendered every progress card, the slower at about 16s. The cards are now rendered once at import.
- `tests/cli.test.ts`: `runValidate` validated the whole corpus inside its test, about 15s. It now runs once at import, with stdout captured, and the test checks what it returned and printed.

No assertion or timeout changes. The `--check` tests still run their CLIs end to end.
