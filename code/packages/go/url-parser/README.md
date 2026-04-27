# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding for Go.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```go
import urlparser "github.com/coding-adventures/url-parser"

url, err := urlparser.Parse("https://example.com:8080/path?q=hello#top")
// url.Scheme == "https"
// *url.Host  == "example.com"
// *url.Port  == 8080
// url.Path   == "/path"

// Relative resolution
resolved, err := url.Resolve("../d")
fmt.Println(resolved.ToUrlString())

// Percent-encoding
encoded := urlparser.PercentEncode("hello world")  // "hello%20world"
decoded, err := urlparser.PercentDecode("caf%C3%A9")  // "cafe"
```

## API

- **`Parse(input string) (*Url, error)`** -- parse an absolute URL string
- **`(*Url).Resolve(relative string) (*Url, error)`** -- resolve a relative URL against this base
- **`(*Url).ToUrlString() string`** -- serialize back to a URL string
- **`(*Url).EffectivePort() *uint16`** -- explicit port or scheme default
- **`(*Url).Authority() string`** -- reconstruct the authority component
- **`PercentEncode(input string) string`** -- encode a string with %XX sequences
- **`PercentDecode(input string) (string, error)`** -- decode %XX sequences back to a string
- **`UrlError`** struct with Kind field: `"MissingScheme"`, `"InvalidScheme"`, `"InvalidPort"`, `"InvalidEncoding"`, `"InvalidHost"`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
