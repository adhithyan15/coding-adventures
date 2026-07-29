//! Compose the browser HTML render tree, shared layout, and shared paint
//! instruction stages without coupling any of them to a platform host.

use coding_adventures_html_parser::BrowserRenderTree;
use html_to_layout::{html_render_tree_to_layout, HtmlTheme};
use image_codec_gif::decode_gif;
use image_codec_jpeg::decode_jpeg;
use layout_block::layout_block;
use layout_ir::{Constraints, Content, ExtValue, PositionedNode, TextMeasurer};
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use paint_instructions::{
    ImageSrc, PaintBase, PaintClip, PaintImage, PaintInstruction, PaintRect, PaintScene, PaintText,
    TextAlign,
};
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.4.0";

const HTML_IMAGE_ALT_METADATA: &str = "html.alt";

/// Bytes fetched by a browser host for one resolved image URI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchedImage {
    pub bytes: Vec<u8>,
    pub media_type: Option<String>,
}

impl FetchedImage {
    pub fn new(bytes: Vec<u8>, media_type: Option<String>) -> Self {
        Self { bytes, media_type }
    }
}

/// Host boundary for synchronously fetching an image resource.
///
/// HTTP, file, cache, and security policy stay outside this package. The
/// returned bytes are decoded here into the shared `PixelContainer` contract.
pub trait HtmlImageFetcher {
    fn fetch(&self, uri: &str) -> Result<FetchedImage, String>;
}

impl<F> HtmlImageFetcher for F
where
    F: Fn(&str) -> Result<FetchedImage, String>,
{
    fn fetch(&self, uri: &str) -> Result<FetchedImage, String> {
        self(uri)
    }
}

/// Failure while fetching or decoding a URI-backed paint image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HtmlImageResourceError {
    Fetch {
        uri: String,
        message: String,
    },
    UnsupportedFormat {
        uri: String,
        media_type: Option<String>,
    },
    Decode {
        uri: String,
        format: &'static str,
        message: String,
    },
}

impl std::fmt::Display for HtmlImageResourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch { uri, message } => {
                write!(formatter, "failed to fetch image {uri}: {message}")
            }
            Self::UnsupportedFormat { uri, media_type } => match media_type {
                Some(media_type) => {
                    write!(
                        formatter,
                        "unsupported image format for {uri} ({media_type})"
                    )
                }
                None => write!(formatter, "unsupported image format for {uri}"),
            },
            Self::Decode {
                uri,
                format,
                message,
            } => write!(
                formatter,
                "failed to decode {format} image {uri}: {message}"
            ),
        }
    }
}

impl std::error::Error for HtmlImageResourceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportedImageFormat {
    Gif,
    Jpeg,
}

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

/// A scene resolved as far as possible plus recoverable image failures.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlImageResolution {
    pub scene: PaintScene,
    pub failures: Vec<HtmlImageResourceError>,
}

/// Fetch and decode every URI-backed image in a paint scene.
///
/// The input scene is never mutated. Resolution happens in a clone and the
/// fully decoded scene is returned only after every image succeeds, so a
/// failed resource cannot leave the caller with a partially resolved scene.
/// Existing `ImageSrc::Pixels` values pass through unchanged.
pub fn resolve_scene_image_resources<F>(
    scene: &PaintScene,
    fetcher: &F,
) -> Result<PaintScene, HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    let mut resolved = scene.clone();
    resolve_instruction_images(&mut resolved.instructions, fetcher)?;
    Ok(resolved)
}

