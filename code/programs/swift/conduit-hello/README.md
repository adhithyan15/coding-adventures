# conduit-hello (Swift)

A complete Sinatra-style demo built on the
[`Conduit`](../../../packages/swift/conduit/README.md) Swift package.

## Run

```sh
swift run ConduitHello        # binds 127.0.0.1:3000
swift run ConduitHello 8080   # or choose a port
```

(The Conduit C ABI must be staged first — see the package README, or just run
this directory's `BUILD`.)

## Routes

| Route | Shows |
| ----- | ----- |
| `GET /`             | an HTML greeting (`html`) |
| `GET /hello/:name`  | a route param in JSON (`json`, `param`) |
| `POST /echo`        | body echo + content-type passthrough (`respond`) |
| `GET /search?q=`    | a query param (`query`) |
| `GET /redirect`     | a 301 to `/` (`redirect`) |
| `GET /halt`         | a 403 via `halt()` |
| `GET /down`         | a 503 from a `before` filter |
| `GET /error`        | a throwing handler → `onError` (500) |
| anything else       | the custom `notFound` (404) |

An `after` hook stamps `x-served-by: conduit-hello` on every response.

## Test

```sh
swift test
```
