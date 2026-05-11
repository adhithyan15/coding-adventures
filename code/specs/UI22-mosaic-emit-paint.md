# UI22 — mosaic-emit-paint: Paint VM Backend

**Status:** Planned → Implementing  
**Layer:** UI  
**Depends on:** UI00 (mosaic-analyzer), P2D00 (paint-instructions), barcode-2d (render dispatch)

---

## 1. Purpose

`mosaic-emit-paint` is a **direct-to-PaintScene** backend for the Mosaic compiler.
Given a Mosaic component source string and a canvas size, it produces a
`PaintScene` (a list of `PaintInstruction` values) that can be rasterized to a
PNG byte stream without any browser, Qt runtime, or display server.

This is the first Mosaic backend to deliver **native raster output**. All other
backends (HTML, WebComponent, React, Qt) require an external runtime to produce
pixels. `mosaic-emit-paint` closes that gap by walking the `MosaicFile` IR
directly and computing pixel-level layout using a naive box model, then emitting
the result as the `PaintInstruction` vocabulary already understood by the
`barcode-2d` rendering engine.

**Key design decision:** this backend does **not** go through `MosaicVM` or the
`MosaicRenderer` trait. Direct IR walking gives full control over the layout
algorithm — the renderer can compute child sizes before deciding how to paint the
parent, which a callback-driven API does not easily support.

```text
.mosaic source
     │
     ▼
mosaic-analyzer  →  MosaicFile (IR)
     │
     ▼
mosaic-emit-paint  (layout pass — walks MosaicFile.component.root directly)
     │
     ├──  barcode-2d::render_scene_png()  →  PNG bytes  (Vec<u8>)
     └──  PaintScene (Vec<PaintInstruction>)             (for compositing)
```

### 1.1 Design constraints

- **No external dependencies at runtime.** Once the crate is compiled, producing
  a PNG requires no system libraries beyond those already in the Rust workspace.
  `barcode-2d` provides the rasterizer.
- **Naive layout only.** A full flexbox implementation lives in `layout-flexbox`
  (UI03). This crate deliberately uses a simple proportional-split algorithm so
  that the layout code stays short and auditable. The algorithm is documented
  completely in §4 so that readers can understand it without referring to external
  specifications.
- **Slot placeholders instead of fixture injection.** Unlike the HTML backend
  (UI18), this backend does not accept a fixture JSON file. Slot references are
  replaced with type-appropriate visible placeholders. This makes the backend
  useful for design previews without requiring fixture data.

---

## 2. Architecture

```text
                  ┌──────────────────────────────────┐
  .mosaic source  │  mosaic-emit-paint                │
  ───────────────►│                                   │
  (width, height) │  1. mosaic_analyzer::analyze()    │
                  │        → MosaicFile               │
                  │                                   │
                  │  2. layout_node()                 │
                  │        → LayoutBox tree           │
                  │                                   │
                  │  3. collect_instructions()        │
                  │        → PaintScene               │
                  └──────────────────┬───────────────┘
                                     │ PaintScene
                         ┌───────────┴────────────┐
                         │                        │
                         ▼                        ▼
               barcode-2d                  caller (compositing,
               render_scene_png()          MosaicBook, tests, etc.)
               → Vec<u8> (PNG)
```

The two phases are:

**Phase 1 — Layout pass:** `layout_node(node, x, y, w, h)` walks the
`MosaicNode` tree recursively. Each call returns a `LayoutBox` that knows its
pixel bounds and the `PaintInstruction` values for that node's visual
representation. Children are laid out before the parent's instructions are
finalized, so a `Column` can know each child's actual height before deciding how
to distribute space.

**Phase 2 — Flatten:** `collect_instructions(layout_box)` performs a pre-order
walk of the `LayoutBox` tree, appending each node's instructions to a flat
`Vec<PaintInstruction>`. Pre-order ensures parents paint before children (painter's
algorithm), so containers appear behind their content.

---

## 3. Public API

