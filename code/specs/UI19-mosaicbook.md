# UI19 — MosaicBook: Cross-Backend Component Viewer

**Status:** Specification  
**Scope:** Local development tool — a Storybook-equivalent that renders Mosaic
components across every compiler backend side-by-side.

---

## 1. Motivation and Goals

The Mosaic compiler currently targets four backends (HTML, WebComponent, React,
Qt) with paint-raster backends (Cairo, Skia) planned. Each backend produces
different output and runs in a different runtime environment. Iterating on a
component requires re-compiling and manually opening the output, with no way to
compare backends at a glance or inject test fixtures interactively.

**MosaicBook** is a local-first development server + browser shell that:

1. **Discovers** all `.mosaic` files in a project tree.
2. **Compiles** each component to every enabled backend on-demand.
3. **Previews** the output in a tabbed or tiled panel — iframes for web
   backends, PNG snapshots for native/paint backends.
4. **Hot-reloads** when source files or fixture files change.
5. **Lets you edit fixtures** in the browser UI without touching source files.

The design mirrors Storybook conceptually, but the transport and rendering
model are necessarily different for native backends.

---

## 2. Glossary

| Term | Definition |
|---|---|
| **Component** | A single `.mosaic` file defining a named component |
| **Story** | A component + a named set of fixture values |
| **Backend** | A compilation target (html, webcomponent, react, qt, cairo, skia) |
| **Preview** | The rendered output of one story on one backend |
| **Fixture** | A JSON object mapping slot names to runtime values |
| **Snapshot** | A PNG image produced by a paint or native backend |
| **Daemon** | A long-running process that owns a native rendering context (Qt, Cairo, Skia) |

---

## 3. Backend Taxonomy

Backends are grouped into three tiers based on how their output is previewed.

### Tier 1 — Browser-native

Output is HTML/JS that runs directly in the browser. Preview is an `<iframe>`.

| Backend | Output | `<iframe>` contents |
|---|---|---|
| `html` | `ComponentName.html` | Direct iframe `src` |
| `webcomponent` | `ComponentName.js` | Inline `<script>` + custom element usage |
| `react` | `ComponentName.tsx` | Vite-served React app in iframe |

The MosaicBook server bundles each Tier-1 output into a self-contained HTML
page and serves it at `/preview/<backend>/<story>`.

### Tier 2 — Paint/raster

The Mosaic VM drives a `PaintScene` renderer (requires `mosaic-emit-paint`, a
planned crate that adds a layout engine on top of `mosaic-vm`). The scene is
passed to `barcode_2d::render_scene_png` (or directly to paint-vm-cairo /
paint-vm-skia). The result is a PNG byte stream sent back to the browser as a
`data:` URL inside an `<img>`.

| Backend | Renderer | System requirement |
|---|---|---|
| `cairo` | `paint-vm-cairo` | `brew install cairo` / `apt install libcairo2-dev` |
| `skia` | `paint-vm-skia` | None (Skia is statically linked) |

> **Note:** Tier 2 requires `mosaic-emit-paint`, which needs a Mosaic layout
> engine (computing pixel positions for Box/Column/Row/Text). This is separate
> work tracked in spec UI22.

### Tier 3 — Compilation-server

Output requires a native runtime that cannot run inside the browser. The
MosaicBook server spawns and manages a **render daemon** for each native
backend. The daemon accepts a render request, produces a PNG snapshot, and
returns it to the server, which proxies it to the browser.

| Backend | Daemon | System requirement |
|---|---|---|
| `qt` | `mosaicbook-qt-daemon` | Qt 6.x installed |

Tier 3 also supports **live native window embedding** (v2 feature, §8) where
the daemon keeps a Qt window alive and streams frame updates over WebSocket.

---

## 4. Story Format

Stories are defined by placing a `.stories.json` file alongside a `.mosaic`
file. The two files are paired by name.

```
src/
  TaskBoard.mosaic
  TaskBoard.stories.json
  ProfileCard.mosaic
  ProfileCard.stories.json
```

### 4.1 `.stories.json` Schema

```jsonc
{
  // Optional: override the human-readable display name in the UI.
  "title": "Task Board",

  // Required: at least one story.
  "stories": [
    {
      // Required: unique within this file.
      "name": "Empty",
      // Required: maps slot names to fixture values.
      "fixtures": {}
    },
    {
      "name": "Three tasks",
      "fixtures": {
        "tasks": ["Buy milk", "Walk dog", "Ship it"],
        "title": "Today's tasks"
      }
    },
    {
      "name": "Many tasks",
      "fixtures": {
        "tasks": ["Task 1", "Task 2", "Task 3", "Task 4", "Task 5"],
        "title": "Backlog"
      }
    }
  ]
}
```