/// Resolve URI-backed images and replace failures with Mosaic-style fallbacks.
///
/// Unlike [`resolve_scene_image_resources`], this tolerant browser path keeps
/// processing after fetch, format, or decode failures. Each failed image is
/// replaced by a clipped one-pixel black border and its HTML `alt` text, while
/// successful images still become decoded pixels. All failures are returned
/// for status-bar or diagnostic reporting by the host.
pub fn resolve_scene_image_resources_with_mosaic_fallback<F>(
    scene: &PaintScene,
    fetcher: &F,
) -> HtmlImageResolution
where
    F: HtmlImageFetcher,
{
    let mut resolved = scene.clone();
    let mut failures = Vec::new();
    resolve_instruction_images_with_mosaic_fallback(
        &mut resolved.instructions,
        fetcher,
        &mut failures,
    );
    HtmlImageResolution {
        scene: resolved,
        failures,
    }
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
    let mut scene = layout_to_paint(&positioned, &options);
    annotate_html_image_metadata(&positioned, &mut scene);
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

fn resolve_instruction_images<F>(
    instructions: &mut [PaintInstruction],
    fetcher: &F,
) -> Result<(), HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    for instruction in instructions {
        match instruction {
            PaintInstruction::Image(image) => {
                if !matches!(image.src, ImageSrc::Uri(_)) {
                    continue;
                }
                image.src = ImageSrc::Pixels(fetch_and_decode_image(image, fetcher)?);
            }
            PaintInstruction::Group(group) => {
                resolve_instruction_images(&mut group.children, fetcher)?;
            }
            PaintInstruction::Layer(layer) => {
                resolve_instruction_images(&mut layer.children, fetcher)?;
            }
            PaintInstruction::Clip(clip) => {
                resolve_instruction_images(&mut clip.children, fetcher)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_instruction_images_with_mosaic_fallback<F>(
    instructions: &mut [PaintInstruction],
    fetcher: &F,
    failures: &mut Vec<HtmlImageResourceError>,
) where
    F: HtmlImageFetcher,
{
    for instruction in instructions {
        let replacement = match instruction {
            PaintInstruction::Image(image) if matches!(image.src, ImageSrc::Uri(_)) => {
                match fetch_and_decode_image(image, fetcher) {
                    Ok(pixels) => {
                        image.src = ImageSrc::Pixels(pixels);
                        None
                    }
                    Err(error) => {
                        let fallback = mosaic_broken_image_fallback(image);
                        failures.push(error);
                        Some(fallback)
                    }
                }
            }
            PaintInstruction::Group(group) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut group.children,
                    fetcher,
                    failures,
                );
                None
            }
            PaintInstruction::Layer(layer) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut layer.children,
                    fetcher,
                    failures,
                );
                None
            }
            PaintInstruction::Clip(clip) => {
                resolve_instruction_images_with_mosaic_fallback(
                    &mut clip.children,
                    fetcher,
                    failures,
                );
                None
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            *instruction = replacement;
        }
    }
}

fn fetch_and_decode_image<F>(
    image: &PaintImage,
    fetcher: &F,
) -> Result<paint_instructions::PixelContainer, HtmlImageResourceError>
where
    F: HtmlImageFetcher,
{
    let ImageSrc::Uri(uri) = &image.src else {
        unreachable!("pixel-backed images are filtered before resource resolution");
    };
    let fetched = fetcher
        .fetch(uri)
        .map_err(|message| HtmlImageResourceError::Fetch {
            uri: uri.clone(),
            message,
        })?;
    match detect_image_format(uri, &fetched) {
        Some(SupportedImageFormat::Gif) => {
            decode_gif(&fetched.bytes).map_err(|message| HtmlImageResourceError::Decode {
                uri: uri.clone(),
                format: "GIF",
                message,
            })
        }
        Some(SupportedImageFormat::Jpeg) => {
            decode_jpeg(&fetched.bytes).map_err(|message| HtmlImageResourceError::Decode {
                uri: uri.clone(),
                format: "JPEG",
                message,
            })
        }
        None => Err(HtmlImageResourceError::UnsupportedFormat {
            uri: uri.clone(),
            media_type: fetched.media_type,
        }),
    }
}

fn mosaic_broken_image_fallback(image: &PaintImage) -> PaintInstruction {
    let width = image.width.max(0.0);
    let height = image.height.max(0.0);
    let mut children = vec![PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(),
        x: image.x + 0.5,
        y: image.y + 0.5,
        width: (width - 1.0).max(0.0),
        height: (height - 1.0).max(0.0),
        fill: None,
        stroke: Some("#000000".into()),
        stroke_width: Some(1.0),
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })];

    let alt = image
        .base
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get(HTML_IMAGE_ALT_METADATA));
    if let Some(alt) = alt.filter(|alt| !alt.is_empty()) {
        let font_size = 12.0_f64.min((height - 4.0).max(1.0));
        children.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: image.x + 3.0,
            y: image.y + 2.0 + font_size,
            text: alt.clone(),
            font_ref: Some("serif".into()),
            font_size,
            fill: Some("#000000".into()),
            text_align: Some(TextAlign::Left),
        }));
    }

    PaintInstruction::Clip(PaintClip {
        base: image.base.clone(),
        x: image.x,
        y: image.y,
        width,
        height,
        children,
    })
}

