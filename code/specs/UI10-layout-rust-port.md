# UI10 — Layout Pipeline Rust Port

## Overview

The layout pipeline — layout-ir (UI02), layout-block (UI07), layout-to-paint
(UI04), document-ast-to-layout (UI06), and layout-text-measure-estimated (UI09)
— currently exists only in TypeScript. All five packages are **pure
computation**: no I/O, no platform dependencies, no event loops. They transform
data structures into other data structures.

The Venture browser is a native Rust binary. It needs these same algorithms in
Rust so it can lay out and paint HTML 1.0 content without embedding a JavaScript
runtime. This spec covers porting all five packages, plus adding a new
Windows-specific text measurer backed by DirectWrite.

The porting is mechanical. The TypeScript originals are imperative, loop-heavy
code with no async, no closures over mutable state, and no prototype chains.
Every function takes explicit arguments and returns explicit results. This
translates to Rust almost line-for-line.

---

## Where It Fits

```
                          TypeScript side             Rust side (this spec)
                          ──────────────             ─────────────────────
  HTML 1.0 parser ──┐
  CommonMark parser ─┤
                     ▼
  DocumentAST ──► document-ast-to-layout ──► LayoutNode tree
                     (UI06)                      │
                                                 ▼
                                          layout-block (UI07)
                                           + TextMeasurer
                                                 │
                                                 ▼
                                          PositionedNode tree
                                                 │
                                                 ▼
                                          layout-to-paint (UI04)
                                                 │
                                                 ▼
                                            PaintScene
                                                 │
                                                 ▼
                                           paint-vm (Rust)
                                                 │
                                                 ▼
                                              pixels
```

The Rust ports mirror the TypeScript packages exactly. Same types, same
algorithms, same test vectors. The only new code is `text-measure-directwrite`,
which calls the Windows DirectWrite API for real font metrics.

---

## Concepts

### Why port instead of FFI?

Calling TypeScript from Rust would require embedding a JS engine (V8, Deno,
quickjs). That defeats the purpose of a native browser. The layout pipeline is
small (~2,000 lines of TypeScript total across all five packages) and
algorithmic. A direct port is simpler, faster, and produces a single static
binary with no runtime dependencies.

### What stays the same

- **Types.** `LayoutNode`, `PositionedNode`, `TextMetrics`, `Display`,
  `PaintScene`, `PaintCommand` — all map 1:1 to Rust structs and enums.
- **Algorithms.** Block layout, inline wrapping, margin collapsing, paint-scene
  generation — all are the same imperative loops.
- **Extension bags.** The `ext` map stays as `HashMap<String, serde_json::Value>`.
  This preserves the open-schema design where each algorithm reads what it needs
  and ignores the rest.
- **Test vectors.** Every TypeScript test case ports to a Rust `#[test]`.

### What changes

- **Ownership.** TypeScript passes trees by reference with shared ownership. Rust
  uses owned trees (`Vec<LayoutNode>`) for the input and builds new owned trees
  for the output. No `Rc`, no `RefCell` — the algorithms are tree-in, tree-out.
- **Trait instead of interface.** `TextMeasurer` becomes a Rust trait. The
  estimated measurer and DirectWrite measurer both implement it.
- **No `any`.** TypeScript's `any` in ext bags becomes `serde_json::Value`, which
  is typed at runtime but checked at access time. Helper methods like
  `ext_f64(node, "block", "marginTop")` wrap the lookup-and-cast pattern.

---

## Public API

### Crate 1: `layout-ir` (port of UI02)

The foundation crate. Defines the core types that every other crate depends on.

