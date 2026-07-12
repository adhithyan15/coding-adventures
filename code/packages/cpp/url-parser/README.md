# url-parser (C++)

A pure ISO **C++17**, header-only URL parser with relative-reference resolution
and percent-coding, in namespace `ca::url`. A faithful port of the Rust
`url-parser` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

Splits an absolute URL into its components (scheme, userinfo, host, port, path,
query, fragment), reassembles them, resolves relative references (RFC 1808,
`.`/`..` removal), and percent-encodes / decodes. The scheme and host are
lower-cased; IPv6 hosts keep their brackets.

## API

```cpp
#include "url_parser.hpp"
namespace url = ca::url;

url::Url u = url::Url::parse("http://host.com:8080/p?x=1#f");
u.host;                        // std::optional<std::string> -> "host.com"
u.effective_port();            // std::optional<uint16_t>    -> 8080
u.authority();                 // "host.com:8080"
u.to_url_string();             // round trip

url::Url base = url::Url::parse("http://host/a/b/c.html");
base.resolve("../d.html").path;  // "/a/d.html"

url::percent_encode("a b");      // "a%20b"
url::percent_decode("a%20b");    // "a b"
```

`Url` uses `std::optional` for the optional components. `parse`, `resolve`, and
`percent_decode` throw `ca::url::ParseError` (carrying an `Error` kind) on
failure.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own cases — component split, lowercasing, `mailto:`,
default & explicit ports, authority, the scheme/port errors, percent
encode/decode (including invalid `%2`/`%GG` and a UTF-8 round trip), and the full
set of `resolve` cases.
