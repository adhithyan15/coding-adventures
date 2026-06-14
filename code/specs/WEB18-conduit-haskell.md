# WEB18 — Conduit for Haskell (via `conduit-capi` C ABI + GHC FFI)

## Summary

Port the **Conduit** web framework to **Haskell**, using GHC's built-in C FFI
(`Foreign.C.*`, `Foreign.Ptr`, `Foreign.StablePtr`) — no third-party FFI library.

Haskell is the seventh and final consumer of the reusable `conduit-capi` C ABI
introduced in WEB12. Like C++, Go, C#, F#, and Dart before it, the Haskell port
binds directly to `conduit_capi.h` — no new Rust shim is required.

The key difference from previous ports is Haskell's lazy/pure runtime:
- Haskell handlers are pure-by-default Haskell functions lifted into `IO`.
- GHC `StablePtr` boxes a Haskell closure into a C-safe `void*` ctx; the matching
  `ctx_free` calls `freeStablePtr` to release it when the app/server is freed.
- GHC `FunPtr` (produced by `foreign import ccall "wrapper"`) wraps a Haskell
  function as a C function pointer callable by `conduit-capi`.
- `conduit_server_serve` blocks; it is imported with the `safe` calling convention
  so GHC's RTS can continue scheduling other green threads while C blocks.

No Dart-style async bridge is needed. GHC's threaded runtime (`-threaded`) allows
C code to call back into Haskell safely from any OS thread.

## Architecture

```
Haskell DSL (Conduit library)
    handlers: Request -> IO Response     ← pure Haskell IO actions
    │  StablePtr boxes the closure as a void* ctx
    │  FunPtr (from "wrapper" import) is the C-callable trampoline
    ▼
conduit-capi (Rust cdylib, extern "C")   ← THE reusable C ABI (WEB12+)
    conduit_app_* / conduit_server_* / conduit_request_* / conduit_response_*
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime → kqueue/epoll
```

### Closure boxing with StablePtr

GHC's garbage collector moves heap objects; a raw `Ptr ()` to a Haskell value
would become invalid as soon as the GC runs. `StablePtr` pins a Haskell value in
place and returns a C-safe pointer that remains valid until explicitly freed:

```haskell
-- Box a Haskell handler as a stable, GC-pinned pointer.
stablePtr <- newStablePtr (handler :: Request -> IO Response)
let ctx = castStablePtrToPtr stablePtr   -- :: Ptr ()

-- Inside the C-callable trampoline:
cHandler :: Ptr () -> Ptr CRequest -> IO (Ptr CResponse)
cHandler ctxPtr reqPtr = do
  fn  <- deRefStablePtr (castPtrToStablePtr ctxPtr)
  req <- peekRequest reqPtr              -- read the C request view into Haskell
  resp <- fn req `catch` \(e :: SomeException) -> do
            conduit_capi_report_error (show e)
            return nullPtr
  case resp of
    Nothing -> return nullPtr
    Just r  -> buildCResponse r

-- The ctx_free destructor frees the StablePtr when conduit-capi is done.
ctxFree :: Ptr () -> IO ()
ctxFree = freeStablePtr . castPtrToStablePtr
```

### FunPtr ("wrapper" import)

```haskell
-- Declare the Haskell-function-to-C-function-pointer wrapper.
-- GHC generates a tiny C-callable stub that dispatches into the GHC RTS.
foreign import ccall "wrapper"
  mkHandler :: (Ptr () -> Ptr CRequest -> IO (Ptr CResponse))
            -> IO (FunPtr (Ptr () -> Ptr CRequest -> IO (Ptr CResponse)))

foreign import ccall "wrapper"
  mkAfter :: (Ptr () -> Ptr CRequest -> Ptr CResponse -> IO (Ptr CResponse))
          -> IO (FunPtr (Ptr () -> Ptr CRequest -> Ptr CResponse -> IO (Ptr CResponse)))

foreign import ccall "wrapper"
  mkCtxFree :: (Ptr () -> IO ())
            -> IO (FunPtr (Ptr () -> IO ()))
```

`FunPtr` is allocated once per handler registration and lives until the C library
calls `ctx_free`, at which point we must also call `freeHaskellFunPtr`. We
co-locate `FunPtr` + `StablePtr` in a tiny struct boxed as the `ctx`:

