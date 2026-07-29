# http-core (Haskell)

Pure Haskell shared HTTP message types and semantic helpers.

HTTP/1.x, HTTP/2, and HTTP/3 use different bytes on the wire, but application
code still consumes the same conceptual message head:

```text
version-specific parser
        |
        v
Header / HttpVersion / BodyKind
        |
        v
RequestHead or ResponseHead
```

This package is that shared middle layer. It performs no socket I/O, HTTP wire
parsing, percent decoding, or body decoding.

## API

- `Header` preserves arrival order, duplicate fields, values, and original name
  spelling. `findHeader` performs ASCII case-insensitive first-value lookup.
- `HttpVersion`, `parseHttpVersion`, and `renderHttpVersion` provide bounded,
  round-trippable `HTTP/x.y` markers.
- `BodyKind` describes no body, a fixed content length, EOF framing, or chunked
  framing.
- `RequestHead` and `ResponseHead` carry the semantic head fields and expose
  delegating header, content-length, and content-type helpers.
- `RequestTarget` splits a raw path, query, and fragment without decoding.
  `queryPairs` preserves duplicate keys and `queryValue` returns the first.
- `RoutePattern` matches literal and `:name` path segments. `matchTarget`
  ignores query strings and fragments.

## Example

```haskell
import CodingAdventures.HttpCore

request :: RequestHead
request =
  RequestHead
    { requestMethod = "GET"
    , requestTarget = "/devices/light-1?verbose=true"
    , requestVersion = HttpVersion 1 1
    , requestHeaders = [Header "Accept" "application/json"]
    }

route :: Maybe [(String, String)]
route = matchTarget (parseRoutePattern "/devices/:id") (requestTarget request)
-- Just [("id", "light-1")]
```

Content length accepts non-negative ASCII decimal values that fit in `Int`.
Content type parsing extracts a trimmed media type and the first
case-insensitive `charset` parameter.

## Development

```sh
cabal check
cabal test
cabal test --enable-coverage
```

The package depends only on `base`; Hspec is a test-only dependency.
