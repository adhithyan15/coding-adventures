# coding_adventures_http1 (Dart)

HTTP/1 request and response head parser with body framing detection.
The Dart lane of the Rust `http1` crate.

```dart
import 'package:coding_adventures_http1/http1.dart';

// `bytes` comes from the caller; the package never touches the filesystem.
final parsed = parseRequestHead(bytes);
print(parsed.head.method); // GET
print(parsed.bodyOffset); // where the body starts
print(parsed.bodyKind); // how to consume it
```

## API

- `parseRequestHead` / `parseResponseHead` return `ParsedRequestHead` /
  `ParsedResponseHead`: the semantic head (`RequestHead` / `ResponseHead`
  from the in-lane `http-core` package), the body offset, and the
  `BodyKind` framing decision.
- `summary()` on either result returns a redacted
  `Http1RequestHeadSummary` / `Http1ResponseHeadSummary`: byte lengths
  stand in for the target, reason, headers, and body.
- Errors: the sealed `Http1ParseError` with seven subtypes carrying the
  reference's exact messages (thrown).

Head splitting skips leading blank lines, accepts LF-only input,
detects chunked transfer coding and Content-Length (first match wins,
ASCII case-insensitive), and applies the bodyless-status rules (1xx,
204, 304) before any header.

## Deliberate divergences from the reference

- Whitespace splitting and trimming use a spelled-out Unicode
  White_Space set: Dart's `\s` misses NEL and wrongly splits the BOM,
  and `String.trim` wrongly strips the BOM, while the reference keeps
  a leading BOM as U+FEFF (preserved here explicitly).
- Content-Length past 2^63 - 1 is rejected: Dart integers are signed
  64-bit, where a 64-bit reference accepts up to 2^64 - 1. No
  framing decision changes, and the rejection carries the same error.
- The `VERSION` constant is not ported, matching the http-core lane.

## Validation

- **Unit** (`test/http1_test.dart`): all 10 reference tests plus the
  error matrix, byte-vs-char lengths, the exact-whitespace behavior,
  BOM preservation, framing precedence, and value semantics.
- **Property** (`test/properties_test.dart`): fail-closed vocabulary,
  request/response round-trips, the framing decision table, request
  defaults, summary redaction, and purity — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign;
  only `Http1ParseError` escapes.
- **Coverage**: 100% lines and functions, 99.5% branches — the two
  uncovered branches are the provably-unreachable empty-lines arms
  kept for reference parity.
