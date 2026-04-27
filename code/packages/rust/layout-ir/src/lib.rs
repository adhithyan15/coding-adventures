//! # layout-ir
//!
//! UI02 — the universal intermediate representation for layout in the
//! coding-adventures stack. Rust implementation of the spec at
//! `code/specs/UI02-layout-ir.md`.
//!
//! Pure types + minimal builders + the [`TextMeasurer`] contract. Zero
//! runtime dependencies, zero I/O. Every downstream layout algorithm
//! and every layout-to-paint converter depends on this crate.
//!
//! ```text
//! Producer (DocumentAST, Mosaic IR, LaTeX IR)
//!     ↓  front-end (document-ast-to-layout, etc.)
//!   LayoutNode tree                     ← this crate
//!     ↓  layout algorithm (block / flexbox / grid)
//!   PositionedNode tree                 ← this crate
//!     ↓  layout-to-paint (UI04)
//!   PaintScene (P2D00)
//! ```
//!
//! ## Design principles inherited from UI02
//!
//! - **Producer-ignorant.** A LayoutNode tree from Markdown looks
//!   structurally identical to one from Mosaic. The layout algorithm
//!   only sees `LayoutNode`.
//! - **Algorithm-ignorant.** The core node carries properties every
//!   algorithm needs. Algorithm-specific fields live in the `ext` bag.
//! - **Extension over restriction.** New layout algorithms grow the
//!   `ext` namespace; they do not modify the core type.
//! - **No smartness.** The IR is a dumb data structure. It does not
//!   validate that the right algorithm was chosen, does not auto-
//!   detect, does not warn on unused fields.

use std::collections::HashMap;

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// Size values
// ═══════════════════════════════════════════════════════════════════════════

/// A size value for width or height.
///
/// - `Fixed(v)` — exact logical units
/// - `Fill`     — fill available space (CSS flex: 1)
/// - `Wrap`     — shrink to fit content (CSS fit-content)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeValue {
    Fixed(f64),
    Fill,
    Wrap,
}

pub fn size_fixed(v: f64) -> SizeValue {
    SizeValue::Fixed(v)
}
pub fn size_fill() -> SizeValue {
    SizeValue::Fill
}
pub fn size_wrap() -> SizeValue {
    SizeValue::Wrap
}

// ═══════════════════════════════════════════════════════════════════════════
// Edges (padding / margin)
// ═══════════════════════════════════════════════════════════════════════════

/// Four-sided spacing. Used for both padding (inside) and margin (outside).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Edges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

pub fn edges_all(v: f64) -> Edges {
    Edges { top: v, right: v, bottom: v, left: v }
}
pub fn edges_xy(x: f64, y: f64) -> Edges {
    Edges { top: y, right: x, bottom: y, left: x }
}
pub fn edges_zero() -> Edges {
    Edges::default()
}

// ═══════════════════════════════════════════════════════════════════════════
// Color
// ═══════════════════════════════════════════════════════════════════════════

/// RGBA with u8 components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 255 }
}
pub fn color_transparent() -> Color {
    Color { r: 0, g: 0, b: 0, a: 0 }
}
pub fn color_black() -> Color {
    Color { r: 0, g: 0, b: 0, a: 255 }
}
pub fn color_white() -> Color {
    Color { r: 255, g: 255, b: 255, a: 255 }
}

// ═══════════════════════════════════════════════════════════════════════════
// Font specification
// ═══════════════════════════════════════════════════════════════════════════

/// Fully-specified font descriptor. No CSS shorthand, no cascade, no
/// inheritance. Every `TextContent` carries a complete `FontSpec`.
#[derive(Clone, Debug, PartialEq)]
pub struct FontSpec {
    /// Family name (e.g. "Helvetica"). Empty = system default UI font.
    pub family: String,
    /// Size in **logical units**. Renderer converts to physical pixels
    /// via device pixel ratio / scale factor.
    pub size: f64,
    /// CSS-style weight 100..=900.
    pub weight: u16,
    pub italic: bool,
    /// Line-height multiplier, e.g. 1.5 = 150% of `size`. Must be > 0.
    pub line_height: f64,
}

pub fn font_spec(family: impl Into<String>, size: f64) -> FontSpec {
    FontSpec {
        family: family.into(),
        size,
        weight: 400,
        italic: false,
        line_height: 1.2,
    }
}

pub fn font_bold(mut spec: FontSpec) -> FontSpec {
    spec.weight = 700;
    spec
}

pub fn font_italic(mut spec: FontSpec) -> FontSpec {
    spec.italic = true;
    spec
}

// ═══════════════════════════════════════════════════════════════════════════
// Text alignment & image fit
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFit {
    Contain,
    Cover,
    Fill,
    None,
}

