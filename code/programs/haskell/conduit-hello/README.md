# conduit-hello — Haskell Conduit demo application (WEB18)

A minimal 8-route web application demonstrating the Haskell Conduit library.

## Routes

| Method | Path           | Description                               |
|--------|----------------|-------------------------------------------|
| GET    | /              | "Hello, World from Haskell Conduit!"      |
| GET    | /hello/:name   | Greet by name (path parameter)            |
| POST   | /echo          | Echo the request body                     |
| GET    | /search?q=...  | Echo a query parameter                    |
| GET    | /redirect      | 302 → /                                   |
| GET    | /halt          | 503 via `halt`                            |
| GET    | /down          | 503 from before-filter (handler not called) |
| GET    | /error         | 500 via on_error (handler throws)         |
| *      | (anything else)| 404 from custom notFound handler          |

The after-hook adds `X-Powered-By: Haskell Conduit` to every response.

## Running

```sh
# Build the Rust engine first
cargo build -p conduit-capi --release

# Run the server (port 8080 by default)
LIB_DIR=$(pwd)/target/release
cabal run conduit-hello --extra-lib-dirs="$LIB_DIR"
# or use the provided script:
sh tools/run-tests.sh
```

Set `PORT=<n>` to change the listening port.

## See also

- `code/packages/haskell/conduit/` — the library
- `code/specs/WEB18-conduit-haskell.md` — design specification
