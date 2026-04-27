# MD00 — CommonMark to Metal: End-to-End Integration

## Overview

MD00 is the **integration spec** that ties the CommonMark parser,
document AST, layout engine, text pipeline (TXT-series), font
pipeline (FNT-series), and paint backend into a single working
rendering application: **native Markdown rendering on macOS
through Metal**.

```
┌──────────────────────────────────────────────────────────────┐
│  Input                                                        │
│  ─────                                                        │
│  A string of CommonMark-formatted Markdown text.             │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  commonmark-parser                                            │
│  ────────────────                                             │
│  Produces a DocumentAST (tree of blocks + inlines).          │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  document-ast-to-layout                                       │
│  ──────────────────────                                       │
│  Converts DocumentAST → LayoutIR (UI02) tree. Each heading,  │
│  paragraph, code block, list item, blockquote becomes a      │
│  LayoutNode with TextContent or ImageContent.                │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  layout                                                       │
│  ──────                                                       │
│  Runs the flexbox-style layout algorithm (TXT01 metrics +    │
│  TXT02 or TXT04 shaper for text measurement).                │
│  Produces PositionedNode[] tree with resolved (x, y, w, h).  │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  layout-to-paint (UI04, amended)                              │
│  ──────────────                                               │
│  Walks PositionedNode tree, shapes each TextContent using    │
│  the provided TextShaper, emits PaintScene with              │
│  PaintGlyphRun instructions.                                  │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  paint-vm-metal                                               │
│  ──────────────                                               │
│  Dispatches each PaintInstruction. For PaintGlyphRun,        │
│  routes by font_ref scheme (P2D06 amendment):                │
│    "coretext:..."     → CTFontDrawGlyphs                     │
│    "font-parser:..."  → FNT02 outline → FNT03 coverage →     │
│                         tint + Metal texture blit            │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             ▼
                        Pixels on screen
```

MD00 is not a new package. It is a spec that documents the
**composition** of existing packages: which trait goes where,
which font_ref binding the consumer chose, which two
paths through the stack are available, and what the expected
end-to-end behavior looks like.

This spec is the acceptance test for the TXT + FNT work. Once
MD00 is implementable — once an application can follow these
arrows and produce pixels from Markdown input — the
infrastructure PRs that built up to this point are validated.

---

## Scope

MD00 covers:

- The reference application architecture for a Markdown-to-
  Metal rendering binary.
- The two available paths through the stack (device-dependent
  via CoreText, device-independent via TXT02+FNT02+FNT03).
- The CSS-like style sheet applied to CommonMark constructs
  (what font, what size, what spacing for each block type).
- The assertions an end-to-end test suite must make to prove
  the integration works.

MD00 does NOT:

- Implement anything. Every package referenced exists (or is
  specced).
- Define the CommonMark parser behavior — that's the
  `commonmark-parser` package's job.
- Define the layout algorithm — that's the layout engine.
- Define how Metal is called — that's `paint-vm-metal`.
- Cover other platforms (Windows DirectWrite path, Linux Pango
  path). Those are peers of MD00 and can be specified as MD00b,
  MD00c in the future if useful.

---

## Reference application

The reference implementation lives at
`code/programs/swift/markdown-reader/` (proposed). It is a macOS
SwiftUI application:

```
markdown-reader/
├── Package.swift
├── Sources/
│   └── MarkdownReader/
│       ├── MarkdownView.swift       // SwiftUI view that embeds a Metal layer
│       ├── Renderer.swift            // Owns the pipeline, drives a frame
│       ├── Pipeline.swift            // The composition described in this spec
│       └── Styles.swift              // CommonMark → layout style sheet
└── Tests/
    └── MarkdownReaderTests/
        ├── SnapshotTests.swift       // Rendered PNG comparison tests
        └── PipelineTests.swift       // Unit tests per pipeline step
```

The entry point is a `SwiftUI.View` that wraps a `CAMetalLayer`
and renders a given Markdown string into it:

```swift
import SwiftUI
import MarkdownReader

struct ContentView: View {
    let markdown = """
    # Hello
    
    This is **bold** and this is *italic*.
    
    - Lists work.
    - With multiple items.
    """

    var body: some View {
        MarkdownView(source: markdown)
            .frame(minWidth: 400, minHeight: 300)
    }
}
```

The SwiftUI view is a thin wrapper; all the real work is in
`Pipeline.swift`.

---

## The pipeline, step by step

### Step 1 — Parse

```swift
let ast: DocumentAST = CommonMarkParser.parse(markdown)
```