// ═══════════════════════════════════════════════════════════════════════════
// Content payloads
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
pub struct TextContent {
    pub value: String,
    pub font: FontSpec,
    pub color: Color,
    /// None = unlimited; wraps at the containing width until this many
    /// lines are reached, then truncates.
    pub max_lines: Option<u32>,
    pub text_align: TextAlign,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageContent {
    pub src: String,
    pub fit: ImageFit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
}

// ═══════════════════════════════════════════════════════════════════════════
// Extension bag
// ═══════════════════════════════════════════════════════════════════════════

/// Typed extension-bag value. Keeps the `ext` map zero-dependency while
/// preserving enough type information for downstream layout algorithms
/// to interpret their namespaced values.
///
/// Algorithms read only their own namespace. Unknown keys are ignored.
#[derive(Clone, Debug, PartialEq)]
pub enum ExtValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<ExtValue>),
    Map(HashMap<String, ExtValue>),
}

/// The layout node's extension bag.
pub type Ext = HashMap<String, ExtValue>;

// ═══════════════════════════════════════════════════════════════════════════
// LayoutNode — the core layout node
// ═══════════════════════════════════════════════════════════════════════════

/// The central type of the layout system. Input to any layout algorithm.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayoutNode {
    pub id: Option<String>,
    /// Leaf nodes carry content; container nodes carry children.
    /// A node should have one or the other, not both.
    pub content: Option<Content>,
    pub children: Vec<LayoutNode>,
    pub width: Option<SizeValue>,
    pub height: Option<SizeValue>,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,
    pub padding: Option<Edges>,
    pub margin: Option<Edges>,
    /// Extension bag. Keyed by algorithm namespace ("block", "flex", …).
    pub ext: Ext,
}