```rust
/// Compile a Mosaic component source to a PaintScene.
///
/// `width` and `height` define the canvas bounds in logical pixels. The
/// top-level component is laid out to fill the entire canvas.
///
/// Returns `Err` if the source cannot be parsed or analyzed.
pub fn render_scene(source: &str, width: f64, height: f64) -> Result<PaintScene, String>;

/// Compile a Mosaic component source to PNG bytes.
///
/// Equivalent to `render_scene(source, width, height)` followed by
/// `barcode_2d::render_scene_png(scene, width as u32, height as u32)`.
pub fn render_png(source: &str, width: f64, height: f64) -> Result<Vec<u8>, String>;

/// Compile a Mosaic component source to a PaintScene using default canvas dimensions.
///
/// Canvas size: DEFAULT_WIDTH × DEFAULT_HEIGHT (400 × 300 logical pixels).
pub fn render_scene_with_defaults(source: &str) -> Result<PaintScene, String>;

/// Compile a Mosaic component source to PNG bytes using default canvas dimensions.
pub fn render_png_with_defaults(source: &str) -> Result<Vec<u8>, String>;
```

All four functions are pure: given the same input, they produce the same output.
They do not mutate global state, perform I/O, or allocate long-lived resources.

---

## 4. Layout Algorithm

The layout algorithm is a **naive proportional box model**. It is intentionally
not a full flexbox implementation — that lives in UI03 (`layout-flexbox`). The
goal here is a small, readable algorithm that produces reasonable-looking
previews for all node types without edge-case complexity.

### 4.1 Representation

```rust
/// The output of the layout pass for a single node.
struct LayoutBox {
    /// Top-left corner of this box in canvas coordinates (logical pixels).
    x: f64,
    y: f64,
    /// Dimensions of this box.
    width: f64,
    height: f64,
    /// Paint instructions for this node only (not children).
    instructions: Vec<PaintInstruction>,
    /// Laid-out children in paint order (parent before children).
    children: Vec<LayoutBox>,
}
```

### 4.2 Entry point

```rust
fn layout_node(
    node: &MosaicNode,
    x: f64,
    y: f64,
    available_width: f64,
    available_height: f64,
) -> LayoutBox
```

This function dispatches on `node.node_type` (a `MosaicNodeType` enum whose
variants map to the primitive names below).

### 4.3 Layout rules per node type

The following table describes how each node type distributes `available_width`
and `available_height` to its children. "Parent width" means `available_width`;
"parent height" means `available_height`.

#### Box / Stack

Children are all placed at the parent's origin `(x, y)` and each receives the
full parent dimensions. This causes children to overlay each other (the "stack"
effect). The box itself occupies the full parent area.

```text
┌────────────────────────────────┐
│ Box (x=0, y=0, w=400, h=300)  │
│ ┌──────────────────────────┐   │
│ │ child A (x=0,y=0,w=400) │   │  ← all children start at same origin
│ └──────────────────────────┘   │
│ ┌──────────────────────────┐   │
│ │ child B (x=0,y=0,w=400) │   │
│ └──────────────────────────┘   │
└────────────────────────────────┘
```

#### Column

Children are stacked vertically. Available height is divided equally among
non-Spacer children. Spacer children share any remaining height after fixed
children are placed.

```text
┌────────────────────────────────┐
│ Column (w=400, h=300)         │
│ ┌──────────────────────────┐   │
│ │ child A (h = 300/3=100)  │   │
│ └──────────────────────────┘   │
│ ┌──────────────────────────┐   │
│ │ child B (h=100)          │   │
│ └──────────────────────────┘   │
│ ┌──────────────────────────┐   │
│ │ Spacer (h=100 remaining) │   │
│ └──────────────────────────┘   │
└────────────────────────────────┘
```

Each child receives the full parent width. Children are laid out top-to-bottom;
each child's `y` origin is the sum of all preceding siblings' heights.

#### Row

Symmetric to Column but horizontal. Available width is divided equally among
non-Spacer children. Each child receives the full parent height. Children are
laid out left-to-right.

#### Spacer

A Spacer inside a Column consumes the remaining height not claimed by siblings.
A Spacer inside a Row consumes the remaining width. Spacers produce no paint
instructions.

If a Spacer appears inside a Box/Stack (where remaining-space semantics do not
apply), it is given zero size.

#### Text

```
height = LINE_HEIGHT_PX    (20 px)
width  = available_width   (spans full parent width)
```

