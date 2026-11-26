## 0.1.0

- Initial Dart lane of the brotli identity: `compress`/
  `decompress` over the in-lane huffman-tree package with
  `literalContext` classification and the twelve-variant
  `BrotliError` type; fail-closed on the reference's silent
  corruptions (256-symbol buckets, 4 GiB inputs, out-of-range
  table symbols).
- Tests mirror the reference suite (15 unit tests), add 6
  extras, 6 seeded property tests over the in-lane `pbt`
  package, and a 2,000-iteration differential fuzz campaign
  over the in-lane `fuzz` package; 100% line/function coverage.
