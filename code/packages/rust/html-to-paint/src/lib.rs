//! Compose the browser HTML render tree, shared layout, and shared paint
//! instruction stages without coupling any of them to a platform host.

use coding_adventures_html_parser::BrowserRenderTree;
use html_to_layout::{html_render_tree_to_layout, HtmlTheme};
use layout_block::layout_block;
use layout_ir::{Constraints, PositionedNode, TextMeasurer};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::PaintScene;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.1.0";

/// Viewport and device scale used by the composed HTML paint pipeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HtmlPaintViewport {
    pub width: f64,
    pub height: f64,
    pub device_pixel_ratio: f64,
}

impl HtmlPaintViewport {
    pub fn new(width: f64, height: f64, device_pixel_ratio: f64) -> Self {
        Self {
            width,
            height,
            device_pixel_ratio,
        }
    }
}

/// Geometry and paint output retained for rendering and later hit-testing.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlPaintOutput {
    pub positioned: PositionedNode,
    pub scene: PaintScene,
}

/// Convert a browser render tree into positioned geometry and a paint scene.
///
/// The caller supplies both layout measurement and the matching TXT00 paint
/// trio. This keeps platform font selection outside the composition package
/// while making the browser pipeline executable through the paint boundary.
pub fn html_render_tree_to_paint<M, S, FM, R>(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
    viewport: HtmlPaintViewport,
    measurer: &M,
    shaper: &S,
    metrics: &FM,
    resolver: &R,
) -> HtmlPaintOutput
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    let width = finite_non_negative(viewport.width);
    let viewport_height = finite_non_negative(viewport.height);
    let layout = html_render_tree_to_layout(render_tree, theme);
    let positioned = layout_block(
        &layout,
        Constraints {
            min_width: 0.0,
            max_width: width,
            min_height: 0.0,
            max_height: f64::MAX,
        },
        measurer,
    );
    let scene_height = positioned.height.max(viewport_height);
    let options = LayoutToPaintOptions {
        width,
        height: scene_height,
        background: theme.page_background,
        device_pixel_ratio: viewport.device_pixel_ratio,
        shaper,
        metrics,
        resolver,
    };
    let scene = layout_to_paint(&positioned, &options);

    HtmlPaintOutput { positioned, scene }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;
    use html_to_layout::mosaic_html_theme;
    use layout_ir::{Color, Content, ExtValue, FontSpec, MeasureResult};
    use paint_instructions::{ImageSrc, PaintInstruction};
    use text_interfaces::{
        Direction, FontQuery, FontResolutionError, Glyph, ShapeOptions, ShapedRun, ShapedText,
        ShapingError,
    };

    struct MonoMeasurer;

    impl TextMeasurer for MonoMeasurer {
        fn measure(&self, text: &str, font: &FontSpec, max_width: Option<f64>) -> MeasureResult {
            let char_width = font.size * 0.5;
            let full_width = text.chars().count() as f64 * char_width;
            let width_limit = max_width.unwrap_or(full_width).max(char_width);
            let line_count = (full_width / width_limit).ceil().max(1.0);
            MeasureResult {
                width: full_width.min(width_limit),
                height: line_count * font.size * font.line_height,
                line_count: line_count as u32,
            }
        }
    }

    #[derive(Clone)]
    struct FakeHandle;

    struct FakeResolver;

    impl FontResolver for FakeResolver {
        type Handle = FakeHandle;

        fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
            if query.family_names.is_empty() {
                Err(FontResolutionError::EmptyQuery)
            } else {
                Ok(FakeHandle)
            }
        }
    }

    struct FakeMetrics;

    impl FontMetrics for FakeMetrics {
        type Handle = FakeHandle;

        fn units_per_em(&self, _: &Self::Handle) -> u32 {
            1000
        }

        fn ascent(&self, _: &Self::Handle) -> i32 {
            800
        }

        fn descent(&self, _: &Self::Handle) -> i32 {
            200
        }

        fn line_gap(&self, _: &Self::Handle) -> i32 {
            0
        }

        fn x_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(500)
        }

        fn cap_height(&self, _: &Self::Handle) -> Option<i32> {
            Some(700)
        }

        fn family_name(&self, _: &Self::Handle) -> String {
            "Fake".into()
        }
    }

    struct FakeShaper;

    impl TextShaper for FakeShaper {
        type Handle = FakeHandle;

        fn shape(
            &self,
            text: &str,
            _: &Self::Handle,
            size: f32,
            options: &ShapeOptions,
        ) -> Result<ShapedText, ShapingError> {
            if options.direction != Direction::Ltr {
                return Err(ShapingError::UnsupportedDirection(options.direction));
            }
            let advance = size / 2.0;
            let glyphs: Vec<_> = text
                .char_indices()
                .map(|(cluster, character)| Glyph {
                    glyph_id: character as u32,
                    cluster: cluster as u32,
                    x_advance: advance,
                    y_advance: 0.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                })
                .collect();
            Ok(ShapedText::single(ShapedRun {
                x_advance_total: glyphs.len() as f32 * advance,
                glyphs,
                font_ref: "fake:mosaic".into(),
            }))
        }

        fn font_ref(&self, _: &Self::Handle) -> String {
            "fake:mosaic".into()
        }
    }

    #[test]
    fn normalizes_invalid_viewport_dimensions() {
        let render = parse_browser_render_tree("<p>Hello</p>").unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(f64::NAN, f64::NEG_INFINITY, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.scene.width, 0.0);
        assert!(output.scene.height >= 0.0);
    }

    #[test]
    fn canned_html_reaches_a_drawable_paint_scene() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'><h1>Mosaic lives</h1>\
             <p>The browser pipeline is <a href='../status'>connected</a>.</p>\
             <img src='logo.gif' width='32' height='24'>",
        )
        .unwrap();
        let theme = mosaic_html_theme();
        let output = html_render_tree_to_paint(
            &render,
            &theme,
            HtmlPaintViewport::new(240.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        assert_eq!(output.scene.width, 240.0);
        assert!(output.scene.height >= output.positioned.height);
        assert!(output.scene.height > 40.0);
        assert_eq!(output.scene.background, "rgb(192, 192, 192)");

        let glyph_runs: Vec<_> = output
            .scene
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                PaintInstruction::GlyphRun(run) => Some(run),
                _ => None,
            })
            .collect();
        assert!(!glyph_runs.is_empty());
        assert!(glyph_runs
            .iter()
            .any(|run| run.fill == Some(color_css(theme.link_color))));
        assert!(output.scene.instructions.iter().any(|instruction| matches!(
            instruction,
            PaintInstruction::Image(image)
                if image.src == ImageSrc::Uri("https://example.test/assets/logo.gif".into())
                    && image.width == 32.0
                    && image.height == 24.0
        )));
        let link = find_positioned_by_html_role(&output.positioned, "link").unwrap();
        assert_eq!(
            positioned_html_string(link, "href"),
            Some("https://example.test/status")
        );
        assert!(positioned_texts(&output.positioned).contains(&"connected"));
    }

    fn color_css(color: Color) -> String {
        format!("rgb({}, {}, {})", color.r, color.g, color.b)
    }

    fn positioned_texts(node: &PositionedNode) -> Vec<&str> {
        let mut texts = Vec::new();
        if let Some(Content::Text(text)) = &node.content {
            texts.push(text.value.as_str());
        }
        for child in &node.children {
            texts.extend(positioned_texts(child));
        }
        texts
    }

    fn find_positioned_by_html_role<'a>(
        node: &'a PositionedNode,
        role: &str,
    ) -> Option<&'a PositionedNode> {
        if positioned_html_string(node, "role") == Some(role) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_by_html_role(child, role))
    }

    fn positioned_html_string<'a>(node: &'a PositionedNode, key: &str) -> Option<&'a str> {
        let ExtValue::Map(values) = node.ext.get("html")? else {
            return None;
        };
        let ExtValue::Str(value) = values.get(key)? else {
            return None;
        };
        Some(value)
    }
}
