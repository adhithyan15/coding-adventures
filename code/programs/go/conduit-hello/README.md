# conduit-hello (Go)

A demonstration web application built with the Go Conduit framework (WEB14).
Shows routes, before-filters, after-hooks, custom error/not-found handlers,
application settings, and the `Halt` escape hatch.

## Running

```sh
# Build the Rust static lib first (one-time).
cd ../../packages/rust/conduit-capi && cargo build --release

# Run the demo.
cd code/programs/go/conduit-hello
CGO_ENABLED=1 go run .
```

Then visit <http://127.0.0.1:3000/> in a browser or with curl:

```sh
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/world
curl -X POST http://127.0.0.1:3000/api/echo -d "ping" -H "Content-Type: text/plain"
curl "http://127.0.0.1:3000/api/query?msg=hello"
curl http://127.0.0.1:3000/api/halt
curl http://127.0.0.1:3000/api/panic
```

## Routes

| Method | Path               | What it does                                  |
|--------|--------------------|-----------------------------------------------|
| GET    | `/`                | HTML home page with links                     |
| GET    | `/hello/:name`     | JSON greeting with route parameter            |
| POST   | `/api/echo`        | Echoes the request body with the same Content-Type |
| GET    | `/api/query?msg=x` | Returns the `msg` query parameter as JSON     |
| GET    | `/api/panic`       | Triggers the OnError handler                  |
| GET    | `/api/halt`        | Demonstrates `conduit.Halt(418, …)`           |
| GET    | `/maintenance`     | Intercepted by the before-filter (503)        |

## Smoke test

```sh
CGO_ENABLED=1 go test ./... -v
```

Runs 8 subtests against a live background server on a random port with a
30-second watchdog. All routes above are covered.