If no `.stories.json` exists for a `.mosaic` file, MosaicBook auto-generates a
single story named `"Default"` with an empty fixtures object — showing the
component with no data.

### 4.2 Fixture Value Types

Fixture values must be JSON-compatible and must match the corresponding slot
type declared in the Mosaic source:

| Mosaic slot type | JSON fixture type |
|---|---|
| `text` | `string` |
| `number` | `number` |
| `bool` | `boolean` |
| `image` | `string` (URL) |
| `color` | `string` (`"#rrggbb"` or CSS color) |
| `list<text>` | `["string", ...]` |
| `list<number>` | `[1, 2, 3]` |
| `node` | `string` (rendered as raw HTML in HTML/WC backends, text in native) |

---

## 5. MosaicBook Server

The server is a single-binary local dev server implemented in Go (building on
the existing `build-tool` infrastructure) or Rust. It exposes an HTTP + WebSocket
API and serves the browser shell as static assets.

### 5.1 Directory structure

```
.mosaicbook/           ← per-project config (gitignored)
  config.json          ← enabled backends, daemon ports, watch roots
  cache/               ← compiled output cache keyed by (file hash, backend)

code/
  programs/go/mosaicbook/    ← server binary source
  packages/rust/mosaic-emit-react/  ← (existing)
  packages/rust/mosaic-emit-qt/     ← (planned, UI20)
  packages/rust/mosaic-emit-paint/  ← (planned, UI22)
```

### 5.2 REST API

All endpoints return JSON unless documented otherwise.

#### `GET /api/stories`

Returns all discovered components and their stories.

```json
{
  "components": [
    {
      "id": "src/TaskBoard",
      "title": "Task Board",
      "source_path": "src/TaskBoard.mosaic",
      "stories": [
        { "name": "Empty",        "fixtures": {} },
        { "name": "Three tasks",  "fixtures": { "tasks": [...] } }
      ]
    }
  ]
}
```

#### `GET /api/backends`

Returns available backends and their status.

```json
{
  "backends": [
    { "id": "html",          "tier": 1, "available": true  },
    { "id": "webcomponent",  "tier": 1, "available": true  },
    { "id": "react",         "tier": 1, "available": true  },
    { "id": "cairo",         "tier": 2, "available": true  },
    { "id": "skia",          "tier": 2, "available": true  },
    { "id": "qt",            "tier": 3, "available": false, "reason": "Qt daemon not running" }
  ]
}
```

#### `GET /preview/{backend}/{component_id}/{story_name}`

For **Tier 1** backends: returns a self-contained HTML page for iframe display.

For **Tier 2** and **Tier 3** backends: returns an HTML page containing a
single `<img>` whose `src` is a data URL embedding the PNG snapshot.

Query parameters:
- `fixtures` — URL-encoded JSON object overriding story fixtures (for the live
  fixture editor, §6.3).
- `width`, `height` — viewport hint for native backends (default: `400×300`).

#### `POST /api/render`

For Tier 2/3 backends — explicitly request a render to PNG.

Request body:
```json
{
  "component_id": "src/TaskBoard",
  "story_name": "Three tasks",
  "backend": "cairo",
  "fixtures": { "tasks": ["Buy milk", "Walk dog"] },
  "width": 400,
  "height": 300
}
```

Response:
```json
{
  "format": "png",
  "data": "<base64-encoded PNG>",
  "render_time_ms": 42,
  "backend_version": "cairo 1.18.4"
}
```

#### `WebSocket /ws`

Clients connect and receive push notifications:

```json
{ "type": "reload",    "component_id": "src/TaskBoard" }
{ "type": "error",     "component_id": "src/TaskBoard", "message": "parse error at line 4" }
{ "type": "daemon_up", "backend": "qt",    "version": "Qt 6.11.0" }
{ "type": "daemon_down","backend": "qt" }
```

---

## 6. Browser Shell (MosaicBook UI)

The shell is a single-page web application served by the MosaicBook server.
It is implemented with plain TypeScript + minimal dependencies (no React
dependency for the shell itself — the shell must bootstrap before any backend
is available).