impl LayoutNode {
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn leaf_text(content: TextContent) -> Self {
        Self { content: Some(Content::Text(content)), ..Default::default() }
    }
    pub fn leaf_image(content: ImageContent) -> Self {
        Self { content: Some(Content::Image(content)), ..Default::default() }
    }
    pub fn container(children: Vec<LayoutNode>) -> Self {
        Self { children, ..Default::default() }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn with_padding(mut self, p: Edges) -> Self {
        self.padding = Some(p);
        self
    }
    pub fn with_margin(mut self, m: Edges) -> Self {
        self.margin = Some(m);
        self
    }
    pub fn with_width(mut self, w: SizeValue) -> Self {
        self.width = Some(w);
        self
    }
    pub fn with_height(mut self, h: SizeValue) -> Self {
        self.height = Some(h);
        self
    }
    pub fn with_ext(mut self, key: impl Into<String>, value: ExtValue) -> Self {
        self.ext.insert(key.into(), value);
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Constraints
// ═══════════════════════════════════════════════════════════════════════════

/// Available space passed into a layout call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    pub min_width: f64,
    pub max_width: f64,
    pub min_height: f64,
    pub max_height: f64,
}

pub fn constraints_fixed(w: f64, h: f64) -> Constraints {
    Constraints { min_width: w, max_width: w, min_height: h, max_height: h }
}

pub fn constraints_width(w: f64) -> Constraints {
    Constraints {
        min_width: 0.0,
        max_width: w,
        min_height: 0.0,
        max_height: f64::MAX,
    }
}

pub fn constraints_unconstrained() -> Constraints {
    Constraints {
        min_width: 0.0,
        max_width: f64::MAX,
        min_height: 0.0,
        max_height: f64::MAX,
    }
}

pub fn constraints_shrink(c: Constraints, dw: f64, dh: f64) -> Constraints {
    Constraints {
        min_width: (c.min_width - dw).max(0.0),
        max_width: (c.max_width - dw).max(0.0),
        min_height: (c.min_height - dh).max(0.0),
        max_height: (c.max_height - dh).max(0.0),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PositionedNode — layout output
// ═══════════════════════════════════════════════════════════════════════════

/// Output of a layout pass. Every node now has concrete position and
/// size relative to its parent's content-area origin.
#[derive(Clone, Debug, PartialEq)]
pub struct PositionedNode {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub id: Option<String>,
    pub content: Option<Content>,
    pub children: Vec<PositionedNode>,
    pub ext: Ext,
}

// ═══════════════════════════════════════════════════════════════════════════
// TextMeasurer — contract shared by all layout algorithms
// ═══════════════════════════════════════════════════════════════════════════

/// Measurement result in logical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasureResult {
    pub width: f64,
    pub height: f64,
    pub line_count: u32,
}

/// The text measurement contract. Layout algorithms take a measurer as
/// a parameter and call it whenever they need to size a text span.
/// Implementations live in separate packages so algorithms never import
/// a specific backend (CoreText, font-parser, Canvas, etc.).
pub trait TextMeasurer {
    /// Measure `text` rendered in `font`. If `max_width` is Some, text
    /// wraps at that width (line count > 1 possible); if None, it is
    /// measured as a single unbounded line.
    fn measure(
        &self,
        text: &str,
        font: &FontSpec,
        max_width: Option<f64>,
    ) -> MeasureResult;
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_value_constructors() {
        assert_eq!(size_fixed(12.0), SizeValue::Fixed(12.0));
        assert_eq!(size_fill(), SizeValue::Fill);
        assert_eq!(size_wrap(), SizeValue::Wrap);
    }

    #[test]
    fn edges_builders() {
        assert_eq!(
            edges_all(4.0),
            Edges { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 }
        );
        assert_eq!(
            edges_xy(8.0, 2.0),
            Edges { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 }
        );
        assert_eq!(edges_zero(), Edges::default());
    }

    #[test]
    fn color_builders() {
        assert_eq!(rgba(1, 2, 3, 4), Color { r: 1, g: 2, b: 3, a: 4 });
        assert_eq!(rgb(10, 20, 30), Color { r: 10, g: 20, b: 30, a: 255 });
        assert_eq!(color_transparent(), Color { r: 0, g: 0, b: 0, a: 0 });
        assert_eq!(color_black(), Color { r: 0, g: 0, b: 0, a: 255 });
        assert_eq!(color_white(), Color { r: 255, g: 255, b: 255, a: 255 });
    }

    #[test]
    fn font_spec_defaults() {
        let s = font_spec("Helvetica", 16.0);
        assert_eq!(s.family, "Helvetica");
        assert_eq!(s.size, 16.0);
        assert_eq!(s.weight, 400);
        assert!(!s.italic);
        assert!(s.line_height > 0.0);
    }

    #[test]
    fn font_bold_italic_compose() {
        let s = font_italic(font_bold(font_spec("Helvetica", 16.0)));
        assert_eq!(s.weight, 700);
        assert!(s.italic);
    }

    #[test]
    fn constraints_helpers() {
        let f = constraints_fixed(300.0, 200.0);
        assert_eq!(f.min_width, 300.0);
        assert_eq!(f.max_width, 300.0);
        assert_eq!(f.min_height, 200.0);
        assert_eq!(f.max_height, 200.0);

        let w = constraints_width(500.0);
        assert_eq!(w.max_width, 500.0);
        assert_eq!(w.max_height, f64::MAX);

        let u = constraints_unconstrained();
        assert_eq!(u.max_width, f64::MAX);

        let shrunk = constraints_shrink(constraints_fixed(100.0, 50.0), 30.0, 10.0);
        assert_eq!(shrunk.min_width, 70.0);
        assert_eq!(shrunk.max_width, 70.0);
        assert_eq!(shrunk.max_height, 40.0);

        let clamped = constraints_shrink(constraints_fixed(5.0, 5.0), 10.0, 10.0);
        assert_eq!(clamped.max_width, 0.0);
        assert_eq!(clamped.max_height, 0.0);
    }

    #[test]
    fn layout_node_builders() {
        let tc = TextContent {
            value: "hello".into(),
            font: font_spec("Helvetica", 16.0),
            color: color_black(),
            max_lines: None,
            text_align: TextAlign::Start,
        };
        let leaf = LayoutNode::leaf_text(tc.clone()).with_id("greeting");
        assert_eq!(leaf.id.as_deref(), Some("greeting"));
        assert_eq!(leaf.content, Some(Content::Text(tc)));
        assert!(leaf.children.is_empty());

        let c = LayoutNode::container(vec![LayoutNode::empty(), LayoutNode::empty()]);
        assert_eq!(c.children.len(), 2);

        let padded = LayoutNode::empty().with_padding(edges_all(8.0));
        assert_eq!(padded.padding, Some(edges_all(8.0)));

        let sized = LayoutNode::empty().with_width(size_fixed(200.0));
        assert_eq!(sized.width, Some(SizeValue::Fixed(200.0)));

        let with_ext = LayoutNode::empty().with_ext("block", ExtValue::Bool(true));
        assert_eq!(with_ext.ext.get("block"), Some(&ExtValue::Bool(true)));
    }

    // ------------------------------------------------------------------
    // TextMeasurer trait — exercise with a trivial fixed-width measurer.
    // ------------------------------------------------------------------

    struct FixedWidthMeasurer {
        char_width: f64,
    }

    impl TextMeasurer for FixedWidthMeasurer {
        fn measure(
            &self,
            text: &str,
            font: &FontSpec,
            max_width: Option<f64>,
        ) -> MeasureResult {
            let chars = text.chars().count() as f64;
            let line_width = chars * self.char_width * font.size;
            match max_width {
                None => MeasureResult {
                    width: line_width,
                    height: font.size * font.line_height,
                    line_count: 1,
                },
                Some(mw) if line_width <= mw => MeasureResult {
                    width: line_width,
                    height: font.size * font.line_height,
                    line_count: 1,
                },
                Some(mw) => {
                    let lines = (line_width / mw).ceil().max(1.0) as u32;
                    MeasureResult {
                        width: mw,
                        height: lines as f64 * font.size * font.line_height,
                        line_count: lines,
                    }
                }
            }
        }
    }

    #[test]
    fn text_measurer_single_line() {
        let m = FixedWidthMeasurer { char_width: 0.5 };
        let r = m.measure("hello", &font_spec("H", 10.0), None);
        assert_eq!(r.width, 25.0);
        assert_eq!(r.line_count, 1);
    }

    #[test]
    fn text_measurer_wraps() {
        let m = FixedWidthMeasurer { char_width: 0.5 };
        // 10 chars × 0.5 × 10.0 = 50.0 total width. maxWidth=20 ⇒ 3 lines.
        let r = m.measure("aaaaaaaaaa", &font_spec("H", 10.0), Some(20.0));
        assert_eq!(r.line_count, 3);
    }
}
