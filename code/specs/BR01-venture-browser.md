# BR01 — Venture: A Cross-Platform Native Web Browser

## Overview

Venture is a cross-platform native web browser built entirely from educational
packages in the coding-adventures monorepo. Rather than embedding a massive
rendering engine like Chromium or Gecko, Venture is a **thin orchestrator** — a
shell that wires together a pipeline of small, focused crates that each handle
one step of turning a URL into pixels on screen.

**Venture v0.1** targets feature parity with **NCSA Mosaic 1.0** (1993), the
browser that made the World Wide Web accessible to ordinary people.

### Why Mosaic?

NCSA Mosaic was released on **April 22, 1993** by Marc Andreessen and Eric Bina
at the University of Illinois at Urbana-Champaign. Before Mosaic, web browsers
existed — Tim Berners-Lee's WorldWideWeb (1990), ViolaWWW (1992), Erwise
(1992) — but they were academic tools used almost exclusively by researchers.

Mosaic changed everything:

- **Inline images** — previous browsers opened images in separate windows.
  Mosaic rendered `<img>` tags directly in the document flow, making pages
  visually rich for the first time.
- **Proportional fonts** — earlier browsers used monospace text exclusively.
  Mosaic used Times New Roman and other proportional typefaces, making pages
  feel like printed documents.
- **Friendly GUI** — toolbar buttons for Back, Forward, Home, and Reload. A
  URL bar you could type into. A status bar showing loading progress. These
  conventions survive unchanged in every browser today.
- **Cross-platform** — versions shipped for X Window System (Unix), Macintosh,
  and Windows, bringing the web to every desktop.

Mosaic was the **Macintosh moment** for the web. Within 18 months of its
release, web traffic went from 0.1% of internet backbone traffic to 97%.
Andreessen left UIUC to co-found Netscape, and the browser wars began.

Venture v0.1 recreates this experience using the coding-adventures package
ecosystem. The web in 1993 was simple enough that a single developer can
understand the entire stack — from TCP socket to rendered pixel — and that is
exactly the educational goal.

## Where It Fits

Venture sits at the top of the stack. It is a **program** (not a library) that
consumes nearly every layer of the coding-adventures package ecosystem:

```
Layer 7 — Programs
  └── BR01 Venture Browser  ← you are here

Layer 6 — Platform Paint VMs
  ├── P2D06 paint-vm-direct2d (Windows)
  ├── P2D07 paint-vm-gdi (Windows fallback)
  └── P2D08 paint-vm-cairo (Linux, future)

Layer 5 — Layout & Paint Translation
  ├── layout-block
  ├── layout-to-paint
  └── document-ast-to-layout

Layer 4 — Document Model
  ├── document-ast
  ├── layout-ir
  └── paint-instructions

Layer 3 — Protocol Parsers
  ├── http1.0-lexer / http1.0-parser / http1.0-client
  └── html1.0-lexer / html1.0-parser

Layer 2 — Network Primitives
  ├── tcp-client
  └── frame-extractor

Layer 1 — Data Formats
  ├── url-parser
  └── text-measure-directwrite
```

Every box is a separate crate. Improving any crate automatically improves the
browser. Adding CSS support in the future means adding a `css-lexer`,
`css-parser`, and updating `document-ast-to-layout` — Venture itself barely
changes.

## Concepts

### The Browser as a Pipeline

A web browser is fundamentally a **data transformation pipeline**. A URL goes
in, pixels come out. Each stage transforms one representation into the next:

```
URL (string)
  │
  ▼
url-parser ──────────────────► ParsedUrl { scheme, host, port, path }
  │
  ▼
tcp-client ──────────────────► TcpStream (raw bytes)
  │
  ▼
http1.0-client ──────────────► HttpResponse { status, headers, body }
  │
  ▼
html1.0-parser ──────────────► DocumentNode (tree of elements & text)
  │
  ▼
document-ast-to-layout ──────► LayoutNode (styled tree with fonts/colors)
  │
  ▼
layout-block ────────────────► PositionedNode (x, y, width, height for each node)
  │
  ▼
layout-to-paint ─────────────► PaintScene (list of draw commands)
  │
  ▼
paint-vm-direct2d ───────────► Pixels on screen
```

