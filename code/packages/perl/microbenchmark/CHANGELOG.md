# Changelog

## 0.1.0

- Add injectable paired collection, strict report replay, and exact analysis.
- Add the indexed-array CLI with monotonic wall timing and signed-64-bit JSON.
- Preserve full binary64 precision in analysis fields so emitted reports replay exactly.
- Add independent oracle, wide-cutoff, fake-clock, replay, and workload tests.
- Validate caller-supplied array elements as signed 64-bit integers in both traversal helpers and use exact checked addition.
- Bound JSON token density before allocating the report AST, while retaining quote-aware strings and the 10,000-pair limit.
- Record Windows Devel::Cover evidence for the production module: 94.2% statement,
  67.0% branch, 50.8% condition, and 94.5% subroutine coverage.
