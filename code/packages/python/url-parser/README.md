# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding for Python.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```python
from url_parser import Url, percent_encode, percent_decode

url = Url.parse("https://example.com:8080/path?q=hello#top")
print(url.scheme)           # "https"
print(url.host)             # "example.com"
print(url.port)             # 8080
print(url.effective_port()) # 8080
print(url.path)             # "/path"

# Relative resolution
base = Url.parse("http://example.com/a/b/c")
resolved = base.resolve("../d")
print(resolved.to_url_string())  # "http://example.com/a/d"

# Percent-encoding
print(percent_encode("hello world"))  # "hello%20world"
print(percent_decode("caf%C3%A9"))    # "cafe"
```

## API

- **`Url.parse(input: str) -> Url`** -- parse an absolute URL string
- **`Url.resolve(relative: str) -> Url`** -- resolve a relative URL against this base
- **`Url.to_url_string() -> str`** -- serialize back to a URL string
- **`Url.effective_port() -> int | None`** -- explicit port or scheme default
- **`Url.authority() -> str`** -- reconstruct the authority component
- **`percent_encode(s: str) -> str`** -- encode a string with %XX sequences
- **`percent_decode(s: str) -> str`** -- decode %XX sequences back to a string
- **Error classes**: `UrlError`, `MissingScheme`, `InvalidScheme`, `InvalidPort`, `InvalidPercentEncoding`, `EmptyHost`, `RelativeWithoutBase`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
