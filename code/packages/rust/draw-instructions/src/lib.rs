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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawRectInstruction {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub fill: String,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawTextInstruction {
    pub x: i32,
    pub y: i32,
    pub value: String,
    pub fill: String,
    pub font_family: String,
    pub font_size: i32,
    pub align: String,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawGroupInstruction {
    pub children: Vec<DrawInstruction>,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawInstruction {
    Rect(DrawRectInstruction),
    Text(DrawTextInstruction),
    Group(DrawGroupInstruction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        metadata,
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
}
