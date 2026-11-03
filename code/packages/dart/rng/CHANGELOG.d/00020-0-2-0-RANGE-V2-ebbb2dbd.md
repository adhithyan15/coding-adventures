## [0.2.0] — next_int_in_range spec v2

### Fixed

- **The range draw could hang forever.** It took the rejection threshold
  modulo 2^64 while drawing only 32 bits, so for a span such as 2^63 + 1
  every possible draw was rejected. For smaller spans it was biased and
  disagreed with the other languages, and no span above 2^32 could reach
  more than 2^32 − 1 above `min`. It now follows spec v2
  (`code/specs/rng.md`): 32-bit draws for spans up to 2^32, 64-bit draws with
  64-bit rejection for wider spans, one plain 64-bit draw for the whole
  64-bit domain. The three generators share one helper.
- `max - min + 1` was computed in wrapping `int`, wrong for spans past 2^63;
  the wide regimes now use `BigInt`.

### Added

- A conformance test running the shared corpus from its flat form
  `code/specs/fixtures/rng-v1/cases.txt`, plus regressions for the whole
  domain, wide ranges and span 2^63 + 1. Changelog sharded.
