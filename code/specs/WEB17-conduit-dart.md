# WEB17 — Dart Conduit Binding

> **Retroactive spec.** The Dart port (`code/packages/dart/conduit/`) shipped
> ahead of this document, so unlike the specs-first ports it is written to match
> the *as-built* implementation. It exists to restore the per-port spec-coverage
> invariant (every WEB port has a `WEBxx-*.md`) and is the source of truth going
> forward — future changes update this file alongside the code.

## Purpose

`coding_adventures_conduit` is a Sinatra/Express-style web framework for Dart 3,
built over the `conduit-capi` Rust cdylib (WEB12) — the same C ABI used by Swift
(WEB12), Go (WEB14), C# (WEB15), and F# (WEB16). It provides a chainable,
`Future`-friendly API idiomatic to Dart.

Unlike every other `conduit-capi` consumer, Dart cannot register native callbacks
that Rust may invoke from a background OS thread. WEB17 therefore introduces a
**second, Dart-specific Rust cdylib — `conduit-dart-bridge`** — that brokers
those callbacks safely onto the Dart isolate. This is the defining technical
characteristic of the port.

## Scope

- Package: `code/packages/dart/conduit/`
- Demo program: `code/programs/dart/conduit-hello/`
- Bridge crate: `code/packages/rust/conduit-dart-bridge/` (new Rust code for WEB17)
- Tests: ≥ 40 tests + a standalone FFI end-to-end test
- Language/runtime: Dart 3 (`dart:ffi`)

## Architecture

```
Your Dart code
    │
    ▼  dart:ffi (DynamicLibrary.open + NativeCallable.isolateLocal)
coding_adventures_conduit  (lib/conduit.dart → lib/src/*.dart)
    │
    ├─► conduit-capi        C ABI: conduit_app_*, conduit_server_*, conduit_request_*, conduit_response_*
    │     (Rust cdylib, WEB12 — shared, unchanged)
    │
    └─► conduit-dart-bridge  post+block channel for background-thread callbacks
          (Rust cdylib, NEW for WEB17)
                │
                ▼
            web-core (WEB00) + embeddable-http-server
```

The route/HTTP engine reuses **conduit-capi** verbatim — no changes to the shared
engine. The only new Rust code is `conduit-dart-bridge`. The Dart package's BUILD
declares `deps=rust/conduit-capi,rust/conduit-dart-bridge`.

## The cross-thread callback problem (and the bridge)

`conduit-capi` dispatches request handlers from a background OS thread spawned by
`serve_background()`. Dart's `NativeCallable.isolateLocal` is only valid on the
Dart isolate's own thread; invoking it from an independent OS thread crashes with
*"Cannot invoke native callback outside an isolate."* So Dart handlers cannot be
registered with conduit-capi directly the way Swift/Go/C#/F# closures are.

`conduit-dart-bridge` solves this with a thread-safe **post+block** channel:

1. Dart calls `conduit_dart_init(NativeApi.initializeApiDLData)` once to populate
   the Dart Embedder DL API pointer table (`Dart_PostCObject_DL`, …).
2. Dart creates a `RawReceivePort` and calls
   `conduit_dart_set_port(port.sendPort.nativePort)`.
3. Dart registers the **bridge's** handler/before/after/ctx_free function pointers
   (returned by `conduit_dart_*_fn()`) with conduit-capi — not `NativeCallable`
   trampolines. The opaque `ctx` is an integer ID for the Dart closure.
4. When conduit-capi calls a bridge handler from its Rust OS thread, the bridge:
   - allocates a response slot guarded by a `Condvar`,
   - posts a `List<int>` message to the Dart event loop via `Dart_PostCObject_DL`
     (safe from any thread),
   - blocks on the `Condvar`.
5. Dart's event loop fires `_onBridgeMessage`, looks up the closure by `ctx` ID,
   runs it, and calls `conduit_dart_complete(slotId, responsePtr)`.
6. `conduit_dart_complete` signals the `Condvar`, unblocking the Rust thread.

Because the Rust thread blocks until the Dart isolate returns, `serve()` must keep
the event loop free — it is therefore **`async`**: always `await server.serve()`
from an `async main()`.

### ARM64 / Apple Silicon note

`Dart_PostCObject_DL` is declared in `dart_api_dl.h` as a global function-pointer
*variable*, not a function. On ARM64 the bridge must call through the loaded
pointer value, not the symbol's data address — see `conduit-dart-bridge`'s README
for the dereferencing detail.

## Threading model

Dart runs on a single isolate event loop. Handlers always execute on that loop
(marshalled by the bridge), so user code is free of data races by construction.
`serveBackground()` starts the accept loop and returns immediately; `serve()`
starts it and completes only when `stop()` is called.

## Dart API surface

