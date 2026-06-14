# mosaicbook-server

A local development server — the "Storybook" for Mosaic components.

MosaicBook discovers `.mosaic` files in your project, compiles them on demand
to browser-native backends (HTML, Web Component, React), and serves an
interactive preview UI in your browser.  File changes are detected
automatically and the preview reloads within one second.

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
| `--root`     | `.` (cwd)        | Directory to scan for `.mosaic` files           |
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

Returns the list of backends and their Phase 1 availability.

```json
{
  "backends": [
    { "id": "html",         "tier": 1, "available": true  },
    { "id": "webcomponent", "tier": 1, "available": true  },
    { "id": "react",        "tier": 1, "available": true  },
    { "id": "qt",           "tier": 3, "available": false, "reason": "Qt daemon not running (Phase 3)" }
  ]
}
```

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
stories.go    — .mosaic + .stories.json discovery, title derivation
compiler.go   — mosaic-compile subprocess invocation
preview.go    — /preview/* handler, backend-specific HTML wrapping
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
