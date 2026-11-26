# Changelog

## Unreleased

- Add a standalone Go test harness that uses an independently encoded
  positional CNF and the native SAT solver to check shortest-path behavior over
  all 17,832 bounded graph/layout/ordered-endpoint cases. SAT models are
  replayed against the source topology, and deliberate higher-cost claims serve
  as witness-producing negative controls. The published graph module gains no
  SAT dependency.
- Add a native bounded fuzz target for query-byte mapping, strict cost bounds,
  SAT status, and model replay; Unix and Windows BUILD fronts run a five-second
  single-worker fuzz smoke.
