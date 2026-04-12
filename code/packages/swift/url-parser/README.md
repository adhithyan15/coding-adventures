# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding for Swift.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```swift
import UrlParser

let url = try Url.parse("https://example.com:8080/path?q=hello#top")
// url.scheme == "https"
// url.host   == "example.com"
// url.port   == 8080
// url.path   == "/path"

// Relative resolution
let base = try Url.parse("http://example.com/a/b/c")
let resolved = try base.resolve("../d")
print(resolved.toUrlString())  // "http://example.com/a/d"

// Effective port (explicit or scheme default)
url.effectivePort()  // Optional(8080)

// Percent-encoding
percentEncode("hello world")       // "hello%20world"
try percentDecode("caf%C3%A9")     // "cafe"
```

## API

- **`Url.parse(_ input: String) throws -> Url`** -- parse an absolute URL string
- **`Url.resolve(_ relative: String) throws -> Url`** -- resolve a relative URL
- **`Url.toUrlString() -> String`** -- serialize back to a URL string
- **`Url.effectivePort() -> UInt16?`** -- explicit port or scheme default
- **`Url.authority() -> String`** -- reconstruct the authority component
- **`percentEncode(_ input: String) -> String`** -- encode with %XX sequences
- **`percentDecode(_ input: String) throws -> String`** -- decode %XX sequences
- **`UrlError`** enum: `.missingScheme`, `.invalidScheme`, `.invalidPort`, `.invalidPercentEncoding`, `.emptyHost`, `.relativeWithoutBase`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