### 6.1 Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  MosaicBook                                   🔴 Qt  🟢 Cairo  🟢 Skia │
├─────────────────┬───────────────────────────────────────────────────┤
│                 │  Backend selector:                                 │
│  📄 TaskBoard   │  [HTML] [WebComponent] [React/TSX] [Cairo] [Qt]    │
│    ● Empty      │─────────────────────────────────────────────────── │
│    ● 3 tasks    │  Story: "Three tasks"              [Edit fixtures] │
│    ● Many       │─────────────────────────────────────────────────── │
│                 │                                                     │
│  📄 ProfileCard │  ┌──────────────┐ ┌───────────────┐ ┌───────────┐ │
│    ● Default    │  │ HTML         │ │ WebComponent  │ │ React/TSX │ │
│                 │  │  <iframe>    │ │  <iframe>     │ │  <iframe> │ │
│                 │  │             │ │              │ │           │ │
│                 │  └──────────────┘ └───────────────┘ └───────────┘ │
│                 │  ┌──────────────┐ ┌───────────────┐               │
│                 │  │ Cairo        │ │ Qt            │               │
│                 │  │  [PNG img]   │ │  [PNG img]    │               │
│                 │  │             │ │              │               │
│                 │  └──────────────┘ └───────────────┘               │
└─────────────────┴───────────────────────────────────────────────────┘
```

### 6.2 Story browser (left panel)

- Lists all components and their stories discovered by the server.
- Live-updates via the WebSocket when files change.
- Clicking a story makes it the active story and triggers compilation for all
  enabled backends.
- Shows a ⚠️ badge when a component has a parse/compile error.

### 6.3 Fixture editor (modal)

Clicking **"Edit fixtures"** opens a JSON editor pre-populated with the current
story's fixtures. Changes are applied immediately (debounced 300 ms) — the
server re-renders all backends with the new fixture values and the previews
update. Changes are **not** written back to `.stories.json` unless the user
explicitly saves (separate "Save to stories file" button).

### 6.4 Backend selector (tabs)

Each backend appears as a tab. Tabs are ordered: HTML → WebComponent → React →
Cairo → Skia → Qt.

- Unavailable backends (§5.2) appear greyed-out.
- In **tiled mode** all available backends are shown in a grid.
- In **focused mode** one backend fills the full preview area.
- Toggle between modes with a keyboard shortcut (`T`).

### 6.5 Preview panes

**Tier 1 (iframe):**
```html
<iframe
  src="/preview/html/src%2FTaskBoard/Three+tasks"
  sandbox="allow-scripts allow-same-origin"
  loading="lazy"
/>
```

The iframe `src` is replaced whenever the story or fixtures change. A loading
spinner is shown during compilation.

**Tier 2 / Tier 3 (image):**
```html
<img
  src="data:image/png;base64,<...>"
  alt="Cairo render of TaskBoard — Three tasks"
