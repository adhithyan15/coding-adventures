# Changelog — Conduit (C++)

## [0.1.1] - 2026-10-23

### Fixed

- **Use-after-free after `bind()`.** `bind()` marked the `Application`
  consumed but registration methods ignored the flag and kept writing through
  the stale `app_` pointer, which `conduit_server_bind` had already freed (it
  takes the app on success *and* on failure). `bind()` now nulls `app_` before
  the native call, and every method goes through a private `live()` check that
  throws `std::logic_error` on a bound or moved-from `Application`.
- `live()` is called in its own statement before each `new Handler(...)`:
  C++ leaves function-argument evaluation order unspecified, so folding it into
  the argument list could allocate the closure first and leak it on the throw.
- New tests: `application_is_unusable_after_bind`,
  `application_is_unusable_after_failed_bind`,
  `moved_from_application_is_unusable`.

## [0.1.0] - 2026-06-13

### Added — WEB13 C++ Conduit port

- A header-only Sinatra/Express-style web framework for C++ over the Rust
  web-core engine, via the reusable `conduit-capi` C ABI (the second consumer
  after Swift). No third-party deps.
- **DSL**: `Application` with `get/post/put/del/patch`, `route`, `before`,
  `after` (transforming), `notFound`, `onError`, `set`/`getSetting`, `bind` —
  all chainable. Handlers are `std::function<Response(const Request&)>`.
- **Response** helpers `html/json/text/respond/redirect`; `redirect` throws on
  CR/LF. **halt(...)** for Sinatra-style non-local exits (throws `Halt`).
- **Request**: `method/path/queryString/body/contentType/remoteAddr/error`,
  `param/query/header` (returning `std::optional`).
- **Server**: `serve` (foreground), `serveBackground`, `stop`, `localPort`,
  `running` — RAII-owned.
- Closures are heap-boxed and freed via a `ctx_free` trampoline; trampolines have
  C linkage (so function-pointer types match the ABI exactly) and catch all C++
  exceptions so none unwind across the C boundary.

### Tests

16 tests (zero-dependency harness): response helpers incl. native round-trip and
CR/LF guard; Application DSL/settings/bind; a full end-to-end server driven by a
POSIX-socket HTTP/1.0 client with a watchdog thread. `tools/run-tests.sh` builds
the C ABI, queries the platform's `native-static-libs`, and links each test.

### Security

Header-injection defense, status clamping, and UTF-8 validation are inherited
from `conduit-capi` (audited once for all C-ABI ports).
