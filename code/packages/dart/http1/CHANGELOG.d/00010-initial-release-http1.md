## 0.1.0

- Initial Dart lane of the http1 identity: HTTP/1 request/response
  head parsing with body-offset and framing (`BodyKind`) decisions
  over the in-lane http-core types, plus redacted summaries that keep
  byte lengths instead of target/reason/header text; the sealed
  seven-variant `Http1ParseError` carries the reference's exact
  messages. Whitespace handling spells out the exact Unicode
  White_Space set (Dart's `\s` and `trim` both mishandle the BOM,
  which the reference preserves), and Content-Length past 2^63 - 1
  is rejected on Dart's signed-64-bit integers.
- Tests mirror the reference suite (10 unit tests), add 7 seeded
  property tests over the in-lane `pbt` package and a 2,000-iteration
  hostile fuzz campaign over the in-lane `fuzz` package; 100%
  line/function and 99.5% branch coverage.
