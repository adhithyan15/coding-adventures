# url_parser

RFC 1738 URL parser with relative resolution and percent-encoding for Elixir.

Part of the Venture browser networking pipeline (NET00).

```text
  scheme://userinfo@host:port/path?query#fragment
  └─────┘ └──────┘ └──┘ └──┘└───┘└────┘└───────┘
  protocol  auth   server port route search  anchor
```

## Usage

```elixir
alias CodingAdventures.UrlParser

{:ok, url} = UrlParser.parse("https://example.com:8080/path?q=hello#top")
# url.scheme == "https"
# url.host   == "example.com"
# url.port   == 8080
# url.path   == "/path"

# Relative resolution
{:ok, resolved} = UrlParser.resolve(url, "../d")
UrlParser.to_url_string(resolved)  # "https://example.com/a/d"

# Effective port (explicit or scheme default)
UrlParser.effective_port(url)  # 8080

# Percent-encoding
UrlParser.percent_encode("hello world")     # "hello%20world"
{:ok, decoded} = UrlParser.percent_decode("caf%C3%A9")  # "cafe"
```

## API

- **`parse(input) :: {:ok, t()} | {:error, atom()}`** -- parse an absolute URL string
- **`resolve(base, relative) :: {:ok, t()} | {:error, atom()}`** -- resolve a relative URL
- **`to_url_string(url) :: String.t()`** -- serialize back to a URL string
- **`effective_port(url) :: non_neg_integer() | nil`** -- explicit port or scheme default
- **`authority(url) :: String.t()`** -- reconstruct the authority component
- **`percent_encode(input) :: String.t()`** -- encode a string with %XX sequences
- **`percent_decode(input) :: {:ok, String.t()} | {:error, atom()}`** -- decode %XX sequences
- **Error atoms**: `:missing_scheme`, `:invalid_scheme`, `:invalid_port`, `:invalid_percent_encoding`, `:relative_without_base`

## Specification

See `code/specs/NET00-url-parser.md` for the full specification.

## Development

```bash
# Run tests
bash BUILD
```
