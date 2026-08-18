# Changelog — mosaicbook-server

## Unreleased

### Added

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
