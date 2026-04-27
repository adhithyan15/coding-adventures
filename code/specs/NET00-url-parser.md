# NET00 — URL Parser

## Overview

A URL (Uniform Resource Locator) is a string that tells you **where** something
is on the internet and **how** to get it. Every time you type an address into a
browser, click a link, or fetch an API — the first step is always the same:
parse the URL.

This package parses URLs according to RFC 1738 (1994), the specification that
Mosaic and early web browsers used. It is deliberately simple — no
Internationalized Domain Names, no `data:` URIs, no WHATWG URL Standard
complexity. Just the fundamentals.

**Analogy:** A URL is like a postal address:

```
  http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2
  └─┬─┘ └────┬─────┘└──────┬───────┘└─┬─┘└─────┬───────┘└──┬───┘└───┬───┘
 scheme  userinfo        host       port     path         query   fragment

  scheme   = "how to deliver"    (http = web, ftp = file transfer)
  host     = "which building"    (www.example.com)
  port     = "which door"        (8080, defaults to 80 for http)
  path     = "which room inside" (/docs/page.html)
  query    = "what to ask for"   (?q=hello)
  fragment = "which paragraph"   (#section2) — client-side only, never sent to server
  userinfo = "credentials"       (alice:secret) — rare today, common in early web
```

## Where It Fits

```
User types "http://info.cern.ch/hypertext/WWW/TheProject.html"
     │
     ▼
url-parser (NET00) ← THIS PACKAGE
     │  Url { scheme: "http", host: "info.cern.ch",
     │        port: 80, path: "/hypertext/WWW/TheProject.html" }
     ▼
tcp-client (NET01) — connect to info.cern.ch:80
     │
     ▼
http1.0-client (NET05) — GET /hypertext/WWW/TheProject.html HTTP/1.0
```

**Depends on:** nothing (std only)
**Depended on by:** http1.0-client (NET05), Venture browser (BR01)

---

## Concepts

### What Is a URL?

RFC 1738 defines the generic URL syntax:

```
<scheme>://<authority>/<path>?<query>#<fragment>
```

Where `<authority>` breaks down further:

```
<userinfo>@<host>:<port>
```

Not all parts are required. The minimal URL is just `scheme:path`, like
`mailto:alice@example.com`. For HTTP URLs, the host is required but everything
else has defaults.

### Absolute vs. Relative URLs

An **absolute URL** contains a scheme and a host — it stands alone:

```
http://www.example.com/docs/page.html
```

A **relative URL** is interpreted against a **base URL** — the URL of the
current page:

```
Base:     http://www.example.com/docs/page.html
Relative: ../images/photo.gif
Resolved: http://www.example.com/images/photo.gif
```

Relative URLs were critical in early HTML. Almost every `<img src="...">` and
`<a href="...">` used a relative path. The resolution algorithm is:

1. If the relative URL has a scheme, it is already absolute — use it directly.
2. If it starts with `//`, inherit only the scheme from base.
3. If it starts with `/`, inherit scheme + authority, replace path.
4. Otherwise, merge: take the base path up to the last `/`, append relative.
5. Normalize: collapse `//` to `/`, resolve `.` and `..` segments.

### Percent-Encoding

URLs can only contain a limited set of ASCII characters. Anything else must be
**percent-encoded**: the byte value written as `%XX` in hexadecimal.

```
Space      → %20
Question   → %3F  (when literal, not as query delimiter)
日本語     → %E6%97%A5%E6%9C%AC%E8%AA%9E  (UTF-8 bytes, each percent-encoded)
```

The **unreserved characters** that do NOT need encoding are:

```
A-Z  a-z  0-9  -  _  .  ~
```

Everything else is either a **reserved delimiter** (`/`, `?`, `#`, `@`, `:`)
with special meaning in the URL structure, or must be percent-encoded.

---

## Public API

```rust
/// A parsed URL with all components separated.
///
/// All string fields are percent-decoded. The `raw` field preserves the
/// original input for round-tripping.
pub struct Url {
    pub scheme: String,          // "http", "ftp", "mailto" — lowercased
    pub userinfo: Option<String>, // "alice:secret" or None
    pub host: Option<String>,     // "www.example.com" — lowercased
    pub port: Option<u16>,        // explicit port, or None (use scheme default)
    pub path: String,             // "/docs/page.html" — always starts with / for http
    pub query: Option<String>,    // "q=hello&lang=en" — without the leading ?
    pub fragment: Option<String>, // "section2" — without the leading #
    raw: String,                  // original input, preserved verbatim
}

impl Url {
    /// Parse an absolute URL string.
    ///
    /// Returns Err if the string is not a valid absolute URL (must have a
    /// scheme). For relative URLs, use `resolve()` instead.
    pub fn parse(input: &str) -> Result<Url, UrlError> { ... }

    /// Resolve a relative URL against this URL as the base.
    ///
    /// Implements the RFC 1738 / RFC 1808 relative resolution algorithm:
    /// - "http://other.com/x" → absolute, returned as-is
    /// - "//other.com/x" → inherits scheme
    /// - "/x" → inherits scheme + authority
    /// - "x" → merges with base path
    /// - "../x" → merges and resolves parent traversal
    pub fn resolve(&self, relative: &str) -> Result<Url, UrlError> { ... }

    /// The effective port — explicit port if set, otherwise scheme default.
    ///
    /// Returns 80 for http, 21 for ftp, 443 for https, None for unknown.
    pub fn effective_port(&self) -> Option<u16> { ... }

    /// The authority string: [userinfo@]host[:port]
    pub fn authority(&self) -> String { ... }

    /// Serialize back to a URL string.
    ///
    /// Percent-encodes components as needed. The result is a valid URL that
    /// round-trips through parse().
    pub fn to_string(&self) -> String { ... }
}

/// Percent-decode a string: "%20" → " ", "%E6%97%A5" → "日"
pub fn percent_decode(input: &str) -> Result<String, UrlError> { ... }

/// Percent-encode a string for use in a URL path or query.
pub fn percent_encode(input: &str) -> String { ... }
```