This pipeline architecture means:

1. **Each stage is independently testable.** You can unit-test the HTML parser
   without a network connection. You can test layout without a window.
2. **Each stage is independently replaceable.** Swap `paint-vm-direct2d` for
   `paint-vm-metal` and you have macOS support — no other code changes.
3. **The browser shell is trivial.** It just calls each stage in sequence and
   manages user interaction (clicks, scrolling, navigation).

### Navigation Model

A browser's navigation state is two stacks and a pointer:

```
     back_stack          current_url         forward_stack
    ┌──────────┐                             ┌──────────┐
    │ page_1   │        page_3               │ page_4   │
    │ page_2   │                             │ page_5   │
    └──────────┘                             └──────────┘

    Navigate to X:  push current → back_stack, clear forward_stack, current = X
    Back:           push current → forward_stack, current = back_stack.pop()
    Forward:        push current → back_stack, current = forward_stack.pop()
    Home:           navigate to home_url (same as Navigate)
    Reload:         re-fetch current_url, re-run pipeline
```

This is the same model every browser uses today. The back and forward stacks
are `Vec<String>` — just lists of URLs.

### Hit-Testing

When the user clicks somewhere in the content area, the browser needs to
determine what they clicked on. This is **hit-testing**: given an (x, y) screen
coordinate, find the element at that position.

During the layout-to-paint stage, each link gets recorded as a `LinkRegion`:

```rust
struct LinkRegion {
    rect: Rect,         // bounding box in content coordinates
    url: String,        // destination URL
}
```

On mouse click:
1. Adjust click position by scroll offset: `content_y = click_y + scroll_y`
2. Linear scan through link regions (fine for Mosaic-era page complexity)
3. If a region contains the point → navigate to that URL
4. If no region matches → do nothing

### Scrolling

The content of a web page is typically taller than the window. Scrolling shifts
which portion of the content is visible:

```
                    ┌─────────────────┐
                    │  viewport_height │
    scroll_y = 0 → │  ╔═══════════╗  │ ← visible region
                    │  ║           ║  │
                    │  ║  content  ║  │
                    │  ║           ║  │
                    │  ╚═══════════╝  │
                    │                 │
                    │  (more content  │
                    │   below...)     │
                    │                 │
                    │content_height   │
                    └─────────────────┘

Scroll down → scroll_y increases → viewport slides down the content
Clamped to: 0 <= scroll_y <= max(0, content_height - viewport_height)
```

Implementation: apply `scroll_y` as a negative Y translation to the entire
PaintScene before rendering. The paint VM sees pre-translated coordinates.

## Architecture

### Full Pipeline Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     Venture Browser (BR01)                       │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                 Thin Orchestrator Shell                    │   │
│  │  - Win32 message loop          - Navigation state         │   │
│  │  - UI chrome (URL bar, toolbar) - Scroll management       │   │
│  │  - Link hit-testing            - Bookmarks                │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                    │
│                              ▼                                    │
│  ┌────────────────── Pipeline Packages ─────────────────────┐   │
│  │                                                            │   │
│  │  url-parser → tcp-client → frame-extractor                │   │
│  │                  → http1.0-lexer → http1.0-parser         │   │
│  │                      → html1.0-lexer → html1.0-parser     │   │
│  │                          → document-ast                    │   │
│  │                              → document-ast-to-layout      │   │
│  │                                  → layout-block            │   │
│  │                                      → layout-to-paint     │   │
│  │                                          → paint-vm-direct2d│  │
│  └────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

Every box in the pipeline is a separate crate. Improving any crate improves
the browser.

### Platform Abstraction

The browser itself has a thin platform layer. Only Windows is implemented for
v0.1; macOS and Linux are future work.