Uses the existing `commonmark-parser` Swift port (if present; if
not, the Rust port via a thin Swift binding). Output is a
`DocumentAST` as defined in the `document-ast` package.

### Step 2 — Convert AST → Layout

```swift
let layoutTree: [LayoutNode] = DocumentAstToLayout.convert(
    ast,
    style: Styles.default  // see "Style sheet" below
)
```

Uses the `document-ast-to-layout` package. Each AST block becomes
a `LayoutNode` with appropriate `TextContent` / `ImageContent`
and an `ext["paint"]` bundle describing backgrounds, borders,
padding — everything the layout phase does not resolve but the
paint phase needs.

### Step 3 — Set up the text trio

The caller picks one of two paths here. **MD00 v1's reference
app uses the CoreText path** — fastest to first pixel on macOS.
The font-parser path is available for tests and for anyone who
wants reproducibility.

#### Path A: CoreText (default in the reference app)

```swift
// Resolve system fonts via CoreText
let resolver  = CoreTextFontResolver()           // TXT05a-coretext
let metrics   = CoreTextMetrics(resolver: resolver)     // TXT03a
let shaper    = CoreTextShaper(resolver: resolver)      // TXT03a
```

All three share the CoreText binding. font_ref values for shaped
runs will be `coretext:<ps_name>@<size>`.

#### Path B: font-parser (reproducible)

```swift
// Load fonts from bundled TTF bytes
let resolver = FontParserResolver()             // TXT05-font-parser
resolver.register(family: "Helvetica Neue",
                  weight: 400, style: .normal,
                  bytes: helveticaNeueRegularTTF)
resolver.register(family: "Helvetica Neue",
                  weight: 700, style: .normal,
                  bytes: helveticaNeueBoldTTF)
// ... register all styles the layout will request

let metrics = FontParserMetrics(resolver: resolver)   // TXT01
let shaper  = NaiveShaper(resolver: resolver)         // TXT02 v1
// (Later: swap NaiveShaper for HarfBuzzShaper from TXT04 when
// advanced shaping lands — same trait, hot-swap works.)
```

All three share the `font-parser:` binding. font_ref values will
be `font-parser:<blake2b-hash>`.

### Step 4 — Layout

```swift
let positionedTree = Layout.flexbox(
    layoutTree,
    constraints: LayoutConstraints(
        maxWidth:  viewportWidth,
        maxHeight: viewportHeight
    ),
    measure: TextMeasurer(shaper: shaper, metrics: metrics)
)
```

The layout engine produces a tree of `PositionedNode` values
with resolved (x, y, width, height). Text measurement is
delegated to a `TextMeasurer` that wraps the shaper — this is
the TXT00 measurement pattern (shape + sum advances).

### Step 5 — Emit PaintScene

```swift
let scene = LayoutToPaint.convert(
    positionedTree,
    options: LayoutToPaintOptions(
        width:     viewportWidth,
        height:    viewportHeight,
        background: .white,
        devicePixelRatio: 2.0,   // Retina display
        shaper:    shaper,       // required for TextContent → PaintGlyphRun
        metrics:   metrics,      // ditto
        resolver:  resolver      // ditto
    )
)
```

This is where the UI04 amendment kicks in. Each `TextContent`
node is shaped by `shaper` and emitted as a
`PaintGlyphRun { font_ref, glyphs, fill, ... }` carrying the
binding-scoped font_ref from step 3.

### Step 6 — Render via PaintVM-Metal

```swift
let vm = PaintVmMetal(device: metalDevice)

// Register the font handle under the font_ref key that the scene
// carries. For the CoreText path:
vm.registry.register(
    fontRef: "coretext:HelveticaNeue-Regular@16.0",
    handle:  coreTextFontHandle
)
// For the font-parser path, register the parsed FontFile:
// vm.registry.register(
//     fontRef: "font-parser:abc123def...",
//     handle:  fontParserFontHandle
// )

vm.render(scene: scene, target: metalDrawable)
```