The text content is the slot placeholder string (see §6) or a literal string
from the `content` prop. A single `PaintText` instruction is emitted.

#### Image

Images are not loaded from URLs at paint time — this backend produces design
previews, not live-data renders. The image area is represented as a gray filled
rectangle with a darker stroke, occupying a fixed 100 × 100 px area (or the
available area if it is smaller):

```
width  = min(available_width,  100.0)
height = min(available_height, 100.0)
```

#### Divider

A 1 px tall filled rectangle spanning the full parent width. The fill color is
`#e0e0e0`. Dividers do not participate in Column/Row space distribution — they
always occupy exactly 1 px of height.

#### Icon

A 24 × 24 px rectangle with a light fill (`#e8e8e8`) and a medium-gray stroke
(`#999999`). Icons do not scale with available space. If `available_width` or
`available_height` is less than 24 px, the icon is clipped to the available area.

#### Grid

The Grid node emits a header row and up to three placeholder data rows. It
requires a `list` slot for its data; in this backend the slot value is always
the list type's placeholder (3 example items).

```
total_height = GRID_HEADER_H + (3 × GRID_ROW_H)
GRID_HEADER_H = 28.0 px
GRID_ROW_H    = 24.0 px
```

The header row is a filled rectangle (`#e0e0e0`) spanning the full width, with
a `PaintText` cell per column. Each data row is an unfilled rectangle with a
bottom border (`#e0e0e0`), with a `PaintText` placeholder cell.

### 4.4 Padding

Every container node (Box, Column, Row, Stack, Scroll) applies `PADDING_PX = 8.0`
of inner padding. Children are placed at `(x + PADDING_PX, y + PADDING_PX)` and
receive `(width − 2×PADDING_PX, height − 2×PADDING_PX)` as their available area.

```text
┌────────────────────────────────┐ ← y
│ padding 8px                   │
│  ┌──────────────────────────┐ │ ← y + 8
│  │ child area               │ │
│  │ w = available - 16       │ │
│  └──────────────────────────┘ │
│ padding 8px                   │
└────────────────────────────────┘
  ← x   ← x + 8       w - 8 → →
```

Leaf nodes (Text, Image, Divider, Spacer, Icon) do not apply padding; they occupy
their full assigned area.

---

## 5. Node → Paint Instruction Mapping

| Mosaic node | Paint instruction(s) | Notes |
|-------------|---------------------|-------|
| Box         | `PaintRect { fill: #f8f8f8, stroke: none }` + children | Background fill |
| Stack       | `PaintRect { fill: #f8f8f8, stroke: none }` + children | Same as Box |
| Column      | `PaintRect { fill: transparent, stroke: none }` + children | No background |
| Row         | `PaintRect { fill: transparent, stroke: none }` + children | No background |
| Scroll      | `PaintRect { fill: transparent, stroke: none }` + children | No scroll chrome |
| Text        | `PaintText { text: "…", font_size: FONT_SIZE, fill: #1a1a1a }` | Content or placeholder |
| Image       | `PaintRect { fill: #cccccc, stroke: #999999 }` | Gray placeholder |
| Spacer      | _(nothing)_ | Consumes space, no output |
| Divider     | `PaintRect { fill: #e0e0e0, height: 1.0 }` | 1 px rule |
| Icon        | `PaintRect { fill: #e8e8e8, stroke: #999999, w: 24, h: 24 }` | Square icon box |
| Grid header | `PaintRect { fill: #e0e0e0 }` + `PaintText` per column | Fixed 28 px height |
| Grid row    | `PaintRect { stroke: #e0e0e0 }` + `PaintText` per cell | Fixed 24 px height |

`PaintRect` and `PaintText` refer to the variant names in the `PaintInstruction`
enum from the `paint-instructions` crate (P2D00).

---

## 6. Slot Value Resolution

This backend does not accept fixture JSON. When a node property references a
slot (e.g. `content: @title`), the renderer substitutes a type-appropriate
**placeholder** string:

