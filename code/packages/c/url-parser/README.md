# url-parser (C)

A pure ISO **C17** URL parser with relative-reference resolution and
percent-coding. A faithful port of the Rust `url-parser` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

Splits an absolute URL into its components and reassembles them:

```
http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2
scheme    userinfo         host        port     path       query  fragment
```

It also implements RFC 1808 relative resolution (`url_resolve`, including
`.`/`..` removal) and percent-encoding / decoding. The scheme and host are
lower-cased; the path starts with `/` for authority-based URLs; query/fragment
exclude their leading `?`/`#`; IPv6 hosts keep their brackets.

## API

```c
#include "url_parser.h"

Url u;
if (url_parse("http://host.com:8080/p?x=1#f", &u) == URL_OK) {
    /* u.scheme, u.host, u.has_port/u.port, u.path, u.query, u.fragment */
    unsigned short p;
    url_effective_port(&u, &p);          /* explicit or scheme default */
    char *auth = url_authority(&u);      /* "host.com:8080" */
    char *s = url_to_string(&u);
    free(auth); free(s);
    url_free(&u);
}

Url base, r;
url_parse("http://host/a/b/c.html", &base);
url_resolve(&base, "../d.html", &r);     /* r.path == "/a/d.html" */

char *enc = url_percent_encode("a b");   /* "a%20b" */
char *dec; url_percent_decode("a%20b", &dec);
```

`url_parse` / `url_resolve` return `URL_OK` or a `URL_ERR_*` status (on error the
`Url` is zeroed). `url_authority` / `url_to_string` / `url_percent_encode` return
a malloc'd string (or `NULL` on allocation failure). Every parsed `Url` must be
released with `url_free`. All growable buffers are overflow-guarded.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own cases — component split, lowercasing, `mailto:`,
default & explicit ports, authority, the scheme/port errors, percent
encode/decode (including invalid `%2`/`%GG` and a UTF-8 round trip), and the
full set of `resolve` cases.
