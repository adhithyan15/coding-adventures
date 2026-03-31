//! # draw-instructions
//!
//! Backend-neutral 2D draw instructions for reusable scene generation.
//!
//! This crate sits between producer logic and renderer logic:
//! - producers decide what should be drawn
//! - renderers decide how to serialize or paint it
//!
//! A barcode package, for example, can emit rectangles and text without knowing
//! anything about SVG syntax.

pub const VERSION: &str = "0.1.0";

use std::collections::BTreeMap;

pub type Metadata = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq)]
pub struct DrawRectInstruction {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill: String,
    /// Optional stroke color for the rectangle outline.
    /// When `Some`, the SVG renderer emits a `stroke` attribute.
    pub stroke: Option<String>,
    /// Optional stroke width in pixels.
    /// Only meaningful when `stroke` is also set.
    pub stroke_width: Option<f64>,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawTextInstruction {
    pub x: i32,
    pub y: i32,
    pub value: String,
    pub fill: String,
    pub font_family: String,
    pub font_size: i32,
    pub align: String,
    /// Optional CSS font-weight value (e.g. "bold", "700").
    /// When `Some`, the SVG renderer emits a `font-weight` attribute.
    pub font_weight: Option<String>,
    pub metadata: Metadata,
}

/// A line segment from (x1, y1) to (x2, y2).
///
/// Lines are purely geometric — they have no fill, only a stroke color
/// and width.  The SVG renderer turns each `DrawLineInstruction` into
/// a `<line>` element.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawLineInstruction {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stroke: String,
    pub stroke_width: f64,
    pub metadata: Metadata,
}

/// A clipping region that hides anything its children draw outside
/// the rectangle (x, y, width, height).
///
/// The SVG renderer turns this into a `<clipPath>` definition plus a
/// `<g clip-path="url(#clip-N)">` wrapper around the children.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawClipInstruction {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub children: Vec<DrawInstruction>,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawGroupInstruction {
    pub children: Vec<DrawInstruction>,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawInstruction {
    Rect(DrawRectInstruction),
    Text(DrawTextInstruction),
    Group(DrawGroupInstruction),
    /// A line segment.
    Line(DrawLineInstruction),
    /// A rectangular clipping region wrapping child instructions.
    Clip(DrawClipInstruction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawScene {
    pub width: i32,
    pub height: i32,
    pub background: String,
    pub instructions: Vec<DrawInstruction>,
    pub metadata: Metadata,
}

pub trait Renderer<T> {
    fn render(&self, scene: &DrawScene) -> T;
}

pub fn draw_rect(x: i32, y: i32, width: i32, height: i32, fill: &str, metadata: Metadata) -> DrawInstruction {
    DrawInstruction::Rect(DrawRectInstruction {
        x,
        y,
        width,
        height,
        fill: if fill.is_empty() { "#000000".into() } else { fill.into() },
        stroke: None,
        stroke_width: None,
        metadata,
    })
}

pub fn draw_text(x: i32, y: i32, value: &str, metadata: Metadata) -> DrawInstruction {
    DrawInstruction::Text(DrawTextInstruction {
        x,
        y,
        value: value.into(),
        fill: "#000000".into(),
        font_family: "monospace".into(),
        font_size: 16,
        align: "middle".into(),
        font_weight: None,
        metadata,
    })
}

pub fn draw_line(x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str, stroke_width: f64) -> DrawInstruction {
    DrawInstruction::Line(DrawLineInstruction {
        x1,
        y1,
        x2,
        y2,
        stroke: stroke.into(),
        stroke_width,
        metadata: Metadata::new(),
    })
}

pub fn draw_clip(x: f64, y: f64, width: f64, height: f64, children: Vec<DrawInstruction>) -> DrawInstruction {
    DrawInstruction::Clip(DrawClipInstruction {
        x,
        y,
        width,
        height,
        children,
        metadata: Metadata::new(),
    })
}

pub fn draw_group(children: Vec<DrawInstruction>, metadata: Metadata) -> DrawInstruction {
    DrawInstruction::Group(DrawGroupInstruction { children, metadata })
}

pub fn create_scene(width: i32, height: i32, instructions: Vec<DrawInstruction>, background: &str, metadata: Metadata) -> DrawScene {
    DrawScene {
        width,
        height,
        background: if background.is_empty() { "#ffffff".into() } else { background.into() },
        instructions,
        metadata,
    }
}

pub fn render_with<T>(scene: &DrawScene, renderer: &impl Renderer<T>) -> T {
    renderer.render(scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRenderer;

    impl Renderer<String> for TestRenderer {
        fn render(&self, _scene: &DrawScene) -> String {
            "ok".into()
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn helpers_build_scene() {
        let rect = draw_rect(1, 2, 3, 4, "#111111", Metadata::new());
        let scene = create_scene(100, 50, vec![draw_group(vec![rect], Metadata::new())], "", Metadata::new());
        assert_eq!(scene.background, "#ffffff");
    }

    #[test]
    fn render_with_delegates() {
        let scene = create_scene(10, 10, vec![], "", Metadata::new());
        assert_eq!(render_with(&scene, &TestRenderer), "ok");
    }

    #[test]
    fn draw_line_creates_correct_struct() {
        let line = draw_line(0.0, 0.0, 100.0, 200.0, "#ff0000", 2.0);
        match line {
            DrawInstruction::Line(ref l) => {
                assert_eq!(l.x1, 0.0);
                assert_eq!(l.y1, 0.0);
                assert_eq!(l.x2, 100.0);
                assert_eq!(l.y2, 200.0);
                assert_eq!(l.stroke, "#ff0000");
                assert_eq!(l.stroke_width, 2.0);
            }
            _ => panic!("expected Line variant"),
        }
    }

    #[test]
    fn draw_clip_creates_correct_struct_with_children() {
        let child = draw_rect(5, 5, 10, 10, "#000", Metadata::new());
        let clip = draw_clip(0.0, 0.0, 50.0, 50.0, vec![child]);
        match clip {
            DrawInstruction::Clip(ref c) => {
                assert_eq!(c.x, 0.0);
                assert_eq!(c.y, 0.0);
                assert_eq!(c.width, 50.0);
                assert_eq!(c.height, 50.0);
                assert_eq!(c.children.len(), 1);
            }
            _ => panic!("expected Clip variant"),
        }
    }

    #[test]
    fn draw_rect_initializes_stroke_as_none() {
        let rect = draw_rect(0, 0, 10, 10, "#000", Metadata::new());
        match rect {
            DrawInstruction::Rect(ref r) => {
                assert!(r.stroke.is_none());
                assert!(r.stroke_width.is_none());
            }
            _ => panic!("expected Rect variant"),
        }
    }

    #[test]
    fn draw_text_initializes_font_weight_as_none() {
        let text = draw_text(0, 0, "hello", Metadata::new());
        match text {
            DrawInstruction::Text(ref t) => {
                assert!(t.font_weight.is_none());
            }
            _ => panic!("expected Text variant"),
        }
    }
}
