# http-core (C++)

Shared HTTP message types and syntax-level helpers in pure ISO C++17,
header-only, in namespace `ca::http`. A faithful port of the Rust `http-core`
crate.

Version-specific parsers disagree about wire syntax, but they should agree about
the semantic shapes application code consumes. This crate provides those shared
shapes — headers, versions, request/response heads, body-framing hints — plus
route-pattern matching, request-target splitting, query-pair iteration, and
`Content-*` parsing. Query values are **not** percent-decoded.

## API

```cpp
#include "http_core.hpp"
namespace http = ca::http;

auto pat = http::RoutePattern::parse("/clip/v2/resource/:kind/:id");
if (auto m = pat.match_target("/clip/v2/resource/light/abc?limit=10")) {
    // *m == {{"kind","light"}, {"id","abc"}}
}

std::vector<http::Header> hs = {{"Content-Type", "text/html; charset=utf-8"}};
auto ct = http::parse_content_type(hs);   // ("text/html", "utf-8")
auto v  = http::HttpVersion::parse("HTTP/1.1");  // {1, 1}
```

Value semantics throughout: `match_path`/`match_target`, `parse_content_type`,
`HttpVersion::parse`, and the head helpers return `std::optional`; `find_header`
returns a `const std::string*` (or `nullptr`). Where Rust returns
`Result<_, String>`, this port returns `std::optional` (the error text is
dropped; the outcome is identical).

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
