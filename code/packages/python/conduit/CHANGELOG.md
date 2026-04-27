# Changelog

## Unreleased

### Phase 1 — Python Flask-like Conduit (WEB03)

First Python port of the Conduit framework, mirroring the Ruby implementation
(WEB02) in a Flask-style Python API.

#### Package structure

- `src/coding_adventures/conduit/` — Python DSL layer
  - `__init__.py` — public exports: `Conduit`, `HaltException`, `NativeServer`
  - `application.py` — `Conduit` class with route decorators (`@app.get`, `@app.post`, …),
    filter decorators (`@app.before_request`, `@app.after_request`), special handlers
    (`@app.not_found`, `@app.error_handler`), and `app.settings` dict
  - `application.py` — `_flask_to_rust_pattern()` converts `<param>` → `:param` for
    the Rust `web-core` router
  - `halt_exception.py` — `HaltException(status, body, headers)` — Python equivalent
    of Ruby's `HaltError`; raised by response helpers, caught before returning to Rust
  - `handler_context.py` — `HandlerContext` with `json/html/text/halt/redirect` helpers
    and `__getattr__` delegation to `Request` so `ctx.path` and `ctx.params` work directly
  - `request.py` — `Request` wrapping the `env` dict; properties for `method`, `path`,
    `params`, `query_params`, `headers`, `body`; memoized `json()` and `form()` body parsers;
    `json()` raises `HaltException(400)` on invalid JSON instead of leaking parse errors
  - `server.py` — `NativeServer`: capsule wrapper + Python-side dispatch methods
    (`native_dispatch_route`, `native_run_before_filters`, `native_run_after_filters`,
    `native_run_not_found`, `native_run_error_handler`) that Rust calls back into

#### Rust extension (`ext/conduit_native/`)

- Zero-dependency cdylib using `python-bridge` (no PyO3, no pyo3-ffi)
- Exports `conduit_native` Python module with functions: `server_new`, `server_serve`,
  `server_stop`, `server_dispose`, `server_running`, `server_local_host`, `server_local_port`
- GIL management declared inline (`PyEval_SaveThread/RestoreThread` around `serve()`;
  `PyGILState_Ensure/Release` around every Python callout from web-core threads)
- `server_new(owner, app, host, port, max_connections) → PyCapsule` — registers all
  routes and hooks with `web-core`, returns a capsule wrapping heap-allocated server state
- Before-routing hook fires for every request before route lookup (matches Sinatra semantics)
- Error handler dispatch: Python exceptions from route handlers are caught and routed to
  `native_run_error_handler` while still holding the GIL (same pattern as Ruby WEB02)
- Response protocol: Python returns `None` (no override) or `[status, [[name,val],...], body]`
- PyCapsule destructor calls `Py_DecRef` on the owner PyObject and drops the server

#### Tests

- `tests/test_halt_exception.py` — 17 tests: construction, `to_response()`, `_normalize_headers`
- `tests/test_request.py` — 29 tests: all properties, JSON/form parsing, error cases
- `tests/test_handler_context.py` — 27 tests: all response helpers, status codes, delegation
- `tests/test_application.py` — 28 tests: pattern conversion, all HTTP verbs, filters, settings
- `tests/test_server_dispatch.py` — 26 tests: all 5 dispatch methods, filter ordering, halt propagation
- **Total: 127 unit tests** (all pure-Python, no Rust required)

#### Security

- `ctx.redirect()` documents CWE-601 (open redirect) risk with a security comment
- `request.json()` returns `HaltException(400)` on invalid JSON to prevent parser errors
  from leaking internal stack details in error responses