```haskell
data HandlerBox = HandlerBox
  { hbFunPtr   :: FunPtr HandlerFn   -- the C-callable wrapper (freed in ctx_free)
  , hbStable   :: StablePtr Handler  -- the Haskell closure   (freed in ctx_free)
  }
-- The ctx is a StablePtr to the HandlerBox itself.
```

### Thread model

GHC's threaded runtime (`-threaded`) multiplexes Haskell green threads over OS
threads. FFI calls tagged `safe` release the GHC token so other Haskell threads
can run; `unsafe` calls hold the token (fast but blocks the RTS). Rules:

- `conduit_server_serve` → `safe` (blocks the calling OS thread for the server
  lifetime; must not hold the GHC token or no Haskell code can run).
- `conduit_server_stop`, `conduit_server_running`, `conduit_server_local_port` →
  `unsafe` (quick; never block).
- All `conduit_app_*`, `conduit_response_*`, `conduit_request_*` → `unsafe`.
- Callbacks from C into Haskell (via `FunPtr`) always enter a new GHC token; no
  annotation needed — this is how `"wrapper"` works.

Programs must link with `-threaded` (`ghc-options: -threaded` in the cabal file).

## File structure

### `code/packages/haskell/conduit/`

```
conduit.cabal
cabal.project          (references this package only; used for `cabal test`)
BUILD                  (# build-tool: deps=rust/conduit-capi\nsh tools/run-tests.sh)
BUILD_windows          (echo "Haskell conduit uses FFI against Rust cdylib — skipping on Windows")
README.md
CHANGELOG.md
required_capabilities.json
src/
  Conduit.hs           (re-exports Application, Request, Response, Server)
  Conduit/
    FFI.hs             (all foreign import ccall declarations + phantom types)
    App.hs             (Application newtype, route/filter registration, DSL)
    Request.hs         (Request type + accessor helpers)
    Response.hs        (Response type, builder helpers: html/json/text/redirect/halt)
    Server.hs          (Server type, serve/serveBackground/stop/localPort/running)
test/
  Spec.hs              (hspec entry point)
  ConduitSpec.hs       (library unit tests)
  ServerE2ESpec.hs     (server E2E over raw socket)
tools/
  run-tests.sh         (cargo build conduit-capi; EXTRA_LIB_DIR; cabal test)
```

### `code/programs/haskell/conduit-hello/`

```
conduit-hello.cabal
cabal.project          (references conduit-hello + conduit packages)
BUILD                  (# build-tool: deps=haskell/conduit,rust/conduit-capi\nsh tools/run-tests.sh)
BUILD_windows          (echo "skipping on Windows")
README.md
CHANGELOG.md
required_capabilities.json
src/
  Main.hs              (8-route hello-world app)
test/
  SmokeSpec.hs         (smoke tests: launch background, hit over socket, stop)
tools/
  run-tests.sh         (cargo build conduit-capi; cabal run + cabal test)
```

## Haskell API design

### Types

```haskell
-- Phantom-typed opaque pointers matching the C handles.
data CApp
data CServer
data CRequest
data CResponse

-- Haskell-facing types.
newtype Application = Application (Ptr CApp)
newtype Server      = Server      (Ptr CServer)

data Request = Request
  { reqMethod      :: !Text
  , reqPath        :: !Text
  , reqQueryString :: !Text
  , reqBody        :: !ByteString
  , reqContentType :: !Text
  , reqRemoteAddr  :: !Text
  , reqError       :: !Text         -- non-empty only in on_error handlers
  , _reqPtr        :: !(Ptr CRequest)  -- kept for param/query/header lookup
  }

data Response = Response
  { respStatus  :: !Word16
  , respBody    :: !ByteString
  , respHeaders :: ![(Text, Text)]
  }
```

### Application DSL

```haskell
newApplication :: IO Application

-- Route registration
get, post, put, delete, patch, options
  :: Application -> Text -> (Request -> IO Response) -> IO ()
addRoute
  :: Application -> Text -> Text -> (Request -> IO Response) -> IO ()

-- Filters
before :: Application -> (Request -> IO (Maybe Response)) -> IO ()
after  :: Application -> (Request -> Response -> IO Response) -> IO ()

-- Handlers
notFound    :: Application -> (Request -> IO Response) -> IO ()
onError     :: Application -> (Request -> IO Response) -> IO ()

-- Settings
setSetting  :: Application -> Text -> Text -> IO ()
getSetting  :: Application -> Text -> IO (Maybe Text)

-- Bind
bind :: Application -> Text -> Word16 -> IO Server  -- consumes app; throws on error
```

