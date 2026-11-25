## 0.1.0

- Initial Dart lane of the codabar identity: `normalizeCodabar`,
  `encodeCodabar`, `expandCodabarRuns`, and `layoutCodabar`/
  `drawCodabar` over the in-lane barcode-layout-1d package, with
  stable v1 error identifiers and spec-order validation precedence
  (cap, guard form, options/conflicts, body scan).
- Tests: 18 unit (2 mirrored), 20 conformance (all 19 Codabar corpus
  cases; registered in consumers.json), 6 seeded properties over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 99%+ coverage gate.
