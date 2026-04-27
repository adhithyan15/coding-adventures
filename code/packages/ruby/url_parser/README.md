# url-parser

RFC 1738 URL parser with relative resolution and percent-encoding. A Ruby port of the Rust `url-parser` crate from this repository.

## What it does

Parses absolute URLs into their component parts (scheme, userinfo, host, port, path, query, fragment), resolves relative URLs against a base per RFC 1808, and handles percent-encoding/decoding.

## URL anatomy

```
http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
└─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
scheme  userinfo        host       port     path         query   fragment
```

## Usage

```ruby
require "coding_adventures_url_parser"

# Parse a URL
url = CodingAdventures::UrlParser::Url.parse("http://www.example.com:8080/docs/page.html?q=hello#s2")
url.scheme         # => "http"
url.host           # => "www.example.com"
url.port           # => 8080
url.path           # => "/docs/page.html"
url.query          # => "q=hello"
url.fragment       # => "s2"
url.effective_port # => 8080

# Resolve a relative URL
base = CodingAdventures::UrlParser::Url.parse("http://host/a/b/c.html")
resolved = base.resolve("../d.html")
resolved.path # => "/a/d.html"

# Percent-encoding
CodingAdventures::UrlParser.percent_encode("hello world") # => "hello%20world"
CodingAdventures::UrlParser.percent_decode("%E6%97%A5")    # => "日"
```

## How it fits in the stack

This is a standalone library with no dependencies. It mirrors the Rust `url-parser` crate and is part of the coding-adventures polyglot package collection.

## Development

```bash
# Run tests
bash BUILD
```
