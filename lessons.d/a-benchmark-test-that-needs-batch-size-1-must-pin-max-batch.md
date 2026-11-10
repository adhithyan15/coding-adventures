---
category: Testing & coverage
---

# A benchmark test that needs batch size 1 must pin max_batch_size, because a coarse clock can time one tiny iteration as 0 ns

test-bootstrap's `instruction_budget_rules_for_live_not_live_and_unsupported`
(in #19099's CI run) passed on Linux and macOS but failed on Windows. It set
`min_batch_duration: 1 ns` and assumed that a single `1 + 1` iteration meets
that, so the batch size stays 1 and a scripted `Live(101)` counter means 101
instructions per iteration. Windows' `Instant` can read 0 ns for such a body.
Calibration then doubled the batch, and 101 / 2 rounded up to 51, under the
100 limit: the expected `Failed` became a `Pass`. The test's own first
assertion (`batch_size == 1`) passed only by luck on that run.

Do this: when a test's arithmetic depends on the batch size, set it directly
(`max_batch_size: 1`), not through a timing threshold. When the test is
about calibration itself, make the body take at least one clock tick, by
spinning until `Instant::elapsed()` is non-zero, instead of assuming any
body takes more than 0 ns. Any "one iteration exceeds N ns" assumption is
false somewhere.