fn annotate_html_image_metadata(positioned: &PositionedNode, scene: &mut PaintScene) {
    let mut image_alts = Vec::new();
    let mut stack = vec![positioned];
    while let Some(node) = stack.pop() {
        if matches!(node.content, Some(Content::Image(_))) {
            image_alts.push(positioned_html_string(node, "alt").map(ToOwned::to_owned));
        }
        for child in node.children.iter().rev() {
            stack.push(child);
        }
    }

    let mut image_alts = image_alts.into_iter();
    for instruction in &mut scene.instructions {
        let PaintInstruction::Image(image) = instruction else {
            continue;
        };
        let Some(alt) = image_alts.next().flatten() else {
            continue;
        };
        image
            .base
            .metadata
            .get_or_insert_with(Default::default)
            .insert(HTML_IMAGE_ALT_METADATA.into(), alt);
    }
}

fn detect_image_format(uri: &str, fetched: &FetchedImage) -> Option<SupportedImageFormat> {
    if fetched.bytes.starts_with(b"GIF87a") || fetched.bytes.starts_with(b"GIF89a") {
        return Some(SupportedImageFormat::Gif);
    }
    if fetched.bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(SupportedImageFormat::Jpeg);
    }

    if let Some(media_type) = fetched.media_type.as_deref() {
        let media_type = media_type.split(';').next().unwrap_or("").trim();
        if media_type.eq_ignore_ascii_case("image/gif") {
            return Some(SupportedImageFormat::Gif);
        }
        if media_type.eq_ignore_ascii_case("image/jpeg")
            || media_type.eq_ignore_ascii_case("image/jpg")
            || media_type.eq_ignore_ascii_case("image/pjpeg")
        {
            return Some(SupportedImageFormat::Jpeg);
        }
    }

    let path = uri
        .split(['?', '#'])
        .next()
        .unwrap_or(uri)
        .to_ascii_lowercase();
    if path.ends_with(".gif") {
        Some(SupportedImageFormat::Gif)
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Some(SupportedImageFormat::Jpeg)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;
    use html_to_layout::mosaic_html_theme;
    use layout_ir::{Color, FontSpec, MeasureResult};
    use paint_instructions::PixelContainer;
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

    #[test]
    fn canned_html_image_fetches_decodes_and_rasterizes_with_cairo() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'><p>Inline image:</p>\
             <img src='logo.gif' width='32' height='24'>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(160.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let mut source = PixelContainer::new(2, 2);
        source.fill(255, 0, 255, 255);
        let gif = image_codec_gif::encode_gif(&source);
        let resolved = resolve_scene_image_resources(&output.scene, &|uri: &str| {
            assert_eq!(uri, "https://example.test/assets/logo.gif");
            Ok(FetchedImage::new(gif.clone(), Some("image/gif".into())))
        })
        .expect("canned image should resolve");

        let image = first_image(&resolved.instructions).expect("resolved image instruction");
        assert!(matches!(
            &image.src,
            ImageSrc::Pixels(pixels)
                if pixels.width == 2
                    && pixels.height == 2
                    && pixels.pixel_at(0, 0) == (255, 0, 255, 255)
        ));

        let pixels =
            paint_vm_cairo::render(&resolved).expect("resolved canned HTML image should rasterize");
        let sample_x = (image.x + image.width / 2.0).floor() as u32;
        let sample_y = (image.y + image.height / 2.0).floor() as u32;
        let (red, green, blue, alpha) = pixels.pixel_at(sample_x, sample_y);
        assert!(red > 200 && green < 50 && blue > 200 && alpha == 255);
    }

    #[test]
    fn jpeg_resources_are_detected_from_bytes_and_decoded() {
        let mut source = PixelContainer::new(2, 2);
        source.fill(20, 100, 180, 255);
        let jpeg = image_codec_jpeg::encode_jpeg(&source);
        let scene = scene_with_uri_image("https://example.test/photo.bin");

        let resolved = resolve_scene_image_resources(&scene, &|_: &str| {
            Ok(FetchedImage::new(
                jpeg.clone(),
                Some("application/octet-stream".into()),
            ))
        })
        .expect("JPEG signature should select the JPEG decoder");

        let image = first_image(&resolved.instructions).unwrap();
        assert!(matches!(
            &image.src,
            ImageSrc::Pixels(pixels) if pixels.width == 2 && pixels.height == 2
        ));
    }

    #[test]
    fn image_resolution_failure_leaves_the_input_scene_unchanged() {
        let scene = scene_with_uri_image("https://example.test/missing.gif");
        let original = scene.clone();

        let error = resolve_scene_image_resources(&scene, &|_: &str| Err("offline".to_string()))
            .unwrap_err();

        assert_eq!(
            error,
            HtmlImageResourceError::Fetch {
                uri: "https://example.test/missing.gif".into(),
                message: "offline".into(),
            }
        );
        assert_eq!(scene, original);
    }

    #[test]
    fn failed_html_image_rasterizes_mosaic_alt_text_fallback() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/assets/'>\
             <p>Image follows:</p><img src='missing.gif' alt='Mosaic logo' width='80' height='24'>",
        )
        .unwrap();
        let output = html_render_tree_to_paint(
            &render,
            &mosaic_html_theme(),
            HtmlPaintViewport::new(180.0, 96.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );

        let image = first_image(&output.scene.instructions).expect("HTML image instruction");
        assert_eq!(
            image
                .base
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get(HTML_IMAGE_ALT_METADATA))
                .map(String::as_str),
            Some("Mosaic logo")
        );
        let fallback_x = image.x.floor() as u32;
        let fallback_y = image.y.floor() as u32;

        let resolution =
            resolve_scene_image_resources_with_mosaic_fallback(&output.scene, &|_: &str| {
                Err("offline".into())
            });
        assert_eq!(
            resolution.failures,
            vec![HtmlImageResourceError::Fetch {
                uri: "https://example.test/assets/missing.gif".into(),
                message: "offline".into(),
            }]
        );
        assert!(!contains_uri_image(&resolution.scene.instructions));
        assert!(contains_text(&resolution.scene.instructions, "Mosaic logo"));

        let pixels = paint_vm_cairo::render(&resolution.scene)
            .expect("broken-image fallback should rasterize");
        let (red, green, blue, alpha) = pixels.pixel_at(fallback_x, fallback_y);
        assert!(red < 192 && green < 192 && blue < 192 && alpha == 255);
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

    fn scene_with_uri_image(uri: &str) -> PaintScene {
        let mut scene = PaintScene::new(8.0, 8.0);
        scene.instructions.push(PaintInstruction::Image(PaintImage {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
            src: ImageSrc::Uri(uri.into()),
            opacity: None,
        }));
        scene
    }

    fn first_image(instructions: &[PaintInstruction]) -> Option<&PaintImage> {
        for instruction in instructions {
            match instruction {
                PaintInstruction::Image(image) => return Some(image),
                PaintInstruction::Group(group) => {
                    if let Some(image) = first_image(&group.children) {
                        return Some(image);
                    }
                }
                PaintInstruction::Layer(layer) => {
                    if let Some(image) = first_image(&layer.children) {
                        return Some(image);
                    }
                }
                PaintInstruction::Clip(clip) => {
                    if let Some(image) = first_image(&clip.children) {
                        return Some(image);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn contains_uri_image(instructions: &[PaintInstruction]) -> bool {
        instructions.iter().any(|instruction| match instruction {
            PaintInstruction::Image(image) => matches!(image.src, ImageSrc::Uri(_)),
            PaintInstruction::Group(group) => contains_uri_image(&group.children),
            PaintInstruction::Layer(layer) => contains_uri_image(&layer.children),
            PaintInstruction::Clip(clip) => contains_uri_image(&clip.children),
            _ => false,
        })
    }

    fn contains_text(instructions: &[PaintInstruction], expected: &str) -> bool {
        instructions.iter().any(|instruction| match instruction {
            PaintInstruction::Text(text) => text.text == expected,
            PaintInstruction::Group(group) => contains_text(&group.children, expected),
            PaintInstruction::Layer(layer) => contains_text(&layer.children, expected),
            PaintInstruction::Clip(clip) => contains_text(&clip.children, expected),
            _ => false,
        })
    }
}