### Error Types

```rust
pub enum UrlError {
    /// No scheme found (e.g., "www.example.com" without "http://")
    MissingScheme,

    /// Scheme contains invalid characters (must be [a-z][a-z0-9+.-]*)
    InvalidScheme,

    /// Port is not a valid u16 (e.g., "http://host:99999")
    InvalidPort,

    /// Percent-encoding is malformed (e.g., "%GG", "%2" truncated)
    InvalidPercentEncoding,

    /// Empty host in an authority-based URL ("http:///path")
    EmptyHost,

    /// Relative URL cannot be resolved without a base
    RelativeWithoutBase,
}
```

---

## Scheme Defaults

| Scheme | Default Port | Authority Required? |
|--------|-------------|-------------------|
| http   | 80          | Yes               |
| https  | 443         | Yes               |
| ftp    | 21          | Yes               |
| mailto | —           | No (path is email)|

For Venture v0.1, only `http` is used. But the parser handles any scheme
generically so it can grow.

---

## Parsing Algorithm

The URL is parsed left-to-right in a single pass, no backtracking:

```
Input: "http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"

Step 1: Find "://" → scheme = "http"
        Remaining: "alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2"

Step 2: Find "#" from the right → fragment = "sec2"
        Remaining: "alice:secret@www.example.com:8080/docs/page.html?q=hello"

Step 3: Find "?" → query = "q=hello"
        Remaining: "alice:secret@www.example.com:8080/docs/page.html"

Step 4: Find first "/" → path = "/docs/page.html"
        Remaining: "alice:secret@www.example.com:8080"

Step 5: Find "@" → userinfo = "alice:secret"
        Remaining: "www.example.com:8080"

Step 6: Find last ":" → port = 8080
        Remaining: "www.example.com"

Step 7: host = "www.example.com" (lowercased)
```

**Edge cases:**
- IPv6 addresses: `http://[::1]:8080/` — brackets delimit the host
- Empty path: `http://host` → path defaults to `/`
- Trailing slash: `http://host/` and `http://host` are different URLs
  (the former has path `/`, the latter has an implied `/`)

---

## Relative Resolution Algorithm

Given base URL `B` and relative reference `R`:

```
if R has scheme:
    return R (already absolute)

if R starts with "//":
    result.scheme = B.scheme
    result.authority = R.authority
    result.path = R.path
    return result

result.scheme = B.scheme
result.authority = B.authority

if R starts with "/":
    result.path = R.path
else:
    result.path = merge(B.path, R.path)

result.path = remove_dot_segments(result.path)
result.query = R.query
result.fragment = R.fragment
return result
```

The `merge()` function:
- Take everything in `B.path` up to and including the last `/`
- Append `R.path`
- Example: base `/a/b/c` + relative `d` = `/a/b/d`

The `remove_dot_segments()` function:
- `/a/b/../c` → `/a/c`
- `/a/./b` → `/a/b`
- `/a/b/../../c` → `/c`

---

## Testing Strategy

### Unit Tests

1. **Basic parsing:** scheme, host, port, path for common URLs
2. **All components:** URLs with every field populated
3. **Missing components:** URLs with only scheme+host, scheme+host+path, etc.
4. **Percent-encoding:** decode `%20`, `%2F`, multi-byte UTF-8 sequences
5. **Percent-encoding roundtrip:** encode then decode returns original
6. **Invalid URLs:** missing scheme, bad port, malformed percent-encoding
7. **Case normalization:** scheme and host are lowercased

### Relative Resolution Tests

8. **Same directory:** `"photo.gif"` against `"http://host/dir/page.html"`
9. **Parent directory:** `"../other/file"` against a nested base
10. **Absolute path:** `"/root/file"` against any base
11. **Scheme-relative:** `"//other.com/path"` inherits scheme
12. **Already absolute:** `"http://other.com"` ignores base entirely
13. **Dot segment removal:** `"/a/b/../c"` → `"/a/c"`
14. **Empty relative:** `""` returns the base URL without fragment
15. **Fragment-only:** `"#sec"` returns base URL with new fragment

### Historical Tests

16. **Real Mosaic-era URLs:** parse actual URLs from archived 1993 web pages
17. **CERN info.cern.ch:** `http://info.cern.ch/hypertext/WWW/TheProject.html`

---

## Scope

**In scope:**
- RFC 1738 URL parsing (http, https, ftp, mailto schemes)
- Relative URL resolution (RFC 1808)
- Percent-encoding and decoding
- Scheme and host case normalization

**Out of scope:**
- WHATWG URL Standard (the modern browser spec — much more complex)
- Internationalized Domain Names (IDN / punycode)
- `data:` and `blob:` URIs
- URL template expansion (RFC 6570)
- Query string key=value parsing (that is application logic, not URL syntax)

---

## Implementation Languages

This package will be implemented in:
- **Rust** (primary, for Venture browser)
- Future: all 9 languages following the standard pattern