The registry pre-registration is how the paint backend resolves
font_ref keys at dispatch time (see P2D06 amendment, §"Why the
registry lookup"). In the reference app, the same code that
built the FontResolver populates the paint VM's registry —
there's no mystery indirection.

---

## Style sheet

A minimal CSS-like mapping from CommonMark construct to layout
style. This is the reference style; applications can customize.

```swift
struct Style {
    var fontFamily: [String]
    var fontSize: Float           // in points (user-space units)
    var fontWeight: Int           // 100..900
    var fontStyle: FontStyle
    var color: Color
    var marginTop: Float
    var marginBottom: Float
    var lineHeight: Float         // multiplier, e.g. 1.5
    var textAlign: TextAlign
    var backgroundColor: Color?
    var paddingX: Float
    var paddingY: Float
}

enum Styles {
    static let body = Style(
        fontFamily: ["Helvetica Neue", "Helvetica", "sans-serif"],
        fontSize: 16.0,
        fontWeight: 400,
        fontStyle: .normal,
        color: .black,
        marginTop: 0, marginBottom: 16,
        lineHeight: 1.5,
        textAlign: .left,
        paddingX: 0, paddingY: 0
    )

    static let h1 = body.with(fontSize: 32.0, fontWeight: 700,
                              marginTop: 24, marginBottom: 16)
    static let h2 = body.with(fontSize: 24.0, fontWeight: 700,
                              marginTop: 20, marginBottom: 12)
    static let h3 = body.with(fontSize: 18.0, fontWeight: 600,
                              marginTop: 16, marginBottom: 10)

    static let p  = body

    static let code = body.with(
        fontFamily: ["Menlo", "Consolas", "monospace"],
        fontSize: 14.0,
        backgroundColor: .grayLight,
        paddingX: 4, paddingY: 2
    )

    static let blockquote = body.with(
        color: .grayMedium,
        paddingX: 16,
        // left-border is added via a PaintRect in the border slot
    )

    static let listItem = body.with(marginTop: 0, marginBottom: 4)
}
```

Rich inline styles (bold, italic, links, inline code) compose
these base styles. Bold = parent `fontWeight` + 300 (capped at
900). Italic = parent with `fontStyle: .italic`. Links = parent
with `color: .blue, underline: true`. Inline code = a monospace
override with `backgroundColor: .grayLight`.

---

## End-to-end tests

MD00's acceptance test is a snapshot suite at
`tests/MarkdownReaderTests/SnapshotTests.swift`:

```swift
func testHeadingAndParagraph() {
    let md = """
    # Hello world
    
    A short paragraph with **bold** and *italic*.
    """
    let pixels = Pipeline.render(md, viewport: CGSize(width: 640, height: 480))
    XCTAssertSnapshotsEqual(pixels, "heading_and_paragraph.png")
}
```

Snapshots live at `tests/snapshots/<name>.png`. The first run
writes the snapshot; subsequent runs compare with a per-pixel
tolerance of ±2 in each channel (for antialiasing variance).

Target snapshots:

| Fixture                | What it tests                                           |
|------------------------|---------------------------------------------------------|
| `empty.png`            | Empty Markdown input — blank white viewport             |
| `single_heading.png`   | `# Hello` — one H1 at top of viewport                  |
| `paragraph.png`        | Plain paragraph with word wrapping                      |
| `bold_italic.png`      | Inline formatting                                       |
| `heading_and_para.png` | Block stacking + consistent baseline                    |
| `unordered_list.png`   | Bullet points with proper indentation                   |
| `code_block.png`       | Fenced code block with monospace font and background    |
| `blockquote.png`       | Quote with left border and indented text                |
| `inline_code.png`      | Inline code with monospace font and subtle background   |
| `emoji.png`            | A basic emoji codepoint (tests TXT04 fallback — later) |
| `unicode_latin.png`    | "Café" — tests accent composition                       |

The snapshot suite runs on CI. Two separate runs: one with the
CoreText path, one with the font-parser path. The CoreText
snapshots may differ across macOS versions (documented and
accepted); the font-parser snapshots MUST be identical across
machines. Any drift on the font-parser track is a bug.

### Per-layer unit tests

Each pipeline step is also tested in isolation:

1. **Parse** — known-good Markdown fixtures produce known-good
   AST JSON.
2. **AST → Layout** — known-good ASTs produce expected
   LayoutNode trees.
3. **Layout** — known-good LayoutNode trees with a mock
   TextMeasurer produce expected PositionedNode positions.
4. **Layout → Paint** — known-good PositionedNode trees with a
   mock TextShaper produce expected PaintScene contents.
5. **Paint** — PaintScene renders to a Metal texture
   byte-compared with a reference.

---

## Performance expectations

Reference hardware: Apple M1 MacBook, 16GB RAM, macOS 14.

| Operation                                   | Budget      |
|---------------------------------------------|-------------|
| Parse 10 KB of Markdown                     | < 2 ms      |
| AST → Layout tree (1000 nodes)              | < 5 ms      |
| Layout with text measurement (1000 nodes)   | < 50 ms     |
| Layout → PaintScene (1000 paint instructions)| < 20 ms    |
| PaintVM-Metal render (1000 instructions)    | < 16.67 ms (60 fps) |
| **End-to-end for a typical README.md**      | **< 100 ms** |

Scrolling an already-rendered document should achieve 60fps
easily — the Metal backend dispatches pre-computed
PaintGlyphRuns with cached glyph bitmaps. Re-layout on viewport
resize is the only expensive operation; it's acceptable to
debounce to ~30 fps during active drag.

---

## What this unblocks

With MD00 implementable:

- A working Markdown viewer on macOS that uses **only the
  coding-adventures stack** for text rendering. No SwiftUI
  Text views, no attributed strings, no platform-specific
  shortcuts.
- A template for the other platforms: MD00b (Windows /
  DirectWrite), MD00c (Linux / Pango), MD00d (browser /
  Canvas via WASM).
- A proving ground for LaTeX rendering — once the CommonMark
  path works, LaTeX is "replace the parser and add math fonts".
- A reference for anyone integrating a new paint backend — the
  MD00 test suite tells you "here is what a correct
  integration produces".

---

## Non-goals

- **Interactive editing.** The viewer renders Markdown; a
  Markdown *editor* is a separate concern (input handling,
  cursor positioning, incremental updates).
- **Scrolling inside the rendered document.** The view is
  fixed-size in v1. A scroll view is a BR01 / Venture-browser
  concern.
- **Hyperlink clicks.** No hit testing in v1; links render as
  styled text but don't respond to interaction.
- **Image loading from URLs.** Images in Markdown (`![alt](url)`)
  are rendered only when the URL points to an embedded bundle
  resource. Network fetching is out of scope.
- **CSS / custom themes beyond the built-in style sheet.** A
  custom theme API can be added once the built-in one is
  validated.
- **Syntax highlighting in code blocks.** Code blocks render in
  monospace with no coloring. Highlighting can layer on top via
  PaintGlyphRun `fill` variance per-run.

---

## Relationship to the whole text-rendering arc

MD00 is the integration layer at the top of everything the
TXT- and FNT-series specs built. The dependency tree:

```
MD00
 ├── commonmark-parser                 (existing)
 ├── document-ast                      (existing)
 ├── document-ast-to-layout            (existing)
 ├── layout-ir + layout algorithm      (existing)
 ├── UI04 layout-to-paint              (amended for pre-shaped glyphs)
 │   ├── text-interfaces (TXT00)        ← pluggable shaper/metrics
 │   ├── font-resolver (TXT05)          ← FontQuery → FontHandle
 │   ├── CoreText path (TXT03a) OR
 │   │   ├── text-metrics-font-parser (TXT01)
 │   │   └── text-shaper-naive (TXT02) OR text-shaper-harfbuzz (TXT04)
 │   └── paint-instructions (P2D00)     ← PaintGlyphRun wire format
 ├── paint-vm-metal                    (existing; amended for P2D06 routing)
 │   ├── paint-vm (P2D01)              (existing)
 │   ├── glyph-parser (FNT02)           ← outline extraction
 │   └── glyph-rasterizer (FNT03)       ← outline → pixels
 └── font-parser (FNT00)                ← shared foundation
```

Every arrow in this tree has either an existing implementation
or a spec with a clear implementation path. MD00 is the proof
that the arrows compose.

---

## Open questions

- **Which CommonMark parser port backs the Swift app.** The
  Rust `commonmark-parser` crate with a Swift FFI binding is the
  likely path; a native Swift re-implementation is possible but
  duplicative. Decide at implementation time.

- **How to ship fonts for the font-parser path.** Embedding the
  reference TTFs (FNT05's test font + a couple of display faces)
  into the app bundle is the simplest option. ~50 KB extra
  binary size. A future variant can stream fonts from a CDN.

- **Whether MD00 should branch into MD00a/b/c/d** immediately.
  Current recommendation: write MD00 (this spec) for macOS /
  CoreText first. Other platforms follow once the pattern is
  validated. The spec structure can grow into sub-specs without
  invalidating this document.

- **Whether the paint VM's font registry should be a service
  locator or explicit per-render-call.** Current design: a
  long-lived registry on the PaintVM. Alternative: pass the
  registry as an argument to `render(scene, target, registry)`.
  The long-lived registry is simpler and matches OS text
  infrastructure patterns; the argument-passing form is more
  explicit. Chose the long-lived form; revisit if it causes
  problems in the implementation.