| Slot type | Placeholder |
|-----------|-------------|
| `text`    | `[slot-name]` e.g. `[title]` |
| `number`  | `0` |
| `bool`    | `true` |
| `image`   | _(gray PaintRect — no text)_ |
| `color`   | `#888888` |
| `list<T>` | 3 placeholder items derived from inner type |
| `node`    | _(empty — no instructions emitted)_ |

For `list<text>` the three placeholder items are:
`["[item 1]", "[item 2]", "[item 3]"]`.

For `list<number>`: `["1", "2", "3"]`.

This gives Grid nodes enough rows to look meaningful in a design preview without
requiring real data.

---

## 7. Color Scheme (Default Theme)

All colors are specified as 6-digit hex strings (no alpha channel in the current
version):

| Role | Color |
|------|-------|
| Canvas background | `#ffffff` |
| Text fill | `#1a1a1a` |
| Box / Stack fill | `#f8f8f8` |
| Column / Row fill | transparent |
| Divider | `#e0e0e0` |
| Grid header fill | `#e0e0e0` |
| Grid row stroke | `#e0e0e0` |
| Image placeholder fill | `#cccccc` |
| Image placeholder stroke | `#999999` |
| Icon placeholder fill | `#e8e8e8` |
| Icon placeholder stroke | `#999999` |

These values mirror common design-system neutral palettes and are chosen to be
legible at all canvas sizes without being visually distracting.

---

## 8. Constants

```rust
pub const DEFAULT_WIDTH:    f64 = 400.0;
pub const DEFAULT_HEIGHT:   f64 = 300.0;

pub const CHAR_WIDTH_PX:    f64 = 8.0;   // Estimated character width for text sizing
pub const LINE_HEIGHT_PX:   f64 = 20.0;  // Height of a single line of text
pub const FONT_SIZE:        f64 = 14.0;  // Font size passed to PaintText
pub const PADDING_PX:       f64 = 8.0;   // Inner padding for container nodes

pub const IMAGE_PLACEHOLDER_SIZE: f64 = 100.0;  // Default image placeholder dimension
pub const ICON_SIZE:        f64 = 24.0;  // Fixed icon placeholder dimension
pub const GRID_HEADER_H:    f64 = 28.0;  // Grid header row height
pub const GRID_ROW_H:       f64 = 24.0;  // Grid data row height
pub const GRID_PLACEHOLDER_ROWS: usize = 3; // Number of placeholder data rows
```

`CHAR_WIDTH_PX` is used only to estimate the width of a `Text` label for display
purposes. Because this backend uses `available_width` for text nodes, the constant
is not currently needed for layout (text always spans the full available width).
It is retained for future use in multi-column text wrapping.

---

## 9. Package

**Crate name:** `mosaic-emit-paint`  
**Version:** 0.1.0  
**Language:** Rust  

**Cargo.toml dependencies:**

```toml
[dependencies]
mosaic-analyzer = { path = "../mosaic-analyzer" }
mosaic-lexer    = { path = "../mosaic-lexer" }
mosaic-parser   = { path = "../mosaic-parser" }
grammar-tools   = { path = "../grammar-tools" }
lexer           = { path = "../lexer" }
directed-graph  = { path = "../directed-graph" }
parser          = { path = "../parser" }
state-machine   = { path = "../state-machine" }
paint-instructions = { path = "../paint-instructions" }
barcode-2d      = { path = "../barcode-2d" }
```

These are all workspace-local crates. There are no external (crates.io) runtime
dependencies.

**Exported symbols:**

```rust
pub use api::{render_scene, render_png, render_scene_with_defaults, render_png_with_defaults};
pub use layout::LayoutBox;
pub use constants::*;
```

---

## 10. Testing Strategy

Tests are organized into three groups: layout correctness, paint instruction
content, and end-to-end PNG validity.

### 10.1 Layout tests