| Platform | Window              | Rendering                                     | Text Measurement           |
|----------|---------------------|-----------------------------------------------|----------------------------|
| Windows  | Win32 CreateWindowExW | paint-vm-direct2d (P2D06) or paint-vm-gdi (P2D07) | text-measure-directwrite  |
| macOS    | Cocoa NSWindow (future) | paint-vm-metal (exists, partial)            | text-measure-coretext (future) |
| Linux    | GTK/X11 (future)    | paint-vm-cairo (P2D08, future)                | text-measure-pango (future) |

### Win32 Window Structure

```
┌─────────────────────────────────────────┐
│ Venture - http://info.cern.ch/          │  ← Window title = page title + URL
├─────────────────────────────────────────┤
│ [Back] [Fwd] [Home] [Reload] │ URL bar │  ← Toolbar + EDIT control
├─────────────────────────────────────────┤
│                                         │
│  World Wide Web                         │  ← Content area (Direct2D render target)
│                                         │
│  The WorldWideWeb (W3) is a wide-area   │    Scrollable via WM_MOUSEWHEEL
│  hypermedia information retrieval        │
│  initiative aiming to give universal     │
│  access to a large universe of           │
│  documents.                              │
│                                         │
│  Everything there is online about W3    │  ← Blue underlined = unvisited link
│  is linked directly or indirectly to    │
│  this document...                       │
│                                         │
├─────────────────────────────────────────┤
│ Done                                    │  ← Status bar: loading / hovered link URL
└─────────────────────────────────────────┘
```

The window is composed of standard Win32 controls:

- **Title bar**: `SetWindowTextW` — updated to show page title and URL.
- **Toolbar**: child `HWND`s — BUTTON controls for Back, Forward, Home, Reload.
- **URL bar**: EDIT control. User presses Enter → navigate.
- **Content area**: owner-drawn region. Venture creates a Direct2D render target
  over this area and paints the PaintScene into it on `WM_PAINT`.
- **Status bar**: STATIC control at the bottom. Shows "Loading...", "Done", or
  the URL of the link under the cursor.

### Loading a Page — Full Sequence

When the user enters a URL or clicks a link, this is everything that happens:

```
User action (Enter key / link click)
  │
  ├─ 1. Push current URL to back_stack, clear forward_stack
  │
  ├─ 2. Show "Loading..." in status bar
  │
  ├─ 3. url_parser::parse(url_string) → ParsedUrl
  │     - Resolve relative URLs against current page base URL
  │
  ├─ 4. http1_0_client::get(parsed_url) → HttpResponse
  │     - Internally: tcp_client::connect → send request → frame_extractor
  │     - Returns: status code, headers, body bytes
  │
  ├─ 5. Check Content-Type header:
  │     - "text/html" → proceed to step 6
  │     - "image/*"   → display standalone image
  │     - other       → show raw text in monospace
  │
  ├─ 6. html1_0_parser::parse(response.body) → DocumentNode tree
  │
  ├─ 7. document_ast_to_layout(doc, mosaic_theme) → LayoutNode tree
  │     - Apply Mosaic-era font/color defaults
  │     - Annotate links with destination URLs
  │
  ├─ 8. layout_block(layout, viewport_width, measurer) → PositionedNode tree
  │     - Compute x, y, width, height for every node
  │     - Text shaping via text-measure-directwrite
  │
  ├─ 9. layout_to_paint(positioned, device_pixel_ratio) → PaintScene
  │     - Convert layout tree to flat list of paint instructions
  │
  ├─ 10. Store PaintScene + Vec<LinkRegion> for hit-testing
  │
  ├─ 11. InvalidateRect(hwnd) → triggers WM_PAINT
  │      → paint_vm_direct2d::render(scene, render_target)
  │
  └─ 12. Show "Done" in status bar
```

### Link Handling

During `document-ast-to-layout`, each `<a href="...">` is annotated with both
its destination URL and a color/underline style:

