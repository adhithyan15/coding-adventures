## 0.1.0

- Initial Dart lane of the code128 identity: `normalizeCode128B`,
  `computeCode128Checksum`, `encodeCode128B`, `expandCode128Runs`,
  and `layoutCode128`/`drawCode128` over the in-lane
  barcode-layout-1d package, with stable v1 error identifiers,
  the 107-entry symbol table, and spec-order validation precedence.
- Tests: 14 unit (1 mirrored), 20 conformance (all 19 Code 128-B
  corpus cases; registered in consumers.json), 6 seeded properties
  over the in-lane `pbt` package, and a 2,000-iteration differential
  fuzz campaign over the in-lane `fuzz` package; 99%+ coverage gate.