```dart
// ── Response ──────────────────────────────────────────────────────────────────
class Response {
  factory Response.html(String body, {int status = 200});
  factory Response.json(String body, {int status = 200});
  factory Response.text(String body, {int status = 200});
  factory Response.redirect(String location, {int status = 302}); // CR/LF-guarded
  factory Response.respond(int status, String body,
      {List<(String, String)> headers = const []});
  Response withStatus(int status);
  Response withHeader(String name, String value);
}

// `halt` short-circuits a handler by throwing; the trampoline returns its Response.
class HaltException implements Exception { final Response response; }

// ── Request ───────────────────────────────────────────────────────────────────
class Request {
  String get method;        // "GET", "POST", …
  String get path;          // "/api/users/42"
  String get queryString;   // "q=hello&page=2"
  String get contentType;   // "application/json"
  String get remoteAddr;    // "127.0.0.1:54321"
  String? param(String name);   // named route parameter
  String? query(String name);   // query-string value
  String? header(String name);  // request header
  Uint8List body();             // raw bytes
  String bodyString();          // UTF-8
}

// ── Application (chainable builder; every method returns the Application) ──────
class Application {
  Application set(String key, String value);
  String? getSetting(String key);
  Application get(String pattern, Response Function(Request) handler);
  Application post(String pattern, Response Function(Request) handler);
  Application put(String pattern, Response Function(Request) handler);
  Application delete(String pattern, Response Function(Request) handler);
  Application patch(String pattern, Response Function(Request) handler);
  Application route(String method, String pattern, Response Function(Request) handler);
  Application before(Response? Function(Request) filter);  // null → continue
  Application after(Response Function(Request, Response) hook);
  Application notFound(Response Function(Request) handler);
  Application onError(Response Function(Request) handler);
  Server bind(String host, int port);
  void dispose();
}

// ── Server ────────────────────────────────────────────────────────────────────
class Server {
  int get localPort;        // the actual bound port (use 0 to auto-assign)
  bool get isRunning;
  Future<void> serve();     // async: start + await until stop()
  void serveBackground();   // start on a background thread, return immediately
  void stop();
  void dispose();           // free native resources
}
```

## Security requirements

Carries the security properties common to the `conduit-capi` ports (WEB12–WEB16):

| Property | Implementation |
|---|---|
| No TOCTOU in lib load | Open `CONDUIT_CAPI_PATH` / `CONDUIT_DART_BRIDGE_PATH` directly — no `File.existsSync` pre-check |
| Null/stale ctx guards | Bridge slot + closure lookups are by integer ID; an unknown ID returns a safe 500 / pass-through |
| Bounds check before allocation | `req.body()` length is validated before allocating the `Uint8List` |
| Sanitised bind-failure error | Public exception is generic; the raw Rust error goes to stderr only |
| Header-injection guard | `Response.redirect` (and header setters) reject CR/LF in values |
| No exception leak to client | Handler exceptions route to `onError`; raw messages are not echoed in responses |
| Status-code range validation | Status validated before the native call |

## Tests

| Group | Where | Notes |
|---|---|---|
| Unit + integration | `test/conduit_test.dart` | ~42 tests: Response/Request builders, Application config, routing, before/after, halt/redirect, and live E2E via `serveBackground()` + `HttpClient` |
| Standalone FFI E2E | `test/standalone_test.dart` | full `bind` → `serveBackground` → `HttpClient` round-trip exercising the bridge end to end |

`tools/run-tests.sh` builds `conduit-capi` + `conduit-dart-bridge` in release,
exports `CONDUIT_CAPI_PATH` / `CONDUIT_DART_BRIDGE_PATH`, and runs `dart test`.

## Files

```
code/specs/WEB17-conduit-dart.md                     (this file)
code/packages/rust/conduit-dart-bridge/              (new Rust cdylib for WEB17)
  BUILD  CHANGELOG.md  Cargo.toml  README.md  build.rs  src/  dart/
code/packages/dart/conduit/
  BUILD  BUILD_windows  CHANGELOG.md  README.md  pubspec.yaml
  required_capabilities.json  tools/run-tests.sh
  lib/conduit.dart
  lib/src/{application,server,request,response,ffi,trampolines}.dart
  test/{conduit_test,standalone_test}.dart
code/programs/dart/conduit-hello/
  BUILD  BUILD_windows  CHANGELOG.md  README.md  pubspec.yaml
  required_capabilities.json  tools/run-tests.sh
  bin/  test/
```

## Relationship to the other ports

WEB17 is the seventh consumer of `conduit-capi` (after Swift, C++, Go, C#, F#, and
the shared Rust facade). It is the only one that required an *additional* native
shim (`conduit-dart-bridge`) because of Dart's isolate-affine FFI callbacks; the
Haskell port (WEB18) followed and, like the others, needed no such shim (GHC's
`safe` FFI lets Rust call Haskell from any OS thread directly).
