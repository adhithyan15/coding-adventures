# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding for TypeScript.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```typescript
import { Url, percentEncode, percentDecode } from "@coding-adventures/url-parser";

const url = Url.parse("https://example.com:8080/path?q=hello#top");
console.log(url.scheme);           // "https"
console.log(url.host);             // "example.com"
console.log(url.port);             // 8080
console.log(url.effectivePort());  // 8080
console.log(url.path);             // "/path"

// Relative resolution
const base = Url.parse("http://example.com/a/b/c");
const resolved = base.resolve("../d");
console.log(resolved.toUrlString());  // "http://example.com/a/d"

// Percent-encoding
console.log(percentEncode("hello world"));  // "hello%20world"
console.log(percentDecode("caf%C3%A9"));    // "cafe"
```

## API

- **`Url.parse(input: string): Url`** -- parse an absolute URL string
- **`Url.resolve(relative: string): Url`** -- resolve a relative URL against this base
- **`Url.toUrlString(): string`** -- serialize back to a URL string
- **`Url.effectivePort(): number | null`** -- explicit port or scheme default
- **`Url.authority(): string`** -- reconstruct the authority component
- **`percentEncode(s: string): string`** -- encode a string with %XX sequences
- **`percentDecode(s: string): string`** -- decode %XX sequences back to a string
- **Error classes**: `UrlError`, `MissingScheme`, `InvalidScheme`, `InvalidPort`, `InvalidPercentEncoding`, `EmptyHost`, `RelativeWithoutBase`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
