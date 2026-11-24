## 0.1.0

- Initial Dart lane of the code39 identity: `normalizeCode39`,
  `encodeCode39Char`, `encodeCode39`, `expandCode39Runs`, and
  `layoutCode39`/`drawCode39` over the in-lane barcode-layout-1d
  package, with stable v1 error identifiers, ASCII-only folding, and
  spec-order validation precedence.
- Tests: 17 unit (4 portable reference tests mirrored), 12
  conformance (all 11 Code 39 corpus cases; registered in
  consumers.json), 6 seeded properties over the in-lane `pbt`
  package, and a 2,000-iteration differential fuzz campaign over the
  in-lane `fuzz` package; 99%+ coverage gate.
