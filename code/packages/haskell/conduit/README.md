# conduit — Haskell Conduit Web Framework (WEB18)

A Sinatra/Express-style web framework for Haskell, backed by the Rust
`web-core` engine via the reusable `conduit-capi` C ABI.

## What is this?

Conduit is a family of web framework ports (WEB08–WEB18) that all share a
single audited Rust engine (`web-core`) and C ABI (`conduit-capi`).  This
package is the Haskell port, using GHC's built-in C FFI:

- **No third-party FFI library** — only `Foreign.C.*`, `Foreign.Ptr`,
  `Foreign.StablePtr`, and `Foreign.ForeignPtr` from `base`.
- **StablePtr boxing** — Haskell closures are pinned in the GHC heap with
  `StablePtr` so the GC can't move them while C holds a reference.
- **FunPtr adjustors** — GHC generates C-callable stubs (`"wrapper"` imports)
  that re-enter the Haskell runtime when called from C.
- **Safe FFI for blocking calls** — `conduit_server_serve` is imported `safe`
  so other Haskell green threads keep running while the server blocks.

## Quick start

```haskell
import Conduit

main :: IO ()
main = do
  app <- newApplication

  get app "/" $ \_ ->
    return (html 200 "<h1>Hello from Haskell!</h1>")

  get app "/hello/:name" $ \req -> do
    mname <- reqParam req "name"
    case mname of
      Nothing   -> return (html 404 "No name")
      Just name -> return (html 200 ("<p>Hello, " <> name <> "!</p>"))

  post app "/echo" $ \req ->
    return (respond 200 (reqBody req))

  before app $ \req ->
    if reqPath req == "/maintenance"
      then return (Just (html 503 "Down for maintenance"))
      else return Nothing

  notFound app $ \_ ->
    return (html 404 "Not found")

  srv <- bind app "0.0.0.0" 8080
  serve srv
```

## Linking

The library links against `conduit_capi` (the Rust cdylib).  Build
`conduit-capi` first with `cargo build -p conduit-capi --release`, then pass
`--extra-lib-dirs` to cabal:

```sh
cargo build -p conduit-capi --release
cabal build --extra-lib-dirs="$(pwd)/target/release"
```

The `tools/run-tests.sh` script does this automatically.

## API summary

| Function           | Description                                      |
|--------------------|--------------------------------------------------|
| `newApplication`   | Create a new application                         |
| `get/post/put/...` | Register a route handler                         |
| `before`           | Register a before-filter (Nothing = continue)    |
| `after`            | Register an after-hook (transforms response)     |
| `notFound`         | Override the 404 handler                         |
| `onError`          | Override the 500 error handler                   |
| `setSetting`       | Store a named setting                            |
| `getSetting`       | Retrieve a named setting                         |
| `bind`             | Bind host:port, return a Server                  |
| `serve`            | Serve on the calling thread (blocks)             |
| `serveBackground`  | Serve on a background thread (returns instantly) |
| `stop`             | Signal the server to stop                        |
| `localPort`        | Query the actual bound port                      |

## Architecture

```
Your Haskell handlers (Request -> IO Response)
         │ StablePtr pins the closure; FunPtr wraps it as a C-callable stub
         ▼
conduit-capi  (Rust cdylib, the shared C ABI — WEB12+)
         │ conduit_app_* / conduit_server_* / conduit_request_* / conduit_response_*
         ▼
conduit (WEB08 Rust facade) → web-core → embeddable-http-server → kqueue/epoll
```

## See also

- `code/specs/WEB18-conduit-haskell.md` — full design specification
- `code/packages/rust/conduit-capi/` — the shared C ABI
- `code/programs/haskell/conduit-hello/` — demo application