- **Unvisited links**: `#0000EE` (blue) with underline
- **Visited links**: `#551A8B` (purple) with underline

Visited URLs are tracked in a `HashSet<String>` for the lifetime of the
session (not persisted in v0.1).

On **WM_MOUSEMOVE**:
1. Hit-test mouse position against link regions
2. If hovering a link: show URL in status bar, set cursor to `IDC_HAND`
3. If not hovering: show "Done" in status bar, set cursor to `IDC_ARROW`

On **WM_LBUTTONDOWN**:
1. Hit-test mouse position against link regions
2. If a link was clicked: resolve relative URL against current page, navigate

### Image Loading

When the HTML parser encounters `<img src="photo.gif">`:

1. The `DocumentNode` tree contains an `ImageNode { src, alt }`.
2. During layout, the image source URL is resolved against the page base URL.
3. A separate `http1_0_client::get()` call fetches the image data.
4. The `image` crate decodes the data (GIF or JPEG for v0.1) into pixels.
5. The pixel data becomes a `PaintImage` instruction in the `PaintScene`.
6. If the image fails to load: render the alt text inside a bordered box
   (the classic "broken image" experience).

For v0.1, images are loaded synchronously (blocking the UI). Asynchronous
image loading is a future enhancement.

Supported formats: **GIF** and **JPEG** (via the `image` crate). BMP is
already implemented in the repo (`image-codec-bmp`) but was rare on the 1993
web.

### Scrolling

- Track `scroll_y: f64` offset, starting at 0.0.
- On `WM_MOUSEWHEEL`: adjust `scroll_y` by delta, clamp to
  `[0, max(0, content_height - viewport_height)]`.
- Apply `scroll_y` as a negative Y translation before rendering — wrap the
  entire PaintScene in a `PaintGroup` with a translate transform.
- Scrollbar: native Win32 scrollbar via `SetScrollInfo` / `WM_VSCROLL`,
  reflecting current scroll position and content height.

### Mosaic-Era Theme Defaults

Venture v0.1 uses styling that matches the original Mosaic look and feel:

```rust
DocumentTheme {
    body_font: "Times New Roman",
    body_font_size: 14.0,
    heading_font: "Times New Roman",
    heading_sizes: [24.0, 20.0, 18.0, 16.0, 14.0, 12.0],  // h1..h6
    heading_bold: true,
    code_font: "Courier New",
    code_font_size: 13.0,
    line_height: 1.4,
    paragraph_spacing: 12.0,
    link_color: "#0000EE",
    visited_link_color: "#551A8B",
    background_color: "#C0C0C0",  // the iconic Mosaic gray
}
```

The gray background (`#C0C0C0`) is the most immediately recognizable Mosaic
trait. Modern browsers default to white, but in 1993 the gray matched the
system window color and felt "native."

### Bookmarks

- **Storage**: `%APPDATA%\Venture\bookmarks.json` (Windows),
  `~/.venture/bookmarks.json` (Unix).
- **Format**: JSON array of `{ "title": "...", "url": "..." }` objects.
- **UI**: Menu bar → Bookmarks → "Add Bookmark" / list of saved bookmarks.
- For v0.1, bookmarks are a **flat list** — no folders or hierarchy.

### View Source

- **Trigger**: Ctrl+U or menu View → Source.
- Opens a **new window** showing the raw HTML text of the current page.
- Implementation: wrap the raw HTML in a synthetic `<pre>` document and run it
  through the same rendering pipeline. The "source" window is just another
  Venture window with a fabricated document.

### Dependencies (Cargo.toml)

