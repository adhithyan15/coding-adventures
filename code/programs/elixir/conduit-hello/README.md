# conduit-hello (Elixir)

Eight-route demo for the Elixir Conduit framework. Mirrors the
Ruby / Python / Lua / TypeScript demos in the same series.

## Run it

```sh
mix deps.get
mix run --no-halt
```

Then in another terminal:

```sh
curl http://127.0.0.1:3000/
curl http://127.0.0.1:3000/hello/Adhithya
curl -X POST -d 'ping=pong' http://127.0.0.1:3000/echo
curl -i http://127.0.0.1:3000/redirect
curl http://127.0.0.1:3000/halt
curl http://127.0.0.1:3000/down
curl http://127.0.0.1:3000/error
curl http://127.0.0.1:3000/missing
```

## Run tests

```sh
mix test
```

## What you'll learn

- Building an `Application` by chaining functional combinators.
- Using `before_filter` for cross-cutting concerns.
- `halt` and `redirect` as `throw`-based control flow on BEAM.
- Custom `not_found_handler` and `error_handler` fallbacks.
- Reading route params from `req.params` and headers from `req.headers`.
