# conduit-hello (C++)

A complete Sinatra-style demo built on the
[`Conduit`](../../../packages/cpp/conduit/README.md) C++ package.

## Run

```sh
sh tools/run.sh           # builds the C ABI + demo, runs the smoke test
./_build/conduit-hello    # then run the server (port 3000)
./_build/conduit-hello 8080
```

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
