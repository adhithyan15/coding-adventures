## 0.1.0

- Initial Dart lane of the font-parser identity: metrics-only
  OpenType/TrueType parsing (`head`, `hhea`, `maxp`, cmap Format 4,
  `hmtx`, kern Format 0, `name`, `OS/2`) plus the OpenType MATH table
  (constants, italic corrections, variant/assembly constructions);
  the sealed five-variant `FontError` carries the reference's exact
  messages, and out-of-range integers throw `ArgumentError` per lane
  convention. Two reference panic paths fail closed instead
  (truncated directory records, zero-hMetrics shared advance).
- Tests mirror the reference suite (28 unit tests plus the 10
  fontTools-oracle MATH tests), add 2 horizontal-construction oracles
  generated the same way, 9 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over the
  in-lane `fuzz` package; 100% line/branch/function coverage.
