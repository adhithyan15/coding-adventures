## Unreleased — Microbenchmark fixture changes select Go and Python consumers

- The canonical paired-results fixture now selects `go/microbenchmark`, `go/rank-statistics`,
  `python/microbenchmark` and `python/rank-statistics` as repository inputs. The Kiln graph export
  carries the same exact fixture path for those consumers.
- Those four consumers also include the canonical fixture in their fallback cache hashes, so
  changing its bytes invalidates cached builds when a cache is used.
- Selector tests cover language filters, lookalike paths and missing consumers; graph tests pin the
  exported input for each lane.
