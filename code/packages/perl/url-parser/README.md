# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding. A faithful Perl port of the Rust `url-parser` crate from the coding-adventures project.

## URL anatomy

```text
  http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
  └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
 scheme  userinfo        host       port     path         query   fragment
```

## Usage

```perl
use CodingAdventures::UrlParser qw(
    parse resolve effective_port authority
    to_url_string percent_encode percent_decode
);

# Parse a URL into its components
my $url = parse("http://www.example.com:8080/docs?q=hello#s2");
# $url->{scheme}   → "http"
# $url->{host}     → "www.example.com"
# $url->{port}     → 8080
# $url->{path}     → "/docs"
# $url->{query}    → "q=hello"
# $url->{fragment} → "s2"

# Effective port (explicit or scheme default)
effective_port($url);  # → 8080
effective_port(parse("https://example.com"));  # → 443

# Resolve relative URLs
my $base     = parse("http://host/a/b/c.html");
my $resolved = resolve($base, "../d.html");
# $resolved->{path} → "/a/d.html"

# Percent-encoding
percent_encode("hello world");     # → "hello%20world"
percent_decode("hello%20world");   # → "hello world"
```

## API

| Function | Description |
|---|---|
| `parse($input)` | Parse an absolute URL string; returns hashref or dies |
| `resolve($base, $relative)` | Resolve a relative URL against a base; returns hashref or dies |
| `effective_port($url)` | Explicit port or scheme default (http:80, https:443, ftp:21) |
| `authority($url)` | Build `[userinfo@]host[:port]` string |
| `to_url_string($url)` | Serialize a parsed URL back to a string |
| `percent_encode($input)` | Encode non-unreserved characters as `%XX` (uppercase) |
| `percent_decode($input)` | Decode `%XX` sequences back to UTF-8 characters |

## Parsing algorithm

Single-pass, left-to-right, no backtracking:

1. `://` separates the scheme; no `://` tries `scheme:path` form (mailto)
2. `#` splits the fragment
3. `?` splits the query
4. First `/` splits path from authority
5. `@` in authority splits userinfo
6. Last `:` in authority (or after `]` for IPv6) splits port
7. Remainder is the host (lowercased; empty becomes `undef`)

## Development

```bash
# Run tests
bash BUILD
```
