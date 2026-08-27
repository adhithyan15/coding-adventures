# Changelog — mosaicbook-server

## Unreleased

### Security

- **Origin/Host validation on every route** (#13178, filed during #12027's
  drop-panel security review). Binding to `localhost` alone doesn't stop a
  page open in the developer's own browser from reaching this port — nor
  does it stop DNS rebinding, where an attacker-controlled domain resolves
  to `127.0.0.1` after the browser's same-origin checks already passed for
  that domain. Either way the `Host` header the server actually receives
  still names the untrusted domain, not `localhost`. `GET
  /api/degradations` made this a materially bigger deal than it already
  was: each request spawns a real `mosaic-compile` subprocess, turning a
  drive-by cross-origin `GET` into repeated build invocations.

  New `requireLocalOrigin` middleware (`security.go`) wraps the entire mux
  once, in `main.go`, rather than per-route — a route added later is
  protected automatically. Rejects any request whose `Host` doesn't name
  `localhost`/`127.0.0.1`/`::1` (with or without a port); and — when an
  `Origin` header is present, as it always is for a genuine cross-origin
  `fetch()` — additionally rejects one whose `Origin` doesn't either (this
  is what catches a same-machine page fetching this server's real
  `localhost:PORT` URL directly, which the `Host` check alone wouldn't
  catch, since `Host` is just where the request was addressed, not where it
  came from). A request with no `Origin` header at all — ordinary
  same-origin page loads, `curl` — passes on the `Host` check alone.

  **Security-review round 2 caught a real bypass in round 1's Origin-only
  design**, before this ever shipped: `GET /preview/{backend}/...` is
  specifically designed to be loaded via `<iframe src=...>` (that's how
  this tool's own browser shell displays a compiled component), and a
  cross-origin *navigation* — which is exactly what an iframe load is —
  carries no `Origin` header at all per the Fetch spec. A hostile page
  embedding `<iframe src="http://localhost:PORT/preview/...">` would have
  sailed straight past an Origin-only check: `Host` reads correctly as
  `localhost` (that really is the destination), `Origin` is simply absent,
  and the request reaches a handler that spawns a real subprocess per
  request — the exact DoS this fix exists to prevent, for the exact
  endpoint shaped to be embedded this way. Fixed by additionally checking
  `Sec-Fetch-Site`, sent by every modern browser for every request type
  including navigations (unlike `Origin`) and not spoofable by page JS:
  `Sec-Fetch-Site: cross-site` is rejected outright, independent of whether
  `Origin` is present.

  Existing handler-level tests are unaffected: they call `srv.mux.ServeHTTP`
  directly, bypassing this wrapper, which sits outside the mux specifically
  so it doesn't have to be threaded through every existing test's request
  construction.

  Verified against the real running server: a forged `Host` header → 403; a
  correct `Host` with a forged `Origin` → 403; a correct `Host` with no
  `Origin` → 200; a correct `Host` with a matching `Origin` → 200; a
  correct `Host`, no `Origin`, but `Sec-Fetch-Site: cross-site` (the
  iframe-navigation bypass shape) → 403; `Sec-Fetch-Site: same-origin` →
  200; a plain `curl` with no headers at all → 200 (unaffected).

### Added

- **Native-backend drop panel** (#12027, part 1 of 2 — see
  `code/specs/UI19-mosaicbook.md` §13). Five new backend tabs (`xaml`,
  `swiftui`, `qt`, `flutter`, `compose`). None has a render daemon yet — no
  daemon binary exists in the repo for any of them, `qt` included — so
  selecting one shows a "what got dropped" panel instead of the iframe,
  sourced from a new `GET /api/degradations/{backend}/{component_id}`
  endpoint. This needs no platform runtime: it shells out to
  `mosaic-compile pkg <package_root> --backend <backend> --output <tmp>
  --profile permissive`, reads back the `mosaic-degradations.json` that
  invocation always writes, and filters `degradations`/`styleDegradations`
  down to the selected component. Works for all five native backends on any
  development machine today, independent of and ahead of the render-daemon
  half of #12027.
  - `component.ManifestPath == ""` (no owning `mosaic-package.toml`) returns
    `{"available": false, "reason": "..."}` without invoking the compiler —
    `mosaic-compile pkg` needs a real package root, and a standalone
    component has none.
  - `nativeComplete` in the response reflects the *selected component's*
    filtered lists, not the whole package's — the Rust
    `DegradationReport.nativeComplete` field is package-wide (any
    degradation anywhere in the package makes it `false`), which would make
    an actually-clean component read as "incomplete" whenever a sibling
    component in the same package has a drop.
  - Verified against the real toolchain: real `cargo build --release -p
    mosaic-compile`, a real `mosaicbook-server` pointed at
    `mosaic-pkg-toolkit`, and a real browser session exercising the Qt tab
    against `Radio` (a real, currently-allowlisted
    `property.radio-group-ignored` degradation — see
    `mosaic-pkg-toolkit/tests/native_complete_gate.rs`) and against `Button`
    (genuinely clean, renders "NATIVE COMPLETE"). Confirmed switching back
    to a Tier-1 tab (`html`) restores the iframe unaffected.
- `handleAPIBackends`'s response schema changed from a single `available`
  boolean to separate `rendered`/`analysis` booleans, since a native backend
  can now be analysis-available (true, no daemon needed) while still not
  rendered (no daemon exists) — a distinction the old single boolean
  couldn't express. Nothing in the frontend consumed this endpoint before
  this change (tabs were static markup, not built from it), so this is not
  a behavior change for the browser shell.
- **Three-file (UI29) component discovery.** MosaicBook previously discovered
  only `.mosaic` files. There are none left anywhere in this repo — every
  component (19 packages, 23 toolkit atoms) is authored as separate
  `.mil`/`.mll`/`.msl` files inside a Mosaic package. The viewer could
  therefore not display a single real component on any backend. Discovery now
  anchors on `*.mil` and pairs siblings by base name:
  - a `.mil` with no matching `.mll` is skipped, not reported as broken — it
    may be a shared interface fragment
  - stylesheet resolution prefers `*.light.msl` and falls back to
    `*.dark.msl`, so a dark-only component previews rather than rendering
    unstyled; neither variant is also tolerated
  - the owning `mosaic-package.toml` is located by walking up from the source
    directory, stopping at the served root so the search cannot escape the tree
- **Three-file compiler invocation.** `compilerArgs` selects between the legacy
  single-file form and `--interface/--layout/--style`. `--package-manifest` is
  passed whenever a manifest was found: it registers the package's exported
  component names, without which a layout cannot reference its siblings (Field
  referencing Input) and the compile fails.

### Notes

- The legacy single-file `.mosaic` path is retained and still tested. Nothing
  in this repo uses it; removing it is a separate decision.
- Verified against the real `mosaic-pkg-toolkit`: 23 atoms discovered, each
  correctly paired with its stylesheet and package manifest (previously 0).

## 0.1.0 — 2026-05-11

### Added

- Phase 1: Tier 1 browser-native backends (`html`, `webcomponent`, `react`)
- Story discovery: recursive `.mosaic` + `.stories.json` scanning with
  CamelCase → "Camel Case" title derivation
- Auto-generation of a single `"Default"` story for `.mosaic` files with no
  accompanying `.stories.json`
- REST API:
  - `GET /api/stories` — full component + story catalogue as JSON
  - `GET /api/backends` — backend availability list (html/webcomponent/react
    available; qt listed as unavailable pending Phase 3)
  - `GET /preview/{backend}/{component_id}/{story_name}` — on-demand compile
    + HTML-wrapped preview for iframe display
- Server-Sent Events at `GET /events` for hot-reload notification
- Polling file watcher (1-second interval) using `filepath.Walk` + mtime
  comparison — no external dependencies, 100% cross-platform
- Browser shell (`static/index.html`): sidebar with component/story tree,
  backend tab bar (HTML / Web Component / React), full-height iframe preview,
  SSE-driven hot reload, keyboard shortcuts (arrow keys, 1–3, R)
- Backend-specific HTML wrapping in `preview.go`:
  - `html`: minimal full page with body styles
  - `webcomponent`: `<script type="module">` + kebab-case custom element usage
  - `react`: unpkg CDN React 18 + ReactDOM + Babel in-browser transform
- Graceful error pages: compilation failures are rendered as styled HTML inside
  the iframe rather than 5xx JSON responses
- CLI flags: `--port` (default 7331), `--root` (default `.`),
  `--compiler` (default `mosaic-compile`)
- Embedded assets via `go:embed` — the binary is entirely self-contained
- 15 unit tests covering:
  - Story discovery (found, auto-generated Default, empty dir, nested dirs)
  - Component ID and title derivation
  - `.stories.json` parsing (valid, empty stories array)
  - `GET /api/stories` handler
  - `GET /api/backends` handler (correct tiers and availability)
  - `GET /preview/` with invalid backend (400), unknown component (404)
  - Preview handler returning HTML error page when compiler not found
  - `toKebabCase` and `componentNameFromID` helpers
- `BUILD` file: `go build` + `go test ./... -v`
- `README.md` with architecture diagram, API reference, story format docs
- `CHANGELOG.md` (this file)
