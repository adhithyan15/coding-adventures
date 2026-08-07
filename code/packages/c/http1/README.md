# http1 (C)

**CCPP02 port campaign — bucket A (pure-ISO).** Parses HTTP/1.1 request and
response **heads** — the start line, the header lines, and the blank line that
ends them — and tells the caller where the body starts and how to frame it. The C
port of the Rust `http1` crate, a pure-ISO crate that needs no OS, so it rides the
`iso-harness` (links nothing, strict-conformance flags on).

```c
Http1ParsedRequestHead p;
if (http1_parse_request_head(
        (const unsigned char *)"POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello",
        49, &p) == HTTP1_OK) {
    /* p.head.method == "POST", p.head.target == "/submit",
       p.head.version == {1,1}, p.head.nheaders == 1,
       p.body_offset == 44, p.body_kind == HTTP_BODY_CONTENT_LENGTH,
       p.body_length == 5 */
    http1_parsed_request_free(&p);
}
```

| Function | Purpose |
|----------|---------|
| `http1_parse_request_head` | bytes → parsed request head (method/target/version/headers + body framing) |
| `http1_parse_response_head` | bytes → parsed response head (version/status/reason/headers + body framing) |
| `http1_parsed_request_free` / `http1_parsed_response_free` | release a parsed head |
| `http1_request_summary` / `http1_response_summary` | redacted, log-safe summaries |

## Composes `c/http-core`

The head vocabulary — `HttpVersion`, `HttpHeader`, `HttpRequestHead`,
`HttpResponseHead`, `HttpBodyKind` — comes from the pure-ISO
[`http-core`](../http-core) package (whose source `run.sh` compiles in; nothing is
linked). http-core provides the *shapes* and validators; this crate is the *wire
parser* that produces them.

## Body framing

- **Request:** chunked `Transfer-Encoding` → `HTTP_BODY_CHUNKED`; a positive
  `Content-Length` → `HTTP_BODY_CONTENT_LENGTH`; otherwise `HTTP_BODY_NONE`.
- **Response:** a 1xx / 204 / 304 status forces `HTTP_BODY_NONE` (overriding any
  header); else chunked → `HTTP_BODY_CHUNKED`; else a positive `Content-Length` →
  `HTTP_BODY_CONTENT_LENGTH`; a zero one → `HTTP_BODY_NONE`; absent →
  `HTTP_BODY_UNTIL_EOF`.

`HttpBodyKind` is a tag only (unlike Rust's `ContentLength(usize)`), so the length
is carried separately in `body_length`, meaningful exactly when
`body_kind == HTTP_BODY_CONTENT_LENGTH`.

## Faithfulness & safety notes

- **Ownership.** http-core's heads *borrow* their strings, so each parsed head
  OWNS the backing storage (method/target/reason and every header name/value,
  copied out of the input) and points its `head` fields into it; the `*_free`
  releases all of it. Do not free `head`'s fields yourself.
- **Line handling.** Leading blank lines are skipped; a line ends at `\n` (a
  trailing `\r` is dropped), so both CRLF and bare-LF heads parse; the first blank
  line ends the head and its end is `body_offset`. Running out of bytes before the
  blank line is `HTTP1_ERR_INCOMPLETE_HEAD`.
- **Parsing.** The start line is exactly three whitespace tokens for a request
  (method/target/version), and version + status + optional reason (joined with
  single spaces) for a response. A header splits at its first `:`; the name is
  trimmed of all whitespace (empty → error), the value of spaces/tabs only.
  Content-Length must be a valid integer; the status a valid `u16`; the version a
  valid `HTTP/x.y`. Head lines are UTF-8-validated (`HTTP1_ERR_INVALID_HEAD_ENCODING`).
- **Untrusted input.** The decoder parses attacker-controlled bytes; every read is
  bounds-checked and every size is `size_t`-overflow guarded. An adversarial
  review confirmed no out-of-bounds access, overflow, or leak.

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/http1
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 59 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
http1/
├── include/http1/http1.h   # public API
├── src/http1.c              # the parser — one pure-ISO source
├── tests/http1_test.c       # the Rust tests + chunked / incomplete / NULL paths
├── tools/run.sh  · run.ps1    # build via iso-harness (+ http-core)
├── BUILD  · BUILD_windows     # deps: c/iso-harness c/http-core
└── .gitignore
```
