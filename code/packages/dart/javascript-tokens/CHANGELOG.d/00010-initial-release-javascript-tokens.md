## 0.1.0

- Initial Dart lane of the javascript-tokens identity:
  `EsVersion` (14 editions, `latest`, `all`, `asStr`, strict
  `parse`/`tryParse`, chronological ordering, `UnknownEsVersion`
  error), `Span` (half-open byte ranges with `length`/`isEmpty`
  and lexicographic ordering), and the `TokenKind` sealed
  hierarchy (19 unit kinds plus payload-carrying `Other`, with
  `isTrivia`/`isEof` and value equality). Zero runtime
  dependencies, like the reference.
- Tests mirror the reference suite (18 of 19 — `default_is_latest`
  has no Dart counterpart) plus 3 lane extras, 9 seeded property
  tests over the in-lane `pbt` package, and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100%
  line coverage (all functions but the const enum constructor, a
  coverage-tool artifact).
