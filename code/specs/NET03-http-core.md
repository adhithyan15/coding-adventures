# NET03 — HTTP Core

## Overview

HTTP has two very different kinds of complexity:

1. **Semantic message shape** — requests, responses, versions, headers, and
   the rules for looking up and interpreting those headers
2. **Version-specific wire syntax** — HTTP/1.x start lines, HTTP/2 binary
   frames, HTTP/3 over QUIC streams

This package captures the first category only. `http-core` is the shared data
model that every HTTP version-specific parser should target.

The browser, an API client, or a server framework should not care whether a
response head came from:

- a text parser reading `HTTP/1.0 200 OK\r\n`
- an HTTP/2 HEADERS frame
- or an HTTP/3 control stream

At that layer, the application needs the same concepts:

- `Header`
- `HttpVersion`
- `BodyKind`
- `RequestHead`
- `ResponseHead`

That is the job of `http-core`.

## Where It Fits

```text
tcp-client / reactor / stream source
  ↓
http1 / http2 / http3 parser
  ↓
http-core
  ↓
client, browser, proxy, cache, renderer
```

`http-core` deliberately contains **no wire parsing**. It is the shared target
for version-specific parsers.

## Concepts

### Header Lists, Not Maps

HTTP headers are often looked up like a dictionary:

```text
Content-Type → text/html
Content-Length → 45
```

But the wire format is really an **ordered list**:

```text
Set-Cookie: session=abc
Set-Cookie: theme=dark
Warning: stale
```

So `http-core` stores headers as an ordered list of `Header` values rather than
as a map. This preserves:

- original order
- duplicate fields
- exact spelling of header names

Convenience helpers provide case-insensitive lookup on top.

### Body Kind

The parser for an HTTP message head usually does not return the body itself. It
returns instructions for how the caller should read or interpret the body.

That instruction is represented by `BodyKind`:

```text
None                    → no body is expected
ContentLength(45)       → read exactly 45 bytes
UntilEof                → read until the peer closes the stream
Chunked                 → decode HTTP/1.1 chunked framing
```

`BodyKind` belongs in the shared core because the idea survives across parser
implementations even when the exact rules differ by version.

### Request Head vs Response Head

`http-core` models **heads**, not full messages:

- `RequestHead` contains method, target, version, and headers
- `ResponseHead` contains version, status, reason, and headers

The head is what the parser can determine before payload decoding and content
handling begin.

### Content-Type Helper

Applications frequently need to answer:

- Is this HTML?
- Which charset should decode this body?

So `http-core` provides a helper that splits:

```text
text/html; charset=utf-8
```

into:

- media type: `text/html`
- charset: `utf-8`

This is a semantic helper, not a wire parser.

## Public API

```rust
pub struct Header {
    pub name: String,
    pub value: String,
}

pub struct HttpVersion {
    pub major: u16,
    pub minor: u16,
}

pub enum BodyKind {
    None,
    ContentLength(usize),
    UntilEof,
    Chunked,
}

pub struct RequestHead {
    pub method: String,
    pub target: String,
    pub version: HttpVersion,
    pub headers: Vec<Header>,
}

pub struct ResponseHead {
    pub version: HttpVersion,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<Header>,
}

pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a str>;
pub fn parse_content_length(headers: &[Header]) -> Option<usize>;
pub fn parse_content_type(headers: &[Header]) -> Option<(String, Option<String>)>;
```

Equivalent APIs should exist in all supported languages.

## Design Decisions

**Why a struct for `HttpVersion` instead of an enum?**

Because every language can model `{ major, minor }` naturally. It is also
future-proof: `HTTP/9.1` is still representable even if we have never heard of
it before.

**Why keep `BodyKind::Chunked` if the first parser target is HTTP/1.0?**

Because the type belongs to the conceptual model even if the first parser only
returns it in a limited set of cases. Adding it later would force API changes in
every language.

**Why store `reason` on responses?**

It is useful for logging and debugging, even though applications should make
decisions primarily on the numeric status code.

## Testing Strategy

1. Header lookup is ASCII case-insensitive.
2. Duplicate headers are preserved and the first match is returned by lookup.
3. `Content-Length` parsing succeeds on valid integers.
4. `Content-Length` parsing rejects malformed integers.
5. `Content-Type` parsing extracts media type and charset.
6. `Content-Type` parsing tolerates extra parameters beyond charset.
7. `HttpVersion` string rendering stays stable.
8. `BodyKind` constructors or tagged values compare correctly.

## Scope

**In scope:**

- shared HTTP request/response head types
- header lookup helpers
- content length helper
- content type helper
- body framing kind representation

**Out of scope:**

- wire parsing
- socket I/O
- HTTP/1 start-line parsing
- HTTP/2 frame parsing
- body decoding
- HTML parsing

## Implementation Languages

This package will be implemented in:

- Python
- Go
- Ruby
- TypeScript
- Rust
- Elixir
- Perl
- Lua
- Swift
