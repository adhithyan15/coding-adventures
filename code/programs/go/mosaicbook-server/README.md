# mosaicbook-server

A local development server — the "Storybook" for Mosaic components.

MosaicBook discovers Mosaic components in your project, compiles them on demand
to browser-native backends (HTML, Web Component, React), and serves an
interactive preview UI in your browser.  File changes are detected
automatically and the preview reloads within one second.

Two authoring forms are discovered:

- **Three-file (UI29)** — `Button.mil` + `Button.mll` + `Button.light.msl`
  inside a Mosaic package.  This is what every component in this repo uses.
  A `.mil` with no sibling `.mll` is skipped (it may be a shared interface
  fragment); the stylesheet prefers `*.light.msl` and falls back to
  `*.dark.msl`.  The owning `mosaic-package.toml` is located automatically and
  passed as `--package-manifest`, which is what lets a layout reference sibling
  components (`Field` referencing `Input`).
- **Legacy single-file** — one `Button.mosaic` containing interface, layout and
  style together.  Still supported; nothing in this repo uses it.

## What is MosaicBook?

[Storybook](https://storybook.js.org/) is a popular tool for developing UI
components in isolation.  MosaicBook serves the same purpose for the Mosaic
component language: it gives you a sidebar of every component in the project, a
set of named story variants for each component, and a live preview rendered
through each compilation backend.

Phase 1 supports three **Tier 1 browser-native** backends:

| Backend        | Output                     | Preview mechanism                  |
| -------------- | -------------------------- | ---------------------------------- |
| `html`         | HTML fragment              | Embedded in a minimal full page    |
| `webcomponent` | JavaScript Custom Element  | `<script type="module">` + usage tag |
| `react`        | JSX/TSX                    | Babel in-browser transform (unpkg) |

Phase 2 (planned) adds Cairo and Skia paint backends.  Phase 3 adds a Qt
daemon for native rendering.

## Building

```bash
cd code/programs/go/mosaicbook-server
go build -o mosaicbook-server .
```

## Running

```bash
./mosaicbook-server \
  --root path/to/your/mosaic/project \
  --compiler path/to/mosaic-compile \
  --port 7331
```

Then open `http://localhost:7331` in your browser.

### Flags

| Flag         | Default          | Description                                     |
| ------------ | ---------------- | ----------------------------------------------- |
| `--port`     | `7331`           | TCP port to listen on                           |
| `--root`     | `.` (cwd)        | Directory to scan for Mosaic components        |
| `--compiler` | `mosaic-compile` | Path (or name on PATH) to the compiler binary   |

## REST API

All endpoints return JSON unless noted.

### `GET /api/stories`

Returns all discovered components and their stories.

```json
{
  "components": [
    {
      "id": "src/Button",
      "title": "Button",
      "source_path": "src/Button.mosaic",
      "stories": [
        { "name": "Default", "fixtures": {} },
        { "name": "Primary", "fixtures": { "label": "Click me" } }
      ]
    }
  ]
}
```

### `GET /api/backends`

Returns the list of backends. `rendered` is whether the backend has a live
preview (Tier 1's iframe); `analysis` is whether degradation analysis is
available (see `/api/degradations` below) — the two are independent, since
every native backend supports analysis today with no render daemon built for
any of them yet.

```json
{
  "backends": [
    { "id": "html",         "tier": 1, "rendered": true,  "analysis": false },
    { "id": "webcomponent", "tier": 1, "rendered": true,  "analysis": false },
    { "id": "react",        "tier": 1, "rendered": true,  "analysis": false },
    { "id": "xaml",         "tier": 3, "rendered": false, "analysis": true, "reason": "no render daemon yet..." },
    { "id": "swiftui",      "tier": 3, "rendered": false, "analysis": true, "reason": "no render daemon yet..." },
    { "id": "qt",           "tier": 3, "rendered": false, "analysis": true, "reason": "no render daemon yet..." },
    { "id": "flutter",      "tier": 3, "rendered": false, "analysis": true, "reason": "no render daemon yet..." },
    { "id": "compose",      "tier": 3, "rendered": false, "analysis": true, "reason": "no render daemon yet..." }
  ]
}
```

### `GET /api/degradations/{backend}/{component_id}`

Runs `mosaic-compile pkg` (a package-wide build, distinct from the
single-component invocation `/preview/` uses) at the `permissive` profile and
returns what that backend's native lowering dropped for the given component —
capability degradations (e.g. no native radio-group exclusion) and style
degradations (e.g. `box-shadow` with no XAML equivalent) alike. `{backend}`
must be one of `xaml`, `swiftui`, `qt`, `flutter`, `compose`; Tier-1 backends
have no lowering step to drop from and are rejected with 400.

```json
{
  "available": true,
  "schemaVersion": 1,
  "profile": "permissive",
  "package": "mosaic-pkg-toolkit",
  "backend": "qt",
  "nativeComplete": false,
  "degradations": [
    {
      "code": "property.radio-group-ignored",
      "backend": "qt",
      "component": "Radio",
      "layoutPath": "root.props[3]",
      "primitive": "HostRadio",
      "reason": "the backend does not apply the authored HostRadio group to a native mutual-exclusion mechanism"
    }
  ],
  "styleDegradations": []
}
```

A component with no owning `mosaic-package.toml` returns
`{"available": false, "reason": "..."}` — `mosaic-compile pkg` needs a real
package root, which a standalone component doesn't have.

See `code/specs/UI19-mosaicbook.md` §13 for the full design, including why
this needs no render daemon or platform toolchain.

### `GET /preview/{backend}/{component_id}/{story_name}`

Compiles the component to the requested backend and returns a self-contained
HTML page for iframe display.  The component ID uses forward slashes
(percent-encoded in the URL):

```
GET /preview/html/src%2FButton/Default
```

On compilation error, returns a styled HTML error page (not a 5xx response) so
the iframe remains a useful debugging surface.

### `GET /events`

Server-Sent Events stream.  The browser shell subscribes to this endpoint for
hot-reload notifications.  The server pushes a message whenever any `.mosaic`
or `.stories.json` file changes:

```
data: {"type":"reload","component_id":"all"}
```

## Story format

Place a `.stories.json` file alongside any `.mosaic` file to define named story
variants:

```json
{
  "title": "Button",
  "stories": [
    {
      "name": "Default",
      "fixtures": {}
    },
    {
      "name": "Primary",
      "fixtures": {
        "label": "Submit",
        "variant": "primary"
      }
    }
  ]
}
```

If no `.stories.json` exists, a single `"Default"` story with empty fixtures is
auto-generated.

## Architecture

```
main.go       — flag parsing, server startup, watcher goroutine launch
server.go     — Server struct, HTTP mux, /api/* handlers, SSE machinery
stories.go    — component discovery (three-file + legacy .mosaic),
                .stories.json pairing, title derivation
compiler.go   — mosaic-compile subprocess invocation (single-component mode)
preview.go    — /preview/* handler, backend-specific HTML wrapping
degradations.go — /api/degradations/* handler (mosaic-compile pkg mode)
watcher.go    — 1-second polling file watcher
static.go     — embed.FS declaration for static/index.html
static/
  index.html  — single-file browser shell SPA (vanilla HTML/CSS/JS)
```

The binary is self-contained: `static/index.html` is embedded at compile time
via `go:embed`, so no separate asset directory is needed alongside the
executable.

## Running tests

```bash
go test ./... -v
```

Tests cover story discovery, title derivation, API handlers, and error paths.
The `mosaic-compile` binary is not required for tests — the compiler error path
is exercised using a deliberately nonexistent compiler path.

## How it fits in the stack

```
.mosaic files         ← authored by the developer
      │
      ▼
mosaic-compile        ← Rust binary (code/packages/rust/mosaic-compile/)
      │
      ▼ html / webcomponent / react output
      │
mosaicbook-server     ← this binary
      │
      ▼
browser shell         ← static/index.html (embedded)
```