| Test | What it covers |
|------|----------------|
| `test_box_fills_canvas` | Box with no children → LayoutBox covers full canvas |
| `test_column_splits_height_equally` | 3 children in Column → each gets h/3 |
| `test_row_splits_width_equally` | 3 children in Row → each gets w/3 |
| `test_spacer_takes_remaining_height` | Column with 1 fixed child + Spacer → Spacer gets residual |
| `test_spacer_takes_remaining_width` | Row with 1 fixed child + Spacer → Spacer gets residual |
| `test_text_height_is_line_height` | Text node → LayoutBox.height == LINE_HEIGHT_PX |
| `test_image_capped_at_100px` | Image in 400×300 canvas → 100×100 LayoutBox |
| `test_icon_is_24px` | Icon node → LayoutBox 24×24 |
| `test_divider_is_1px_tall` | Divider → LayoutBox.height == 1.0 |
| `test_padding_reduces_child_area` | Column child available_width == parent - 2*PADDING |
| `test_stack_children_at_origin` | Stack with 2 children → both children at same (x,y) |
| `test_nested_column_row` | Column containing a Row → children positions correct |
| `test_grid_height` | Grid → total height == GRID_HEADER_H + 3*GRID_ROW_H |

### 10.2 Paint instruction tests

| Test | What it covers |
|------|----------------|
| `test_box_emits_rect` | Box → at least one PaintRect with fill #f8f8f8 |
| `test_column_emits_rect` | Column → PaintRect (transparent OK) |
| `test_text_emits_paint_text` | Text node → PaintText instruction present |
| `test_text_placeholder_in_output` | `content: @title` → PaintText.text == "[title]" |
| `test_image_emits_gray_rect` | Image → PaintRect fill == #cccccc |
| `test_divider_emits_rect` | Divider → PaintRect fill == #e0e0e0 with height 1 |
| `test_spacer_no_instructions` | Spacer → zero PaintInstruction values |
| `test_icon_emits_rect` | Icon → PaintRect fill == #e8e8e8 with 24×24 size |
| `test_grid_header_and_rows` | Grid → >= 4 PaintRect + >= 4 PaintText instructions |
| `test_slot_number_placeholder` | `number` slot → "0" in PaintText |
| `test_slot_bool_placeholder` | `bool` slot → "true" in PaintText |
| `test_list_slot_three_items` | `list<text>` slot → 3 PaintText items |
| `test_pre_order_paint` | Parent rect appears before child rect in instruction list |

### 10.3 End-to-end PNG tests

| Test | What it covers |
|------|----------------|
| `test_render_png_valid_header` | `render_png_with_defaults(src)` starts with `\x89PNG\r\n\x1a\n` |
| `test_render_png_dimensions` | PNG IHDR chunk encodes correct width × height |
| `test_render_png_default_size` | `render_png_with_defaults` → 400×300 |
| `test_render_png_custom_size` | `render_png(src, 800, 600)` → 800×600 PNG |
| `test_render_png_bad_source_is_err` | Malformed source → `Err(…)` not panic |

---

## 11. Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| UI00 (mosaic-analyzer) | Provides `MosaicFile`, `MosaicNode`, `MosaicSlot` — the IR this backend consumes |
| UI01 (mosaic-vm) | **Not used** — this backend walks the IR directly, bypassing `MosaicVM` |
| UI03 (layout-flexbox) | A future full flexbox layout engine; `mosaic-emit-paint` uses a simplified subset |
| UI04 (layout-to-paint) | Converts layout IR to PaintScene; `mosaic-emit-paint` performs inline layout + paint without a separate layout IR step |
| UI19 (MosaicBook) | The primary consumer; uses Phase 2 (paint/raster tier) preview images |
| P2D00 (paint-instructions) | Defines the `PaintInstruction` enum and `PaintScene` type alias |
| barcode-2d | Provides `render_scene_png()` for rasterizing `PaintScene` to PNG |

---

## 12. v2 Roadmap

- **Fixture injection:** accept an optional `HashMap<String, SlotValue>` so that
  real slot data can be rendered instead of placeholders, matching UI18's fixture
  support.
- **Full flexbox layout:** delegate layout to `layout-flexbox` (UI03) once it
  stabilizes, keeping `mosaic-emit-paint` as a thin adapter.
- **Text wrapping:** multi-line text layout using `CHAR_WIDTH_PX` to estimate line
  breaks within the available width.
- **Image loading:** accept an optional `image_resolver: impl Fn(&str) -> Option<Vec<u8>>`
  callback so that real images can be rasterized into the preview.
- **Theme customization:** accept a `PaintTheme` struct to override the default
  color scheme for dark mode and high-contrast previews.
