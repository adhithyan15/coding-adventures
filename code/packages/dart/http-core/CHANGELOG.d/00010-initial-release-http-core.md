## 0.1.0

- Initial Dart lane of the http-core identity: `Header`,
  `RequestHead`/`ResponseHead`, `HttpVersion`,
  `RequestTarget` with lazy query pairs, `RoutePattern` with
  path-only target matching, `BodyKind`, and the
  find/parse/split helpers; `InvalidHttpVersion` carries the
  reference's exact message.
- Tests mirror the reference suite (9 unit tests), add 15 edge
  extras, 6 seeded property tests over the in-lane `pbt`
  package, and a 2,000-iteration differential fuzz campaign over
  the in-lane `fuzz` package; 100% line/function coverage.
