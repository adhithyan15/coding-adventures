# conduit-hello (Lua)

A runnable demo of the [Conduit](../../../packages/lua/conduit) web framework for
Lua. It exercises every feature of the Conduit Lua DSL on the shared Rust
`web-core` engine: routing with path params, JSON bodies, before/after filters,
`halt`, `redirect`, and custom not-found / error handlers.

## Routes

| Method | Path           | Behaviour                                    |
|--------|----------------|----------------------------------------------|
| GET    | `/`            | HTML greeting                                |
| GET    | `/hello/:name` | JSON `{ message = "Hello <name>" }`           |
| POST   | `/echo`        | Echoes the JSON request body                 |
| GET    | `/redirect`    | `301` redirect to `/`                         |
| GET    | `/halt`        | `403` via `halt()`                            |
| GET    | `/down`        | `503` via a `before` filter (route unreached) |
| GET    | `/error`       | `500` via a custom `error_handler`            |
| *any*  | (unmatched)    | `404` via a custom `not_found` handler        |

## Run it

```sh
lua hello.lua
```

Then, in another terminal:

```sh
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/Adhithya
curl -X POST http://127.0.0.1:3000/echo \
     -H 'Content-Type: application/json' -d '{"ping":"pong"}'
curl -i http://127.0.0.1:3000/redirect
curl http://127.0.0.1:3000/halt
curl http://127.0.0.1:3000/down
curl http://127.0.0.1:3000/error
curl http://127.0.0.1:3000/missing
```

## How it's structured

`hello.lua` exposes a `build_app()` factory that returns a configured
`conduit.Application` without starting a server, plus a `serve()` helper. When
run directly (`lua hello.lua`) it serves in the foreground on port 3000; when
loaded as a module (by the tests) it only returns the factory. This is what lets
the test suite construct the same app and drive it over a background server.

## Tests

`tests/test_hello.lua` is an end-to-end suite (busted): it loads `build_app()`,
serves it on an ephemeral port in the background, and hits every route over real
HTTP via `luasocket`, asserting status codes and bodies. If `luasocket` is not
installed the E2E tests are skipped (pending), mirroring the conduit library's
own server tests.

```sh
cd tests && busted . --pattern=test_
```

The `BUILD` file compiles the conduit package's native extension first, then runs
these tests.
