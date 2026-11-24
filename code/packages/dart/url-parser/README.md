# coding_adventures_url_parser (Dart)

URL parsing, resolution, and canonicalization. The Dart lane of the Rust
`url-parser` crate (spec NET00): absolute-URL parsing with `scheme://` and
`scheme:path` forms, RFC 1808 relative resolution, identity
canonicalization, and percent-coding.

```dart
import 'package:coding_adventures_url_parser/coding_adventures_url_parser.dart';

final url = Url.parse('http://alice:secret@www.example.com:8080/docs?q=hi#s2');
print(url.host); // www.example.com
print(url.effectivePort()); // 8080

final docs = Url.parse('http://host/a/b/c.html').resolve('../d.html?x=1');
print(docs.toUrlString()); // http://host/a/d.html?x=1

print(percentDecode(percentEncode('hello world/日本語')));
```

## API

- `Url.parse(input)` — splits scheme, userinfo, host, port, path, query,
  fragment. Scheme and host are lowercased; the input is preserved for
  round-tripping. Throws [UrlError] (`missingScheme`, `invalidScheme`,
  `invalidPort`).
- `url.resolve(relative)` — empty reference (drops fragment), `#fragment`,
  absolute references, `//host/path`, `/path`, and merged relative paths
  with dot-segment removal.
- `url.effectivePort()` — explicit port else scheme default (http 80, https
  443, ftp 21). `url.authority()` — `[userinfo@]host[:port]`.
  `url.toUrlString()` — serialization (round-trips through `parse`, except
  the lossy no-host corner below).
- `url.canonicalize()` — lowercasing, default-port removal, percent-escape
  normalization (unreserved decoded, retained uppercased), dot-segment
  removal. Fragments are kept. Throws on malformed escapes.
- `percentEncode` / `percentDecode` — `%XX` conversion over UTF-8 bytes.
  Decode throws `invalidPercentEncoding` on truncated escapes, non-hex
  digits, invalid UTF-8, and lone surrogates.

## Reference notes

Faithful quirks (mirrored, with tests pinning each):

- Components store raw substrings, not decoded text; the fragment splits at
  the FIRST `#` (the spec text says "from the right" and "percent-decoded"
  — both are doc bugs against the code, logged for the spec owner).
- `UrlError.emptyHost` / `relativeWithoutBase` exist but are never thrown
  (in either lane).
- `host:` / `host:abc` keep the colon in the host; `[::1]:` is an error.
- No-host serialization drops userinfo and port (`http://:8080/p` →
  `http:/p`). Fixed-point properties therefore generate ports/userinfo only
  with hosts.
- Query-only references merge onto the base directory (`?q=2` on
  `/a/b` → `/a/`), unlike strict RFC 3986.
- Equality covers the preserved input, so `HTTP://h/p` ≠ `http://h/p`.

Dart adaptations: `Result` becomes thrown `UrlError` (an `enum implements
Exception`); lone surrogates (unrepresentable in Rust strings) fail
percent-coding with `invalidPercentEncoding`; the decoder pads around
Dart's leading-BOM stripping so `decode(encode('\uFEFF'))` round-trips as
in the reference.

## Validation

- **Unit** (`test/url_parser_test.dart`): mirrors the reference suite
  case-for-case, plus scheme/authority/resolve/canonicalize/percent/value
  extras. 87 tests.
- **Property** (`test/properties_test.dart`): codec round-trip and charset,
  parse/serialize fixed point, canonicalize idempotence and form,
  port-range errors, resolve-empty law — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration campaign over hostile
  strings (delimiters, escapes, lone/paired surrogates) asserting totality
  (only `UrlError` escapes), fixed points, and idempotence. It caught one
  real pre-release bug: Dart's `utf8.decode` strips a leading BOM, which
  the reference preserves (fixed with a BOM-safe decode + regression
  tests). Ad-hoc 100,000-iteration runs over five seeds: zero crashes.
- **Coverage**: 99.7% lines, 99.1% branches, 97.1% functions (one defensive
  arm documented in code; one implicit enum constructor).