```rust
// --- Display enum ---

/// How a node participates in layout.
///
/// Think of this like the CSS `display` property, but simpler. In HTML 1.0,
/// every element is either Block (headings, paragraphs, lists) or Inline
/// (text runs, links, emphasis). The other variants exist for future layout
/// algorithms (flexbox, grid) but are not used by layout-block.
pub enum Display {
    Block,
    Inline,
    Flex,
    Grid,
    None,
}

// --- LayoutNode ---

/// A node in the layout input tree.
///
/// This is what producers (document-ast-to-layout, mosaic-ir-to-layout) build.
/// It describes *what* to lay out, not *where* it goes. The layout algorithm
/// reads this tree and produces a PositionedNode tree with resolved coordinates.
///
/// Extension properties live in `ext`. For example, a block-layout node might
/// carry `ext["block"]["marginTop"] = 12.0`. The layout-block algorithm reads
/// those; layout-flexbox would ignore them. This keeps the core type small and
/// algorithm-agnostic.
pub struct LayoutNode {
    pub display: Display,
    pub children: Vec<LayoutNode>,
    pub content: Option<String>,
    pub ext: HashMap<String, serde_json::Value>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
}

// --- PositionedNode ---

/// A node in the layout output tree, with resolved position and size.
///
/// Every field is concrete — no Options, no unknowns. The layout algorithm has
/// decided where this node goes and how big it is.
pub struct PositionedNode {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub children: Vec<PositionedNode>,
    pub content: Option<String>,
    pub ext: HashMap<String, serde_json::Value>,
}

// --- TextMeasurer trait ---

/// Measures text for layout purposes.
///
/// The layout algorithm calls this whenever it encounters a text node and needs
/// to know how wide and tall the text will be at a given font size, constrained
/// to a maximum width (for line wrapping).
///
/// Two implementations exist:
/// - `EstimatedMeasurer` — zero-dependency, uses average character widths.
///   Deterministic and fast. Used in tests and on platforms without font APIs.
/// - `DirectWriteMeasurer` — calls Windows DirectWrite for real font metrics.
///   Used in the Venture browser on Windows.
pub trait TextMeasurer {
    fn measure(&self, text: &str, font_ref: &str, font_size: f64, max_width: f64) -> TextMetrics;
}

/// The result of measuring a block of text.
pub struct TextMetrics {
    pub width: f64,
    pub height: f64,
    pub line_count: usize,
}
```

**Location:** `code/packages/rust/layout-ir/`

---

### Crate 2: `layout-block` (port of UI07)

Block and inline flow layout — the layout algorithm used for document content.

```rust
/// Lay out a tree of LayoutNodes using block-and-inline flow.
///
/// This is the layout algorithm that web browsers use for normal document flow:
///
/// - **Block-level children** stack vertically. Each block child gets the full
///   `available_width` (minus its own margins/padding). Its height is determined
///   by its content.
///
/// - **Inline-level children** flow left-to-right within a line. When the
///   accumulated width exceeds `available_width`, the line wraps. Text nodes
///   are measured using the `measurer` to determine how many words fit on each
///   line.
///
/// - **Margin collapsing:** When two adjacent block elements have margins, the
///   space between them is the *larger* of the two margins, not the sum. This
///   matches CSS behavior and prevents double-spacing between paragraphs.
///
/// - **Padding and borders** are read from `ext["block"]`:
///   `paddingTop`, `paddingRight`, `paddingBottom`, `paddingLeft`,
///   `borderTopWidth`, `borderRightWidth`, `borderBottomWidth`, `borderLeftWidth`.
///
/// The function returns a PositionedNode tree with resolved x, y, width, height
/// for every node.
pub fn layout_block(
    node: &LayoutNode,
    available_width: f64,
    measurer: &dyn TextMeasurer,
) -> PositionedNode
```

**Location:** `code/packages/rust/layout-block/`

---

### Crate 3: `layout-to-paint` (port of UI04)

Converts the positioned layout tree into a flat list of paint instructions.

```rust
/// Convert a positioned layout tree into a PaintScene.
///
/// This is the bridge between layout (which knows about boxes and text) and
/// painting (which knows about rectangles, colors, and glyph runs).
///
/// The conversion is a depth-first tree walk. For each PositionedNode:
///
///   1. If ext["paint"]["backgroundColor"] is set → emit a PaintRect for the
///      background, filling the node's bounding box.
///
///   2. If ext["paint"]["borderWidth"] > 0 → emit PaintRect commands for each
///      border edge (top, right, bottom, left) with the border color.
///
///   3. If the node has `content` (a text leaf) → emit a PaintGlyphRun with
///      the text, position, font, and color from ext["paint"].
///
///   4. If ext["paint"]["opacity"] < 1.0 → wrap the node's commands in a
///      PaintLayer with the given opacity.
///
///   5. Recurse into children.
///
/// The `device_pixel_ratio` scales all coordinates from logical units to
/// physical pixels. On a 2x Retina display, pass 2.0; on a standard display,
/// pass 1.0.
pub fn layout_to_paint(
    root: &PositionedNode,
    device_pixel_ratio: f64,
) -> PaintScene

/// A scene is an ordered list of paint commands.
pub struct PaintScene {
    pub commands: Vec<PaintCommand>,
}

/// A single paint instruction.
pub enum PaintCommand {
    /// Fill a rectangle with a solid color.
    Rect {
        x: f64, y: f64, width: f64, height: f64,
        color: String,
        corner_radius: f64,
    },
    /// Draw a run of text glyphs.
    GlyphRun {
        x: f64, y: f64,
        text: String,
        font: String,
        font_size: f64,
        color: String,
    },
    /// Begin an opacity group. All commands until the matching PopLayer
    /// are composited at the given opacity.
    PushLayer { opacity: f64 },
    /// End an opacity group.
    PopLayer,
}
```