```toml
[dependencies]
url-parser             = { path = "../../../packages/rust/url-parser" }
tcp-client             = { path = "../../../packages/rust/tcp-client" }
frame-extractor        = { path = "../../../packages/rust/frame-extractor" }
http1_0_lexer          = { path = "../../../packages/rust/http1.0-lexer" }
http1_0_parser         = { path = "../../../packages/rust/http1.0-parser" }
http1_0_client         = { path = "../../../packages/rust/http1.0-client" }
html1_0_lexer          = { path = "../../../packages/rust/html1.0-lexer" }
html1_0_parser         = { path = "../../../packages/rust/html1.0-parser" }
document_ast           = { path = "../../../packages/rust/document-ast" }
document_ast_to_layout = { path = "../../../packages/rust/document-ast-to-layout" }
layout_ir              = { path = "../../../packages/rust/layout-ir" }
layout_block           = { path = "../../../packages/rust/layout-block" }
layout_to_paint        = { path = "../../../packages/rust/layout-to-paint" }
paint_instructions     = { path = "../../../packages/rust/paint-instructions" }
paint_vm_direct2d      = { path = "../../../packages/rust/paint-vm-direct2d" }
text_measure_directwrite = { path = "../../../packages/rust/text-measure-directwrite" }
windows                = { version = "0.58", features = ["..."] }
image                  = "0.25"  # GIF and JPEG decoding
```

## Testing Strategy

### Unit Tests

1. **Navigation model** — Verify back/forward/home stack transitions:
   - Navigate A → B → C, then Back → verify current=B, forward=[C]
   - Forward → verify current=C, forward=[]
   - Navigate A → B → C, Back, Navigate D → verify forward stack cleared

2. **Link hit-testing** — Given a known set of `LinkRegion`s and a click
   coordinate, verify the correct link (or no link) is returned.

3. **Scroll offset clamping** — Verify scroll_y stays within bounds:
   - Cannot go negative
   - Cannot exceed `content_height - viewport_height`
   - Content shorter than viewport → scroll_y stays at 0

4. **URL resolution** — Relative URLs resolved correctly against page base:
   - `href="page2.html"` on `http://example.com/dir/page1.html`
     → `http://example.com/dir/page2.html`

5. **Bookmark persistence** — Add/remove bookmarks, verify JSON round-trip.

### Integration Tests

6. **Canned HTML rendering** — Load a known HTML file, run it through the full
   pipeline, verify the resulting `PaintScene` contains expected instructions
   (e.g., text at expected positions, link with expected color).

7. **Live page load** — Load `http://info.cern.ch/` (the first web page, still
   online), verify it renders without panic and produces a non-empty PaintScene.

8. **Image loading** — Load an HTML page with an `<img>` tag, verify the
   PaintScene contains a `PaintImage` instruction with correct dimensions.

### Visual Tests

9. **Screenshot comparison** — Render known pages and compare against reference
   screenshots of original NCSA Mosaic. This is not pixel-perfect matching but
   a qualitative check that the rendering "looks right."

### Manual Test Scenarios

10. **Full navigation flow** — Open Venture, navigate to info.cern.ch, click
    links, use Back/Forward, add a bookmark, view source.

## Scope

### In Scope (Venture v0.1)

- URL bar navigation with Enter key
- Back, Forward, Home, Reload toolbar buttons
- HTML 1.0 rendering via the full pipeline
- Vertical scrolling with mouse wheel and scrollbar
- Link clicking with hit-testing and navigation
- Inline images (GIF, JPEG) loaded synchronously
- Unvisited/visited link colors (blue/purple)
- Bookmarks: add, list, click to navigate
- View Source (Ctrl+U)
- Status bar: "Loading..." / "Done" / hovered link URL
- Window title: page title + URL
- Windows platform only (Direct2D primary, GDI fallback)

### Out of Scope (Future Versions)

- Multiple windows or tabs
- Printing
- HTTPS / TLS
- HTML forms (`<form>`, `<input>`)
- JavaScript
- CSS (beyond inline Mosaic-era defaults)
- Download manager
- HTTP cache
- Proxy configuration
- Find in page (Ctrl+F)
- History persistence across sessions (in-memory only for v0.1)
- Asynchronous image loading
- macOS platform layer (Cocoa + Metal)
- Linux platform layer (GTK + Cairo)
