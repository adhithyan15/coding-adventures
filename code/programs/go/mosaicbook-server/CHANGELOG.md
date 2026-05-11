# Changelog — mosaicbook-server

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
