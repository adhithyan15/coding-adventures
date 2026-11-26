# coding_adventures_http_core (Dart)

Shared HTTP message types and helpers. The Dart lane of the Rust
`http-core` crate.

```dart
import 'package:coding_adventures_http_core/http_core.dart';

final head = RequestHead(
  method: 'GET',
  target: '/api/devices?room=kitchen',
  version: HttpVersion.parse('HTTP/1.1'),
  headers: [Header('Accept', 'application/json')],
);
print(head.path()); // /api/devices
print(head.queryValue('room')); // kitchen
```

## API

- `Header`, `RequestHead`, `ResponseHead` — semantic message
  shapes with `header()`, `contentLength()`, `contentType()`,
  and (requests) `path()` / `queryValue()`.
- `HttpVersion.parse` — `HTTP/x.y` with 16-bit parts; throws
  [InvalidHttpVersion] unless exact.
- `parseRequestTarget` — path/query/fragment split;
  `RequestTarget.queryPairs()` streams raw pairs (never
  percent-decoded) and `queryValue()` takes the first match.
- `RoutePattern.parse` / `matchPath` / `matchTarget` — `:param`
  capture over path segments; targets match by path only.
- `findHeader` (first match, ASCII case-insensitive),
  `parseContentLength`, `parseContentType` (media type plus
  optional charset), `splitPathSegments`, `BodyKind`.

Numeric fields validate ASCII-digits-only before parsing (Dart's
`int.parse` accepts signs; Rust's unsigned parse does not):
versions must fit 16 bits exactly, while content lengths at or
above 2^63 return null where the reference would return `Some`
(documented, boundary-tested). Header lookup folds ASCII case
only.

## Validation

- **Unit** (`test/http_core_test.dart`): mirrors all 9 reference
  tests, plus edge matrices over versions, headers, content
  helpers, targets, routes, query pairs, body kinds, and value
  semantics. 24 tests.
- **Property** (`test/properties_test.dart`): version
  round-trips, header lookup semantics, content-length digits,
  target invariants, route matching, and segment splitting —
  fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile Latin-1 targets, patterns, headers, and
  versions; only [InvalidHttpVersion] escapes. Ad-hoc
  20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
