---
category: Testing & coverage
---

# Whole-corpus exhaustive runs are 10x slower under coverage tracing, so test plumbing on a consistent slice and run the full corpus once per session

The first draft of the `warrant-corpus` tests ran the 572,109-case ct-compare corpus in nine
different tests. Plain it takes about 6 seconds; under `pytest --cov` the same hot loop took
about 25, and the suite took 3 minutes 40 seconds.

Fix: one session-scoped fixture runs the real, Rust-emitted corpus exactly once and every test that
needs "the whole corpus passed" shares it (so the digest agreement with the Rust compiler is
asserted once, on real data). All command-line, manifest and failure-classification tests use a
consistent two-suite slice of that corpus (512 cases; the controls and TFV are filtered to match).
The suite now takes about 20 seconds with 99.9% coverage. Build the slice by transforming a copy of
the real corpus rather than hand-writing one, so it cannot drift from the format.
