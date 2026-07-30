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
- `parseResponseHead` takes a `ResponseContext` containing the corresponding
  case-sensitive request method and HTTP version, parses an exact three-digit
  status code and ordered headers, and applies HEAD/CONNECT/transfer-version
  response semantics.
- `ParsedRequestHead` and `ParsedResponseHead` carry the semantic head, exact
  byte offset immediately after the terminating blank line, and body framing.
- `ParsedResponseHead.responseSwitchesProtocol` identifies a successful
  CONNECT tunnel transition.
- `Http1ParseError` distinguishes incomplete heads, start-line, version,
  status, header, framing, and resource-limit failures without retaining raw
  wire text.

Framing fails closed when `Transfer-Encoding` and `Content-Length` coexist.
Request transfer coding requires HTTP/1.1 and exactly one final `chunked`
coding; parameterized transfer codings are rejected rather than normalized.
Duplicate and comma-coalesced content lengths are accepted only when every
bounded decimal value agrees. Responses reject transfer coding when either
side uses HTTP/1.0 before applying HEAD, successful CONNECT, 1xx, 204, and 304
semantics or ordinary chunked, content-length, and EOF framing.

Decimal status and content-length parsing is bounded. Signed, malformed, and
overflowing values fail without constructing attacker-sized arbitrary-precision
integers.

Before decoding bytes to Haskell `String`, the parser caps a head at 65,536
bytes, a line at 8,192 bytes, the field section at 100 lines, and transfer
coding lists at 16 elements. Field names use strict RFC token bytes, whitespace
before a colon and obsolete folding are rejected, and structural start-line
delimiters are exact single spaces. Status lines require the reason delimiter
even when the reason phrase is empty.

## Example

```haskell
import CodingAdventures.Http1
import CodingAdventures.HttpCore (HttpVersion (..))
import qualified Data.ByteString.Char8 as Bytes

example :: Either Http1ParseError ParsedResponseHead
example =
  parseResponseHead
    ResponseContext
      { contextRequestMethod = "GET"
      , contextRequestVersion = HttpVersion 1 1
      }
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

The current suite has 28 Hspec examples and measures 97% expression and 90%
alternative coverage with GHC 9.4.8.
