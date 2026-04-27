# Changelog

## Unreleased

### Phase 3 — Full Sinatra demo (WEB02)

- **8 routes** exercising every Phase 3 DSL feature: `GET /`, `GET /hello/:name`,
  `POST /echo`, `GET /redirect`, `GET /halt`, `GET /down`, `GET /error`, and any
  unmatched path handled by a custom not-found block.
- **`before` filter** — blocks `GET /down` with `503 Under maintenance` before any
  route lookup (fires for every request, including unmatched paths).
- **`after` filter** — logs every request method and path to stdout.
- **`json` helper** — `GET /hello/:name` returns `{ message:, app: }` JSON.
- **`html` helper** — `GET /` returns an HTML greeting page.
- **`halt` helper** — `GET /halt` returns `403 Forbidden` unconditionally.
- **`redirect` helper** — `GET /redirect` returns `301` to `/`.
- **`not_found` handler** — custom `404` HTML page with the unmatched path.
- **`error` handler** — `GET /error` raises intentionally; handler returns `500` JSON
  with error detail.
- **`set :app_name`** — app name printed at startup via `app.settings[:app_name]`.
- **9 tests** (up from 3); coverage includes all routes, before filter, custom
  not-found, and custom error handler.

### Phase 2

- Added `conduit-hello` demo program.
- `GET /` route returning `"Hello from Conduit!"`.
- `GET /hello/:name` route returning `"Hello <name>"` — demonstrates Sinatra-style
  named route parameters resolved by the Rust `web-core` router.
- `BUILD` file for the project build tool.
- `README.md` explaining the stack and how to run.
