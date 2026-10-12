---
category: Testing & coverage
---

# Repeated corpus reads can turn snapshot assertions into platform timeouts

Run 38190146667 passed the repaired chapter and figure-target suites, then failed
six macOS cases across letter-ledger, levels, verbs and track-progress. The first
three files repeatedly parse the same immutable corpus or calculate the same
snapshot report. The progress file already shares its rendered cards, but its
two real CLI invocations independently load the entire corpus inside test bodies.

Reuse the existing scripts fixture, share the level corpus and summary, and share
the verb coverage report. Preserve every assertion and keep synthetic fixtures
independent. Give the two live CLI integration cases an explicit 120-second budget,
as with the modality CLI integration, while keeping their real invocations in the
tests. Ordinary limits remain 30 seconds, workers remain two, and validation uses
a 2 GiB heap cap. Do not introduce a production loader cache or conceal live CLI
work in unbounded module initialization.

The unchanged progress suite reproduced both failures on Windows with the normal
configuration and a 2 GiB heap: `--check` took 80.497 seconds and `--report` took
89.562 seconds, both exceeding 30 seconds. Its corpus fixture imported in 80.17
seconds; total runtime was 250.42 seconds. This directly measures why these two
filesystem integration cases need a larger explicit budget.

After merging main at acfa62d004, the four repaired files passed together under
the unchanged two-worker configuration and 2 GiB heap cap: 64 passed, one existing
Windows symlink-privilege skip, 286.80 seconds total. The live CLI calls took
60.187 and 88.250 seconds; level, ledger and verb assertions retained their normal
30-second budgets. TypeScript compilation and lesson validation also passed.
