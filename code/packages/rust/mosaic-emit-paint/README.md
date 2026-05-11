# mosaic-emit-paint

Paint VM backend for the Mosaic compiler: **Mosaic source → PaintScene → PNG**.

Instead of producing HTML/JSX/QML text, this crate walks the `MosaicFile` tree
from `mosaic-analyzer` directly and maps each node type to `PaintInstruction`
variants (`PaintRect`, `PaintText`). The resulting `PaintScene` can be
rasterized to PNG bytes in a single call using the `barcode-2d` rendering
pipeline (Metal / Direct2D / Cairo / Skia, platform-dependent).

## Why a Paint backend?

| Backend            | Requires         | Output     |
|--------------------|------------------|------------|
| mosaic-emit-html   | A browser        | HTML text  |
| mosaic-emit-react  | React + build    | TSX text   |
| **mosaic-emit-paint** | **Nothing**  | **PNG bytes** |

`mosaic-emit-paint` is zero-dependency for the caller — no DOM, no Qt, no JS
runtime. It is the right backend for:

- Server-side thumbnail generation of Mosaic UI designs.
- Integration tests that compare pixels (not markup strings).
- Design tooling that needs a fast raster preview.

## Node → Paint mapping

| Mosaic node  | Paint output                                          |
|--------------|-------------------------------------------------------|
| `Box`        | `PaintRect` fill `#f8f8f8` + children overlaid        |
| `Stack`      | `PaintRect` fill `#f8f8f8` + children overlaid        |
| `Column`     | Background rect + children stacked vertically         |
| `Row`        | Background rect + children stacked horizontally       |
| `Text`       | `PaintText` at baseline (y + 16px)                    |
| `Image`      | `PaintRect` fill `#cccccc` stroke `#999999`           |
| `Spacer`     | *(nothing — absorbs remaining space)*                 |
| `Divider`    | `PaintRect` fill `#e0e0e0`, height 1 px               |
| `Icon`       | `PaintRect` fill `#e8e8e8` stroke `#999999`, 24×24   |
| `Grid`       | Header rect + 2 sample row rects                      |
| `Scroll`     | Stroked rect + children overlaid                      |
| `@slot_ref`  | `PaintText` `"@{slot}"` in gray                       |
| `when @s {}` | Dashed `PaintRect` + label `PaintText`               |
| `each @s {}` | 3× label `PaintText` "(1)" / "(2)" / "(3)"           |
| unknown tag  | Pale yellow dashed `PaintRect` placeholder            |

## Layout model

The layout engine is a **naive box model** — not CSS flexbox (that lives in
`layout-flexbox`):

- **Column**: available height ÷ number-of-non-Spacer-children per child.
- **Row**: available width ÷ number-of-non-Spacer-children per child.
- **Box / Stack / Scroll**: all children overlaid at the same origin.
- **Spacer**: absorbs its allocated slice of space, emits nothing.

Slot references and `when`/`each` blocks are rendered as labeled placeholders
so static previews remain meaningful without runtime data.

## Public API

```rust
/// Compile a Mosaic source string into a PaintScene.
pub fn render_scene(source: &str, width: f64, height: f64) -> Result<PaintScene, String>

/// Compile a Mosaic source string and render it to PNG bytes.
pub fn render_png(source: &str, width: f64, height: f64) -> Result<Vec<u8>, String>

/// 400×300 convenience wrapper for render_scene.
pub fn render_scene_with_defaults(source: &str) -> Result<PaintScene, String>

/// 400×300 convenience wrapper for render_png.
pub fn render_png_with_defaults(source: &str) -> Result<Vec<u8>, String>

/// Crate version: "0.1.0"
pub const VERSION: &str = "0.1.0";
```

## Usage example

```rust
use mosaic_emit_paint::{render_png_with_defaults, render_scene};

// Produce a PaintScene for a 800×600 canvas:
let scene = render_scene(
    r#"component ProfileCard {
        slot name: text;
        Column {
            Image { }
            Text { content: @name; }
        }
    }"#,
    800.0,
    600.0,
).unwrap();
println!("{} instructions", scene.instructions.len());

// Or go straight to PNG bytes (400×300 default):
let png_bytes = render_png_with_defaults(
    "component Banner { Box { Text { content: \"Hello\"; } } }",
).unwrap();
std::fs::write("banner.png", png_bytes).unwrap();
```

## How it fits in the stack

```text
mosaic-lexer  →  mosaic-parser  →  mosaic-analyzer
                                          │
                                  mosaic-emit-paint   ← this crate
                                          │
                                    PaintScene
                                          │
                               barcode-2d::render_scene_png
                                          │
                                      Vec<u8>  (PNG)
```

The crate does **not** use `MosaicRenderer` / `MosaicVM` — it directly walks
the `MosaicFile` tree for full layout control.
