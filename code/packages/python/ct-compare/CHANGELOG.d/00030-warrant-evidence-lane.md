## Unreleased

- First non-Rust Warrant evidence lane (VER04i, WD-101). The package now runs
  the language-neutral `warrant-corpus-v1` corpus that the Rust compiler emits
  from `code/specs/ct-compare.warrant`, through `evidence.bindings`, with the
  shared `warrant-corpus` runner. No library source changed.
- `evidence/test-manifest.json` is the lane manifest written by a real run:
  12 examples, 9 exhaustive suites (572,109 enumerated cases, domains
  `Bytes[347]`, `Bytes<N>`, `U64[377]`), and 8 negative controls, each caught
  by a required claim. Exhaustive means exhaustive over those finite domains,
  not "all inputs".
- `BUILD` regenerates the manifest in memory and fails if the committed file
  drifted (`check`), proves each negative control makes the runner print
  `not ok` and exit non-zero (`sabotage`), and validates the manifest's
  structure against the corpus and the consumer registry.
- `tests/warrant_evidence.py` is the registered `runner_entry`; the consumer
  registry row stays `planned` until the report-side loader (WD-004b1) exists.
- Dev dependency on the (test-only) `coding-adventures-warrant-corpus` runner.
