# http1 (Haskell)

Pure Haskell HTTP/1 request and response head parsing.

`http1` owns the boundary between raw connection bytes and the version-neutral
message types from [`http-core`](../http-core):

```text
strict ByteString
       |
       v
start line + ordered headers
       |
       +--> RequestHead or ResponseHead
       +--> exact body offset
       `--> BodyKind
```

The package performs no socket I/O and does not consume, decode, or log body
bytes. Both CRLF and bare-LF head terminators are accepted.

## API

- `parseRequestHead` parses a method, raw target, bounded HTTP version, and
  ordered duplicate-preserving headers.
- `parseResponseHead` parses a bounded status code, normalized reason phrase,
  and ordered headers.
- `ParsedRequestHead` and `ParsedResponseHead` carry the semantic head, exact
  byte offset immediately after the terminating blank line, and body framing.
- `Http1ParseError` distinguishes incomplete heads, start-line, version,
  status, header, and content-length failures.

Request framing gives `Transfer-Encoding: chunked` precedence over
`Content-Length`; absent or zero lengths mean no body. Responses first make
1xx, 204, and 304 status codes bodyless, then apply chunked, content-length,
and EOF framing in that order.

Decimal status and content-length parsing is bounded. Signed, malformed, and
overflowing values fail without constructing attacker-sized arbitrary-precision
integers.

## Example

```haskell
import CodingAdventures.Http1
import qualified Data.ByteString.Char8 as Bytes

example :: Either Http1ParseError ParsedResponseHead
example =
  parseResponseHead
    (Bytes.pack "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody")
```

The successful result contains `ContentLength 4`, and its body offset points to
the `b` in `body`.

## Development

```sh
cabal check
cabal test
cabal test --enable-coverage
```

The package has no process, filesystem, network, environment, dynamic-loading,
or unsafe capabilities. Its only runtime dependencies are `base`, `bytestring`,
and the local pure `http-core` package.
