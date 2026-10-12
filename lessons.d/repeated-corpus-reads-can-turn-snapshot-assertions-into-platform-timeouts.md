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
