# HWEB08 — HTTP/1.0 request-line slice (1996)

This profile adds the first HTTP/1.0 request-line slice to the Python
historical-web reference. Its identifier is `http-1.0-request-line-1996`.
It consumes or emits one request line in memory; it does not parse headers,
request bodies, responses, sockets, URLs, or proxy routing. The existing
`http-0.9` functions retain their HWEB00 behavior.

## API

`historical_web` exports an immutable result record and two functions:

```python
parse_http10_request_line(wire: bytes) -> Http10RequestLine
encode_http10_request_line(method: str, target: str) -> bytes

@dataclass(frozen=True, slots=True)
class Http10RequestLine:
    method: str
    target: str
```

Only exact built-in `bytes` inputs are accepted by the parser, and exact
built-in `str` values by the encoder. A type mismatch raises `TypeError`.
Malformed wire or a value outside this profile raises `ValueError`. There is
no result/error wrapper, network I/O, implicit whitespace splitting, URL
normalization, percent decoding, or header parsing.

## Accepted line grammar

The complete input is exactly:

```text
method SP target SP "HTTP/1.0" CRLF
```

`SP` is one byte `0x20`; `CRLF` is exactly `0x0d 0x0a`. No leading/trailing
bytes, extra spaces, additional lines, headers, or body are accepted. The
method is exactly one of uppercase `GET`, `HEAD`, or `POST`. Method casing is
significant.

This parser follows the single-SP Request-Line generation grammar in section
5.1. RFC 1945 Appendix B separately recommends tolerant operational servers
accept any amount of SP or HT between fields. This educational intake profile
deliberately does not implement that tolerant parsing recommendation.

The request target is a nonempty absolute path: it starts with `/`, then may
contain only ASCII letters, digits, `/`, `.`, `_`, and `-`. The root path `/`
is valid. This intentionally narrow path alphabet excludes queries,
percent-escapes, parameters, fragments, absolute URIs, and other URI
characters even though RFC 1945 permits broader `Request-URI` forms. The
encoder and parser share this same closed subset. Accepted target bytes are
ASCII, so the returned `target` is the exact spelling from the wire.

Encoding validates both arguments against the closed method/path domains,
then emits exactly the canonical spacing, version literal, and CRLF shown
above. Parsing checks the complete wire form and returns a frozen
`Http10RequestLine` without normalizing either field.

## RFC 1945 context and limits

RFC 1945 section 5.1 defines a Request-Line as `Method SP Request-URI SP
HTTP-Version CRLF`, and says no CR or LF may occur except in the final CRLF.
Section 5.1.1 defines the case-sensitive methods GET, HEAD, POST, and
extension methods. Section 5.1.2 permits an `absoluteURI` or `abs_path`,
depending on the request context. This profile follows the framing and
case-sensitive core, but deliberately limits methods to those three
standardized methods and Request-URI to a simple ASCII `abs_path` subset.

HTTP/0.9's simple request is only `GET SP document-address CRLF`; it has no
version field and only GET. HWEB08 requires the explicit `HTTP/1.0` token and
supports HEAD and POST. It stops after the request line, so the following
header section from RFC 1945 section 5.2 is outside this profile. Compare
[HWEB00](HWEB00-historical-web.md) for the earlier wire contract and
[RFC 1945](https://www.rfc-editor.org/rfc/rfc1945) for the complete 1996
protocol. This is not a general HTTP/1.0 parser or a modern HTTP parser.
