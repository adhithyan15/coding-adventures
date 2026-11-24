## 0.1.0

- Initial Dart lane of the upc-a identity: `normalizeUpcA`,
  `computeUpcACheckDigit`, `encodeUpcA`, `expandUpcARuns`, and
  `layoutUpcA`/`drawUpcA` over the in-lane barcode-layout-1d package,
  with stable v1 error identifiers and spec-order validation
  precedence (length before character, 4096 scalar cap).
- Tests: 15 unit (2 mirrored), 13 conformance (all 12 UPC-A corpus
  cases; registered in consumers.json), 7 seeded properties over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 95%+ coverage gate.