**Location:** `code/packages/rust/layout-to-paint/`

---

### Crate 4: `document-ast-to-layout` (port of UI06)

Converts a DocumentAST (the output of commonmark-parser or html1.0-parser) into
a LayoutNode tree ready for block layout.

```rust
/// Convert a document tree into a layout tree.
///
/// The `theme` controls all visual properties: fonts, sizes, colors, spacing.
/// This keeps the conversion pure — no hard-coded magic numbers, no reading
/// from CSS files, no platform queries. Everything comes from the theme.
///
/// Mapping from document nodes to layout nodes:
///
///   DocumentNode         → LayoutNode
///   ─────────────          ──────────
///   Heading(level)       → Block, font = theme.heading_font,
///                           size = theme.heading_sizes[level - 1],
///                           marginBottom = theme.paragraph_spacing
///   Paragraph            → Block, font = theme.body_font,
///                           size = theme.body_font_size,
///                           marginBottom = theme.paragraph_spacing
///   Text(string)         → Inline leaf with content = string
///   Link(href)           → Inline, ext["paint"]["color"] = theme.link_color
///   Emphasis             → Inline, ext["paint"]["fontStyle"] = "italic"
///   Strong               → Inline, ext["paint"]["fontWeight"] = "bold"
///   CodeBlock(lang, src) → Block, font = theme.code_font,
///                           size = theme.code_font_size,
///                           ext["paint"]["backgroundColor"] = "#E8E8E8"
///   InlineCode(src)      → Inline, font = theme.code_font,
///                           size = theme.code_font_size
///   List(ordered, items) → Block with indentation,
///                           each item gets a bullet/number prefix
///   HorizontalRule       → Block, height = 1.0,
///                           ext["paint"]["backgroundColor"] = "#808080"
///
pub fn document_to_layout(
    doc: &DocumentNode,
    theme: &DocumentTheme,
) -> LayoutNode

/// Visual theme for document rendering.
///
/// These defaults produce something close to NCSA Mosaic (1993): Times New
/// Roman body text, gray background, blue links, purple visited links.
pub struct DocumentTheme {
    pub body_font: String,
    pub body_font_size: f64,
    pub heading_font: String,
    pub heading_sizes: [f64; 6],
    pub code_font: String,
    pub code_font_size: f64,
    pub line_height: f64,
    pub paragraph_spacing: f64,
    pub link_color: String,
    pub visited_link_color: String,
    pub background_color: String,
}

impl Default for DocumentTheme {
    fn default() -> Self {
        Self {
            body_font: "Times New Roman".into(),
            body_font_size: 14.0,
            heading_font: "Times New Roman".into(),
            heading_sizes: [24.0, 20.0, 18.0, 16.0, 14.0, 12.0],
            code_font: "Courier New".into(),
            code_font_size: 13.0,
            line_height: 1.4,
            paragraph_spacing: 12.0,
            link_color: "#0000EE".into(),
            visited_link_color: "#551A8B".into(),
            background_color: "#C0C0C0".into(),
        }
    }
}
```

**Location:** `code/packages/rust/document-ast-to-layout/`

---

### Crate 5: `text-measure-directwrite` (new, Windows-specific)

Native Windows text measurement implementing the `TextMeasurer` trait. This is
the only crate in the set that has platform dependencies.

```rust
/// Real text measurement using Windows DirectWrite.
///
/// DirectWrite is the modern Windows text rendering API. It handles font
/// fallback, complex scripts (Arabic, CJK), ligatures, and kerning. We use
/// it here for a single purpose: given a string, a font, a size, and a
/// max width, how wide and tall is the resulting text block?
///
/// Internally:
///   1. Create an IDWriteFactory (cached for the lifetime of the measurer).
///   2. For each (font_ref, font_size) pair, create an IDWriteTextFormat
///      (cached in a HashMap).
///   3. For each measure() call, create an IDWriteTextLayout with the text
///      and max_width constraint.
///   4. Call GetMetrics() to read width, height, and lineCount.
///   5. Return TextMetrics.
///
/// The IDWriteTextLayout is not cached — it is cheap to create and specific
/// to each (text, maxWidth) pair.
pub struct DirectWriteMeasurer {
    factory: /* IDWriteFactory COM pointer */,
    format_cache: HashMap<(String, OrderedFloat<f64>), /* IDWriteTextFormat */>,
}

impl TextMeasurer for DirectWriteMeasurer {
    fn measure(&self, text: &str, font_ref: &str, font_size: f64, max_width: f64) -> TextMetrics;
}
```