### Server

```haskell
serve           :: Server -> IO ()          -- blocks; 0 ok
serveBackground :: Server -> IO ()          -- returns immediately
stop            :: Server -> IO ()
localPort       :: Server -> IO Word16
running         :: Server -> IO Bool
freeServer      :: Server -> IO ()
```

### Response helpers

```haskell
respond   :: Word16 -> ByteString -> Response
html      :: Word16 -> Text -> Response
json      :: Word16 -> Text -> Response
text      :: Word16 -> Text -> Response
redirect  :: Word16 -> Text -> Response     -- throws if location contains CR/LF
halt      :: Word16 -> ByteString -> IO a   -- throws ConduitHalt
```

`ConduitHalt` is a Haskell exception: when a handler throws it, the trampoline
catches it and returns the embedded `Response` directly (same as Swift's `halt`).

## Request accessors

`reqParam`, `reqQuery`, `reqHeader` call the C accessors via the stored `_reqPtr`
(valid only during the handler call):

```haskell
reqParam  :: Request -> Text -> IO (Maybe Text)
reqQuery  :: Request -> Text -> IO (Maybe Text)
reqHeader :: Request -> Text -> IO (Maybe Text)
```

These are `IO` because they call back into C.

## Test plan (target 30+ tests)

### Unit tests (`ConduitSpec.hs`)

- Response: `html/json/text` set correct status + Content-Type header.
- Response: `redirect` rejects CR/LF in location (throws).
- Response: `halt` throws `ConduitHalt` with the right status.
- Application: `setSetting` / `getSetting` round-trip.
- Request: `peekRequest` correctly unmarshals method/path/queryString/body.

### Server E2E (`ServerE2ESpec.hs`)

All over raw socket on port 0 + `serveBackground`; stopped by `stop` in
`after_`/cleanup:

1. `GET /` → 200 "Hello, Haskell!"
2. `GET /hello/world` → 200 "Hello, world!"
3. `GET /hello/` (no name) → 404 custom not_found
4. `POST /echo` with body → 200 body echoed back
5. `GET /search?q=haskell` → 200 "You searched for: haskell"
6. `GET /redirect` → 302 + Location header
7. `GET /halt` → 503 (halt)
8. `GET /down` → 503 from before-filter
9. `GET /error` → 500 from on_error handler
10. `GET /missing` → 404 from notFound handler
11. Concurrent requests: 5 simultaneous GETs to `/` all 200 (-threaded RTS).

## Security

- Same audited boundary as all prior capi ports: header-injection defense, status
  clamping, UTF-8 validation, catch_unwind.
- `StablePtr` lifetime: freed in `ctx_free`; no double-free because `ctx_free` is
  called exactly once by conduit-capi.
- `FunPtr` lifetime: freed alongside the `StablePtr` in the same `ctx_free`.
- Callbacks catch all Haskell exceptions: a throwing handler reports the error via
  `conduit_capi_report_error` and returns `nullPtr` (→ on_error, not a crash).
- `reqParam`/`reqQuery`/`reqHeader` are safe to call only during the handler (the
  `_reqPtr` is a borrowed view); returning `Request` outside the callback is
  undefined. Documented, not enforced at the type level.

## Build

- `BUILD`: `# build-tool: deps=rust/conduit-capi` then `sh tools/run-tests.sh`
- `tools/run-tests.sh`:
  1. `cargo build -p conduit-capi --release -q` (from repo root)
  2. Detect OS → `libconduit_capi.{dylib,so}` path
  3. `cabal test --extra-lib-dirs="$LIB_DIR" --extra-include-dirs="$INCLUDE_DIR"`
- `conduit.cabal` extra-libraries: `conduit_capi`; `ghc-options: -threaded`
- `BUILD_windows`: echo skip (Windows cross-compile for the cdylib out of scope)
- Lessons: `cabal.project` must enumerate `packages: .` only (not a wildcard that
  picks up sibling packages — they have their own `cabal.project` files).
