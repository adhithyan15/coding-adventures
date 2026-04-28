# Changelog — coding-adventures-conduit

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

**WEB05 — TypeScript/Node.js port of the Conduit web framework.**

This is the third language port of the Conduit framework (after Ruby/Sinatra
and Lua), targeting Node.js via N-API.

#### TypeScript DSL layer (`src/`)

- **`Application`** — route and filter registry with Express-style DSL:
  `get()`, `post()`, `put()`, `delete()`, `patch()`, `before()`, `after()`,
  `notFound()`, `onError()`, `set()` / `getSetting()`.  Supports method
  chaining throughout.

- **`Server`** — binds an `Application` to a TCP socket via the Rust cdylib.
  `serve()` (blocking) and `serveBackground()` (non-blocking, for tests).
  `stop()`, `localPort`, `running` properties.

- **`Request`** — wraps the CGI-style env map produced by the Rust side.
  Exposes `method`, `path`, `queryString`, `params` (route captures),
  `queryParams`, `headers`, `bodyText`, `contentType`, `contentLength`.
  `json<T>()` and `form()` with lazy parsing and caching.

- **`HaltError`** — subclass of `Error` with `__conduit_halt = true` sentinel,
  `status`, `body`, and `haltHeaderPairs: [string, string][]`.  Thrown by
  `halt()` and `redirect()`.

- **`halt(status, body?, headers?)`** — short-circuit helper (throws
  `HaltError`).  Equivalent to Sinatra's `halt`.

- **`redirect(location, status?)`** — sets `Location` header and defaults
  to `302 Found`.

- **`html(body, status?)`**, **`json(value, status?)`**,
  **`text(body, status?)`**, **`respond(status, body, headers?)`** —
  response builder helpers.

#### Rust N-API cdylib (`ext/conduit_native_node/`)

- `conduit_native_node` Rust crate: N-API cdylib that drives `web-core`.
  Exports `newApp()` and `newServer()` to the Node.js module system.

- Per-request dispatch via `napi_threadsafe_function` (TSFN): requests arrive
  on a Rust background thread, are queued to the V8 main thread via TSFN,
  and the background thread blocks on a `Condvar` until JS resolves.

- `NativeApp` JS object: `addRoute()`, `addBefore()`, `addAfter()`,
  `setNotFound()`, `setErrorHandler()`, `setSetting()`, `getSetting()`.

- `NativeServer` JS object: `serve()`, `serveBackground()`, `stop()`,
  `localPort()`, `running()`.

- `HaltError` protocol: Rust detects `__conduit_halt = true` on the exception
  and converts it to a `WebResponse` instead of routing to the error handler.

- `build.rs`: platform-specific linker flags (`-undefined dynamic_lookup` on
  macOS; nothing needed on Linux).

#### `node-bridge` extensions (`packages/rust/node-bridge/`)

- New type aliases: `napi_ref`, `napi_threadsafe_function`, TSFN call/release
  mode constants, `napi_valuetype` enum constants.
- New `extern "C"` declarations: `napi_typeof`, `napi_is_array`,
  `napi_get_value_int32`, `napi_get_value_bool`, `napi_create_object`,
  `napi_get_named_property`, `napi_call_function`, `napi_new_instance`,
  `napi_is_exception_pending`, `napi_get_and_clear_last_exception`,
  `napi_create_reference`, `napi_get_reference_value`, `napi_delete_reference`,
  `napi_create_threadsafe_function`, `napi_acquire_threadsafe_function`,
  `napi_call_threadsafe_function`, `napi_release_threadsafe_function`,
  `napi_ref_threadsafe_function`, `napi_unref_threadsafe_function`.
- Safe wrappers: `value_type()`, `is_array()`, `i32_from_js()`,
  `bool_from_js()`, `object_new()`, `get_property()`, `call_function()`,
  `exception_pending()`, `clear_exception()`, `create_ref()`, `deref()`,
  `delete_ref()`, `tsfn_create()`, `tsfn_acquire()`, `tsfn_call()`,
  `tsfn_release()`, `tsfn_ref()`, `tsfn_unref()`.

#### Tests (`tests/`)

- **`halt_error.test.ts`** — 16 unit tests covering `HaltError`, `halt()`,
  `redirect()`.
- **`request.test.ts`** — 20 unit tests covering all `Request` fields,
  `json()`, `form()`, query string parsing.
- **`handler_context.test.ts`** — 16 unit tests covering `html()`, `json()`,
  `text()`, `respond()`, re-exported helpers.
- **`application.test.ts`** — 22 unit tests covering the full DSL.
- **`server.test.ts`** — 20 E2E tests covering the full request lifecycle
  via real TCP: routes, before filter, halt, redirect, not-found, error
  handler, query params, JSON body, headers, after filter, server metadata.

#### Spec

- `code/specs/WEB05-conduit-typescript.md` — full specification including
  threading model, TSFN design, HaltError protocol, BUILD, tests plan.

#### Demo program

- `code/programs/typescript/conduit-hello/` — 8-route demo mirroring the
  Ruby and Lua hello programs.
