# Changelog

## Unreleased

### Phase 1 — Full Flask-like demo (WEB03)

- **8 routes** exercising every WEB03 DSL feature: `GET /`, `GET /hello/<name>`,
  `POST /echo`, `GET /redirect`, `GET /halt`, `GET /down`, `GET /error`, and any
  unmatched path handled by a custom not_found block.
- **`@app.before_request` filter** — blocks `GET /down` with `503 Under maintenance`
  before any route lookup (fires for every request, including unmatched paths).
- **`@app.after_request` filter** — logs every request method and path to stdout.
- **`ctx.json` helper** — `GET /hello/<name>` returns `{ message:, app: }` JSON.
- **`ctx.html` helper** — `GET /` returns an HTML greeting page.
- **`ctx.halt` helper** — `GET /halt` returns `403 Forbidden` unconditionally.
- **`ctx.redirect` helper** — `GET /redirect` returns `301` to `/`.
- **`@app.not_found` handler** — custom `404` HTML page with the unmatched path.
- **`@app.error_handler`** — `GET /error` raises intentionally; handler returns
  `500` JSON with error detail.
- **`app.settings["app_name"]`** — app name printed at startup and in responses.
- **38 E2E tests** covering all 8 routes plus settings, before-filter for
  unmatched paths, redirect location header, and JSON echo parsing.
