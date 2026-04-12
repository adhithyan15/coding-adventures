# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding.

Part of the **Venture** browser networking pipeline (NET00). Parses absolute URLs into structured components, resolves relative URLs against a base, and handles percent-encoding/decoding.

## URL anatomy

```text
  http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
  └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
 scheme  userinfo        host       port     path         query   fragment
```

## Usage

```rust
use url_parser::Url;

// Parse an absolute URL
let url = Url::parse("http://www.example.com:8080/docs/page.html?q=hello#s2").unwrap();
assert_eq!(url.scheme, "http");
assert_eq!(url.host.as_deref(), Some("www.example.com"));
assert_eq!(url.port, Some(8080));
assert_eq!(url.path, "/docs/page.html");
assert_eq!(url.query.as_deref(), Some("q=hello"));
assert_eq!(url.fragment.as_deref(), Some("s2"));

// Resolve a relative URL
let base = Url::parse("http://host/a/b/c.html").unwrap();
let resolved = base.resolve("../d.html").unwrap();
assert_eq!(resolved.path, "/a/d.html");

// Percent-encoding
use url_parser::{percent_encode, percent_decode};
assert_eq!(percent_encode("hello world"), "hello%20world");
assert_eq!(percent_decode("hello%20world").unwrap(), "hello world");
```

## API

- `Url::parse(input)` -- parse an absolute URL
- `Url::resolve(relative)` -- resolve a relative URL against this base
- `Url::effective_port()` -- explicit port or scheme default (80/443/21)
- `Url::authority()` -- reconstruct `[userinfo@]host[:port]`
- `Url::to_url_string()` -- serialize back to URL string
- `percent_encode(input)` -- encode non-unreserved characters
- `percent_decode(input)` -- decode `%XX` sequences

## Spec

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
cargo test -p url-parser -- --nocapture
```