This crate also re-exports an `EstimatedMeasurer` — a port of
layout-text-measure-estimated (UI09) — as a zero-dependency fallback:

```rust
/// Estimated text measurement using average character widths.
///
/// This is a port of the TypeScript layout-text-measure-estimated package.
/// It uses a table of average character widths (as a fraction of font size)
/// to estimate text dimensions without any platform font API.
///
/// The estimates are rough but deterministic. A monospace font at 13px gives
/// ~7.8px per character. A proportional font at 14px gives ~6.5px per
/// character on average. These numbers come from measuring common fonts on
/// Windows and macOS and averaging the results.
///
/// Primary use cases:
/// - Unit tests (deterministic, no platform dependency)
/// - Fallback on platforms without a native text API
/// - Server-side rendering where fonts are unavailable
pub struct EstimatedMeasurer;

impl TextMeasurer for EstimatedMeasurer {
    fn measure(&self, text: &str, font_ref: &str, font_size: f64, max_width: f64) -> TextMetrics;
}
```

**Location:** `code/packages/rust/text-measure-directwrite/`

---

## Testing Strategy

### Port TypeScript tests 1:1

Every existing TypeScript test case becomes a Rust `#[test]`. The test name,
input, and expected output stay the same. This is the primary correctness
guarantee: if the Rust port produces the same output as the TypeScript original
for every test vector, the port is correct.

### Use EstimatedMeasurer for deterministic tests

All layout tests use `EstimatedMeasurer`, not `DirectWriteMeasurer`. This makes
tests deterministic across platforms and CI environments. The estimated measurer
produces the same numbers on Windows, macOS, and Linux.

### Float tolerance

Layout produces `f64` coordinates. Comparing floats for exact equality is
fragile. All assertions use an epsilon of `1e-6`:

```rust
fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6,
        "expected {expected}, got {actual}");
}
```

### Cross-validation with TypeScript

For a set of representative inputs (a simple paragraph, a heading + paragraph, a
nested list, a code block), run both the TypeScript and Rust pipelines and
compare the PositionedNode trees field-by-field. This catches semantic drift
where individual tests pass but the overall behavior diverges.

### DirectWrite-specific tests

The DirectWrite measurer gets its own test suite that:

1. Measures known strings ("Hello, world!" in Times New Roman 14px) and checks
   that width and height are within reasonable bounds (not zero, not absurdly
   large).
2. Verifies that line wrapping occurs: a long string with `max_width = 100.0`
   should produce `line_count > 1`.
3. Compares against the estimated measurer to ensure they agree within 20% for
   common Latin text. (They will diverge for CJK or complex scripts, and that is
   expected.)

These tests are gated behind `#[cfg(target_os = "windows")]` since DirectWrite
is Windows-only.

---

## Scope

### In scope

- Rust port of `layout-ir` (UI02) — core types and traits
- Rust port of `layout-block` (UI07) — block + inline flow layout
- Rust port of `layout-to-paint` (UI04) — positioned tree to paint scene
- Rust port of `document-ast-to-layout` (UI06) — document AST to layout tree
- Rust port of `layout-text-measure-estimated` (UI09) — zero-dep text measurer
- New crate: `text-measure-directwrite` — Windows DirectWrite text measurer
- 1:1 test ports from TypeScript to Rust
- Cross-validation test harness

### Out of scope

- **layout-flexbox (UI03):** Not needed for HTML 1.0 content. Block + inline
  flow covers everything in the HTML 1.0 spec (headings, paragraphs, lists,
  preformatted text, horizontal rules, links, images).
- **layout-grid (UI08):** Same reason — no CSS Grid in HTML 1.0.
- **CSS property resolution:** The Venture browser uses `DocumentTheme` directly,
  not a CSS cascade. CSS support is a separate future effort.
- **Animation / transitions:** Static layout only. The Venture browser does not
  animate layout changes.
- **Incremental relayout:** Every layout pass processes the full tree. For HTML
  1.0 documents (which are small), this is fast enough. Incremental relayout is
  an optimization for a future spec.
- **Non-Windows text measurement:** CoreText (macOS), FreeType/HarfBuzz (Linux)
  measurers are future crates. The EstimatedMeasurer covers non-Windows platforms
  for now.
