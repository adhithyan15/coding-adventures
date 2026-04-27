# url_parser

RFC 1738 URL parser with relative resolution and percent-encoding for Lua.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```lua
local url_parser = require("coding_adventures.url_parser")

local url, err = url_parser.parse("https://example.com:8080/path?q=hello#top")
print(url.scheme)   -- "https"
print(url.host)     -- "example.com"
print(url.port)     -- 8080
print(url.path)     -- "/path"

-- Relative resolution
local resolved, err = url_parser.resolve(url, "../d")
print(url_parser.to_url_string(resolved))

-- Effective port (explicit or scheme default)
print(url_parser.effective_port(url))  -- 8080

-- Percent-encoding
print(url_parser.percent_encode("hello world"))  -- "hello%20world"
local decoded, err = url_parser.percent_decode("caf%C3%A9")  -- "cafe"
```

## API

- **`parse(input) -> url, err`** -- parse an absolute URL string
- **`resolve(base, relative) -> url, err`** -- resolve a relative URL against a base
- **`to_url_string(url) -> string`** -- serialize back to a URL string
- **`effective_port(url) -> number|nil`** -- explicit port or scheme default
- **`authority(url) -> string`** -- reconstruct the authority component
- **`percent_encode(input) -> string`** -- encode a string with %XX sequences
- **`percent_decode(input) -> string, err`** -- decode %XX sequences back to a string
- **Error strings**: `"missing_scheme"`, `"invalid_scheme"`, `"invalid_percent_encoding"`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
