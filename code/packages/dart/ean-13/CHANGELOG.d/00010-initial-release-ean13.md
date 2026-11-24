## 0.1.0

- Initial Dart lane of the ean-13 identity: `normalizeEan13`,
  `computeEan13CheckDigit`, `leftParityPattern`, `encodeEan13`,
  `expandEan13Runs`, and `layoutEan13`/`drawEan13` over the in-lane
  barcode-layout-1d package, with stable v1 error identifiers and
  spec-order validation precedence (length before character, 4096
  scalar cap).
- Tests: 15 unit (1 mirrored), 21 conformance (all 20 EAN-13 corpus
  cases; registered in consumers.json), 7 seeded properties over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 99%+ coverage gate.
