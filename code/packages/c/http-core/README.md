# http-core (C)

Shared HTTP message types and syntax-level helpers in pure ISO C17. A faithful
port of the Rust `http-core` crate.

Version-specific parsers disagree about wire syntax, but they should agree about
the semantic shapes application code consumes. This crate provides those shared
shapes — headers, versions, request/response heads, body-framing hints — plus
the helpers that read them: route-pattern matching, request-target splitting,
query-pair iteration, and `Content-*` parsing.

It is a **syntax-level** core: query values are **not** percent-decoded, so a
caller can apply its own decoding policy.

## API

```c
#include "http_core.h"

/* Route matching with named captures. */
HttpRoutePattern *pat = http_route_parse("/clip/v2/resource/:kind/:id");
HttpPairs m;
if (http_route_match_target(pat, "/clip/v2/resource/light/abc?limit=10", &m) == 1) {
    /* m.items[0] = {"kind","light"}, m.items[1] = {"id","abc"} */
    http_pairs_free(&m);
}
http_route_free(pat);

/* Header + Content-* helpers over a caller-owned HttpHeader array. */
HttpHeader hs[] = {{"Content-Type", "text/html; charset=utf-8"}};
char *media = NULL, *charset = NULL;
http_parse_content_type(hs, 1, &media, &charset);  /* "text/html", "utf-8" */
free(media); free(charset);
```

Functions that produce variable-length results write malloc'd data through an
out-parameter and return a status (`1` found / `0` absent / `-1` OOM, or `0`/`-1`
where there is no "absent" case); release each with the matching `*_free`.
Headers are passed in as caller-owned `HttpHeader` arrays and never freed by the
library. Where Rust returns `Result`/`Option`, this port returns a status code
(the error text is dropped; the outcome is identical).

## Portability

Pure ISO C17 — no POSIX `strdup`/`strndup`, no extensions. Compiles clean under
GCC, Clang, and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors, via the shared [`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