/>
```

The image is replaced when the server sends a new snapshot via WebSocket.

### 6.6 Keyboard shortcuts

| Key | Action |
|---|---|
| `T` | Toggle tiled / focused mode |
| `↑` / `↓` | Previous / next story |
| `1`–`6` | Jump to backend by index |
| `R` | Force re-render all backends |
| `E` | Open fixture editor |
| `Escape` | Close fixture editor |

---

## 7. Native Render Daemon Protocol (Tier 3)

The render daemon is a long-running process that holds a native rendering
context. It accepts render requests over a Unix domain socket (on macOS/Linux)
or a named pipe (on Windows).

### 7.1 Qt daemon (`mosaicbook-qt-daemon`)

The Qt daemon runs a `QGuiApplication` with an offscreen `QQuickView`. On each
render request it:

1. Receives the QML source compiled by `mosaic-emit-qt`.
2. Loads the QML into the offscreen `QQuickView`.
3. Sets fixture values as QML context properties.
4. Calls `QQuickWindow::grabWindow()` to capture a PNG.
5. Sends the PNG bytes back over the socket.

The daemon keeps the `QQuickView` alive between requests to avoid cold-start
overhead. QML is reloaded only when the source changes (compared by hash).

#### Socket message format

Messages are length-prefixed JSON + optional binary payload:

```
[4-byte big-endian length][JSON payload][optional binary]
```

**Request:**
```json
{
  "type": "render",
  "qml": "import QtQuick 2.15; Rectangle { ... }",
  "fixtures": { "tasks": ["a", "b"] },
  "width": 400,
  "height": 300
}
```

**Response:**
```json
{
  "type": "render_ok",
  "format": "png",
  "width": 400,
  "height": 300,
  "render_time_ms": 18
}
```
Followed immediately by `width * height * 4` raw bytes (RGBA) **or** a
PNG-encoded payload (controlled by the request's `"format"` field).

**Error response:**
```json
{
  "type": "render_error",
  "message": "QML parse error at line 3: unknown type"
}
```

### 7.2 Daemon lifecycle

The MosaicBook server:
1. Attempts to connect to the daemon socket on startup.
2. If not running, spawns the daemon as a child process.
3. Monitors the daemon via a heartbeat request every 5 seconds.
4. If the daemon exits, marks the Qt backend as unavailable and notifies
   connected browsers via WebSocket.
5. Provides a "Restart daemon" button in the browser shell.

### 7.3 Cairo / Skia daemons (Tier 2)

Cairo and Skia rendering is synchronous and runs in-process in the MosaicBook
server (no daemon needed). The server calls `barcode_2d::render_scene_png` (or
the planned `mosaic_emit_paint::render_png`) directly and returns the PNG.

---

## 8. Native Window Embedding (v2)

Instead of PNG snapshots, v2 allows embedding a live native Qt window inside
the browser shell. This enables real mouse/keyboard interaction with the
rendered component.

### 8.1 Electron shell

In v2, MosaicBook is wrapped in an **Electron** application. Electron gives
access to Node.js APIs and the ability to host native windows.

On macOS, the Qt window's `NSView` is reparented into the Electron `BrowserWindow`'s
`WKWebView` using the `nativeWindowHandle()` API.

On Linux, XEmbed protocol (`_XEMBED` X11 atoms) is used.

On Windows, the Qt HWND is re-parented into the Electron window.

### 8.2 Frame streaming fallback

For platforms where window reparenting is not feasible, the daemon can stream
frames over WebSocket at a configurable FPS (default: 30):

```json
{ "type": "frame_start", "backend": "qt", "width": 400, "height": 300, "fps": 30 }
```

Followed by a binary WebSocket frame per rendered frame (raw RGBA or JPEG for
bandwidth efficiency).

### 8.3 Input forwarding

Mouse and keyboard events from the browser are forwarded to the daemon via
WebSocket:

```json
{ "type": "mouse_move", "x": 120, "y": 85 }
{ "type": "mouse_down", "button": "left", "x": 120, "y": 85 }
{ "type": "key_down",   "key": "Enter" }
```

---

## 9. Configuration

`.mosaicbook/config.json`:

```json
{
  "version": 1,
  "watch_roots": ["src", "components"],
  "backends": {
    "html":         { "enabled": true },
    "webcomponent": { "enabled": true },
    "react":        { "enabled": true },
    "cairo":        { "enabled": true },
    "skia":         { "enabled": true },
    "qt": {
      "enabled": true,
      "daemon_binary": "mosaicbook-qt-daemon",
      "socket_path": "/tmp/mosaicbook-qt.sock",
      "startup_timeout_ms": 5000
    }
  },
  "server": {
    "port": 7743,
    "open_browser": true
  },
  "preview": {
    "default_width": 400,
    "default_height": 300,
    "default_mode": "tiled"
  }
}
```

---

## 10. Implementation Layers and Milestones

### Phase 1 — Tier 1 server (browser backends only)

Deliverables:
- `mosaicbook-server` binary (Go or Rust) with file watching + HTTP server
- REST API: `/api/stories`, `/api/backends`, `/preview/{backend}/{component}/{story}`
- Browser shell: story browser, iframe preview, backend tabs, hot-reload via WS
- Supported backends: `html`, `webcomponent`, `react`

### Phase 2 — Tier 2 (paint backends)

Deliverables:
- `mosaic-emit-paint` crate (spec UI22): Mosaic → layout → PaintScene
- `/api/render` endpoint for Cairo and Skia
- PNG snapshot display in the browser shell
- Requires merging spec UI22 (layout engine)

### Phase 3 — Tier 3 daemon (Qt)

Deliverables:
- `mosaic-emit-qt` crate (spec UI20): Mosaic → QML
- `mosaicbook-qt-daemon` binary (C++ or Rust + Qt bindings)
- Daemon lifecycle management in the server
- PNG snapshot display + daemon status indicator in the shell

### Phase 4 — Native window embedding (Electron)

Deliverables:
- Electron wrapper for the browser shell
- NSView / XEmbed / HWND reparenting
- Input forwarding protocol
- Frame streaming fallback

---

## 11. Security Considerations

- The MosaicBook server binds to `localhost` only and is not exposed to the
  network.
- Story fixture values from `.stories.json` and the fixture editor are
  HTML-escaped by the backend renderers before embedding in HTML/JSX output.
- The daemon socket is mode `0600` and accessible only to the current user.
- QML loaded by the Qt daemon is compiled from Mosaic source — user-controlled
  identifiers (component names, slot names) are constrained to identifier syntax
  by `mosaic-analyzer` before reaching the QML emitter.
- The fixture editor sends fixture values to the server; the server validates
  that they match the declared slot types before passing them to the compiler.

---

## 12. Non-Goals (v1)

- **Multi-user / network serving**: MosaicBook is a local dev tool only.
- **Publishing stories**: No Storybook-compatible static export format (yet).
- **Accessibility testing**: No a11y audit integration in v1.
- **Visual regression testing**: No snapshot diffing in v1 (planned for v2).
- **TypeScript compilation for React preview**: v1 uses Vite's built-in TSX
  transform; no custom Babel/esbuild pipeline.
