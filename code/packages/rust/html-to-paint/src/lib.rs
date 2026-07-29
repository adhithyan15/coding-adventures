//! Compose the browser HTML render tree, shared layout, and shared paint
//! instruction stages without coupling any of them to a platform host.

use coding_adventures_html_parser::BrowserRenderTree;
use html_to_layout::{html_render_tree_to_layout, HtmlTheme};
use layout_block::layout_block;
use layout_ir::{Constraints, ExtValue, PositionedNode, TextMeasurer};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::PaintScene;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.2.1";

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

/// A clickable link rectangle in logical document-content coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub url: String,
}

impl LinkRegion {
    /// Return whether a content-space point falls inside the region.
    ///
    /// The right and bottom edges are exclusive so adjacent regions do not
    /// both claim a point on their shared boundary.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && x >= self.x
            && x < self.x + self.width
            && y >= self.y
            && y < self.y + self.height
    }
}

/// Geometry, link hit regions, and paint output for a browser host.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlPaintOutput {
    pub positioned: PositionedNode,
    pub links: Vec<LinkRegion>,
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
    let links = extract_link_regions(&positioned);

    HtmlPaintOutput {
        positioned,
        links,
        scene,
    }
}

/// Extract resolved link rectangles while accumulating parent-relative layout
/// coordinates into absolute logical document coordinates.
pub fn extract_link_regions(root: &PositionedNode) -> Vec<LinkRegion> {
    let mut regions = Vec::new();
    let mut stack = vec![(root, 0.0, 0.0)];

    while let Some((node, parent_x, parent_y)) = stack.pop() {
        let absolute_x = parent_x + node.x;
        let absolute_y = parent_y + node.y;
        if positioned_html_string(node, "role") == Some("link") {
            if let Some(url) = positioned_html_string(node, "href") {
                if valid_link_box(absolute_x, absolute_y, node.width, node.height)
                    && !url.is_empty()
                {
                    regions.push(LinkRegion {
                        x: absolute_x,
                        y: absolute_y,
                        width: node.width,
                        height: node.height,
                        url: url.to_string(),
                    });
                }
            }
        }

        for child in node.children.iter().rev() {
            stack.push((child, absolute_x, absolute_y));
        }
    }

    regions
}

/// Hit-test a viewport-space point against logical document link regions.
///
/// `scroll_y` is added to the viewport y coordinate to recover document
/// content coordinates. Negative or non-finite scroll offsets are treated as
/// zero; scrolling policy and clamping remain the browser host's concern.
pub fn hit_test_link(
    regions: &[LinkRegion],
    viewport_x: f64,
    viewport_y: f64,
    scroll_y: f64,
) -> Option<&LinkRegion> {
    let scroll_y = finite_non_negative(scroll_y);
    let content_y = viewport_y + scroll_y;
    regions
        .iter()
        .find(|region| region.contains(viewport_x, content_y))
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn valid_link_box(x: f64, y: f64, width: f64, height: f64) -> bool {
    x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;
    use html_to_layout::mosaic_html_theme;
    use layout_ir::{Color, Content, FontSpec, MeasureResult};
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
        assert_eq!(output.links.len(), 1);
        assert_eq!(output.links[0].url, "https://example.test/status");
        assert_eq!(
            hit_test_link(
                &output.links,
                output.links[0].x + output.links[0].width / 2.0,
                output.links[0].y + output.links[0].height / 2.0,
                0.0,
            ),
            Some(&output.links[0])
        );
        assert!(positioned_texts(&output.positioned).contains(&"connected"));
    }

    #[test]
    fn extraction_accumulates_parent_coordinates_and_skips_empty_boxes() {
        let visible = positioned_link(5.0, 7.0, 20.0, 10.0, "https://example.test/visible");
        let empty = positioned_link(30.0, 7.0, 0.0, 10.0, "https://example.test/empty");
        let root = PositionedNode {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 100.0,
            id: None,
            content: None,
            children: vec![visible, empty],
            ext: Default::default(),
        };

        assert_eq!(
            extract_link_regions(&root),
            vec![LinkRegion {
                x: 15.0,
                y: 27.0,
                width: 20.0,
                height: 10.0,
                url: "https://example.test/visible".into(),
            }]
        );
    }

    #[test]
    fn hit_testing_applies_scroll_and_uses_half_open_edges() {
        let region = LinkRegion {
            x: 10.0,
            y: 80.0,
            width: 30.0,
            height: 12.0,
            url: "https://example.test/next".into(),
        };

        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 10.0, 20.0, 60.0),
            Some(&region)
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 40.0, 20.0, 60.0),
            None
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), 10.0, 32.0, 60.0),
            None
        );
        assert_eq!(
            hit_test_link(std::slice::from_ref(&region), f64::NAN, 20.0, 60.0),
            None
        );
    }

    #[test]
    fn canned_html_rasterizes_to_rgba_pixels_with_cairo() {
        let render = parse_browser_render_tree(
            "<h1>Mosaic</h1><p>Hello <a href='https://example.test/'>world</a>.</p>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(200.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let pixels = paint_vm_cairo::render(&output.scene)
            .expect("canned HTML paint scene should rasterize");
        assert_eq!(pixels.width, 200);
        assert_eq!(pixels.height, output.scene.height.ceil() as u32);
        assert_eq!(
            pixels.data.len(),
            pixels.width as usize * pixels.height as usize * 4
        );
        assert!(pixels
            .data
            .chunks_exact(4)
            .any(|pixel| pixel != [192, 192, 192, 255]));
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
        super::positioned_html_string(node, key)
    }

    fn positioned_link(x: f64, y: f64, width: f64, height: f64, url: &str) -> PositionedNode {
        PositionedNode {
            x,
            y,
            width,
            height,
            id: None,
            content: None,
            children: Vec::new(),
            ext: std::collections::HashMap::from([(
                "html".into(),
                ExtValue::Map(std::collections::HashMap::from([
                    ("role".into(), ExtValue::Str("link".into())),
                    ("href".into(), ExtValue::Str(url.into())),
                ])),
            )]),
        }
    }
}
