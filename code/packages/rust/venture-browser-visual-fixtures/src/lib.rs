//! Reusable visual fixtures for Venture's browser pipeline and host shells.

use html_to_layout::mosaic_html_theme;
use html_to_paint::{HtmlPaintViewport, LinkRegion};
use image_codec_gif::encode_gif;
use layout_ir::{FontSpec, MeasureResult, PositionedNode, TextMeasurer};
use paint_codec_png::encode_png;
use paint_instructions::{PaintInstruction, PaintScene, PixelContainer};
use std::path::Path;
use text_interfaces::{
    Direction, FontMetrics, FontQuery, FontResolutionError, FontResolver, Glyph, ShapeOptions,
    ShapedRun, ShapedText, ShapingError, TextShaper,
};
use venture_browser_core::{
    BrowserFetchResponse, BrowserPage, BrowserPagePipeline, BrowserScrollCommand, BrowserViewport,
};

pub const VERSION: &str = "0.1.0";
pub const FIXTURE_PATH: &str = "/visual.html";
pub const IMAGE_PATH: &str = "/checker.gif";
pub const MISSING_IMAGE_PATH: &str = "/missing.gif";
pub const VIEWPORT_WIDTH: f64 = 240.0;
pub const VIEWPORT_HEIGHT: f64 = 120.0;

pub const FIXTURE_HTML: &str = r#"<!doctype html>
<html>
<head><title>Venture visual fixture</title></head>
<body>
  <h1 id="masthead">Venture Visual Atlas</h1>
  <p id="mixed-inline">A compact page with <b>bold type</b>, <i>italic type</i>, and a <a href="next.html">long wrapped link that crosses more than one visual line</a>.</p>
  <p id="image-row">Decoded image <img id="decoded-image" src="checker.gif" alt="checker" width="32" height="24"> and fallback <img id="fallback-image" src="missing.gif" alt="missing fixture" width="54" height="24"> stay inline.</p>
  <pre id="preformatted">GET /visual.html HTTP/1.0
Host: venture.test

preformatted columns stay aligned</pre>
  <p id="scroll-anchor"><a href="chapter.html">A wrapped chapter link remains hittable after scrolling through the viewport</a>.</p>
  <p>Venture composes parsing, layout, paint, images, and native presentation from reusable components.</p>
  <p id="tail">End of the deterministic visual fixture.</p>
</body>
</html>"#;

#[derive(Clone, Debug, PartialEq)]
pub struct RectFixture {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RectFixture {
    fn from_node(node: &PositionedNode, x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            width: node.width,
            height: node.height,
        }
    }

    fn concise(&self) -> String {
        format!(
            "({:.2}, {:.2}) {:.2}x{:.2}",
            self.x, self.y, self.width, self.height
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedGeometry {
    pub id: &'static str,
    pub rect: RectFixture,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometrySnapshot {
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub content_height: f64,
    pub max_scroll_y: f64,
    pub elements: Vec<NamedGeometry>,
    pub links: Vec<LinkRegion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaProbe {
    pub width: u32,
    pub height: u32,
    pub hash: u64,
    pub background_pixels: usize,
    pub ink_pixels: usize,
    pub blue_pixels: usize,
    pub magenta_pixels: usize,
    pub cyan_pixels: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenshotSnapshot {
    pub full: PixelContainer,
    pub full_probe: RgbaProbe,
    pub structural: PixelContainer,
    pub structural_probe: RgbaProbe,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisualFixtureCapture {
    pub geometry: GeometrySnapshot,
    pub initial: ScreenshotSnapshot,
    pub image: ScreenshotSnapshot,
    pub scrolled: ScreenshotSnapshot,
    pub image_scroll_offset_y: f64,
    pub scroll_offset_y: f64,
}

impl VisualFixtureCapture {
    pub fn diagnostics(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !approximately(self.geometry.viewport_width, VIEWPORT_WIDTH) {
            problems.push(format!(
                "viewport width: expected {VIEWPORT_WIDTH}, got {:.2}",
                self.geometry.viewport_width
            ));
        }
        if !approximately(self.geometry.viewport_height, VIEWPORT_HEIGHT) {
            problems.push(format!(
                "viewport height: expected {VIEWPORT_HEIGHT}, got {:.2}",
                self.geometry.viewport_height
            ));
        }
        if self.geometry.content_height <= VIEWPORT_HEIGHT * 2.0 {
            problems.push(format!(
                "content height did not exercise scrolling: {:.2}",
                self.geometry.content_height
            ));
        }
        if !approximately(self.geometry.content_height, BASELINE_CONTENT_HEIGHT) {
            problems.push(format!(
                "content height: expected {BASELINE_CONTENT_HEIGHT:.2}, got {:.2}",
                self.geometry.content_height
            ));
        }
        let expected_image_scroll = self
            .element("image-row")
            .map(|element| (element.rect.y - 16.0).clamp(0.0, self.geometry.max_scroll_y));
        if expected_image_scroll
            .is_some_and(|expected| !approximately(self.image_scroll_offset_y, expected))
        {
            problems.push(format!(
                "image offset: expected {:.2}, got {:.2}",
                expected_image_scroll.unwrap_or_default(),
                self.image_scroll_offset_y
            ));
        }
        let expected_scroll = self
            .element("scroll-anchor")
            .map(|element| (element.rect.y - 16.0).clamp(0.0, self.geometry.max_scroll_y));
        if expected_scroll.is_some_and(|expected| !approximately(self.scroll_offset_y, expected)) {
            problems.push(format!(
                "scrolled offset: expected {:.2}, got {:.2}",
                expected_scroll.unwrap_or_default(),
                self.scroll_offset_y
            ));
        }
        for id in REQUIRED_ELEMENT_IDS {
            if !self.elements().any(|element| element.id == id) {
                problems.push(format!("missing positioned fixture element #{id}"));
            }
        }
        for (id, expected) in BASELINE_ELEMENTS {
            match self.element(id) {
                Some(element) if !same_rect(&element.rect, &expected) => problems.push(format!(
                    "#{id} geometry: expected {}, got {}",
                    expected.concise(),
                    element.rect.concise()
                )),
                None => {}
                Some(_) => {}
            }
        }
        if self.geometry.links.len() != BASELINE_LINKS.len() {
            problems.push(format!(
                "link-region count: expected {}, got {}",
                BASELINE_LINKS.len(),
                self.geometry.links.len()
            ));
        }
        for (index, (suffix, expected)) in BASELINE_LINKS.iter().enumerate() {
            let Some(link) = self.geometry.links.get(index) else {
                break;
            };
            let actual = RectFixture {
                x: link.x,
                y: link.y,
                width: link.width,
                height: link.height,
            };
            if !link.url.ends_with(suffix) || !same_rect(&actual, expected) {
                problems.push(format!(
                    "link region {index}: expected *{suffix} at {}, got {} at {}",
                    expected.concise(),
                    link.url,
                    actual.concise()
                ));
            }
        }
        let wrapped_links = self
            .geometry
            .links
            .iter()
            .filter(|link| link.url.ends_with("/next.html"))
            .count();
        if wrapped_links < 2 {
            problems.push(format!(
                "wrapped link should expose at least two tight regions, got {wrapped_links}"
            ));
        }
        for (name, screenshot) in [("initial", &self.initial), ("scrolled", &self.scrolled)] {
            if (screenshot.full_probe.width, screenshot.full_probe.height) != (240, 120) {
                problems.push(format!(
                    "{name} full screenshot dimensions were {}x{}",
                    screenshot.full_probe.width, screenshot.full_probe.height
                ));
            }
            if screenshot.full_probe.ink_pixels == 0 || screenshot.full_probe.blue_pixels == 0 {
                problems.push(format!(
                    "{name} screenshot lost text/link paint: {:?}",
                    screenshot.full_probe
                ));
            }
        }
        if self.image.full_probe.magenta_pixels == 0 || self.image.full_probe.cyan_pixels == 0 {
            problems.push(format!(
                "image screenshot lost decoded image colors: {:?}",
                self.image.full_probe
            ));
        }
        if self.image.structural_probe.magenta_pixels == 0
            || self.image.structural_probe.cyan_pixels == 0
        {
            problems.push(format!(
                "structural image screenshot lost decoded image colors: {:?}",
                self.image.structural_probe
            ));
        }
        if self.initial.structural_probe.blue_pixels == 0
            || self.scrolled.structural_probe.blue_pixels == 0
        {
            problems.push(
                "structural screenshots lost wrapped-link decoration in a viewport state".into(),
            );
        }
        if self.initial.structural_probe.hash == self.scrolled.structural_probe.hash {
            problems.push("initial and scrolled structural screenshots are identical".into());
        }
        problems
    }

    pub fn assert_valid(&self) {
        let problems = self.diagnostics();
        assert!(
            problems.is_empty(),
            "Venture visual fixture mismatch:\n- {}\n\n{}",
            problems.join("\n- "),
            self.describe()
        );
    }

    pub fn describe(&self) -> String {
        let elements = self
            .elements()
            .map(|element| format!("#{}={}", element.id, element.rect.concise()))
            .collect::<Vec<_>>()
            .join(", ");
        let links = self
            .geometry
            .links
            .iter()
            .map(|link| {
                format!(
                    "{}=({:.2}, {:.2}) {:.2}x{:.2}",
                    link.url, link.x, link.y, link.width, link.height
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "viewport={:.0}x{:.0}, content_height={:.2}, max_scroll={:.2}, links=[{}], elements=[{}], initial={:?}, image={:?}, scrolled={:?}",
            self.geometry.viewport_width,
            self.geometry.viewport_height,
            self.geometry.content_height,
            self.geometry.max_scroll_y,
            links,
            elements,
            self.initial.structural_probe,
            self.image.structural_probe,
            self.scrolled.structural_probe,
        )
    }

    pub fn write_pngs(&self, output: impl AsRef<Path>) -> Result<(), String> {
        let output = output.as_ref();
        std::fs::create_dir_all(output).map_err(|error| error.to_string())?;
        for (name, pixels) in [
            ("initial-full.png", &self.initial.full),
            ("initial-structural.png", &self.initial.structural),
            ("image-full.png", &self.image.full),
            ("image-structural.png", &self.image.structural),
            ("scrolled-full.png", &self.scrolled.full),
            ("scrolled-structural.png", &self.scrolled.structural),
        ] {
            std::fs::write(output.join(name), encode_png(pixels))
                .map_err(|error| format!("write {name}: {error}"))?;
        }
        Ok(())
    }

    fn elements(&self) -> impl Iterator<Item = &NamedGeometry> {
        self.geometry.elements.iter()
    }

    fn element(&self, id: &str) -> Option<&NamedGeometry> {
        self.elements().find(|element| element.id == id)
    }
}

pub fn fixture_response(origin: &str, requested_url: &str) -> Result<BrowserFetchResponse, String> {
    let origin = origin.trim_end_matches('/');
    let page_url = format!("{origin}{FIXTURE_PATH}");
    let image_url = format!("{origin}{IMAGE_PATH}");
    match requested_url {
        url if url == page_url => Ok(BrowserFetchResponse::new(
            url,
            200,
            Some("text/html; charset=utf-8".into()),
            FIXTURE_HTML.as_bytes().to_vec(),
        )),
        url if url == image_url => Ok(BrowserFetchResponse::new(
            url,
            200,
            Some("image/gif".into()),
            checker_gif(),
        )),
        url if url == format!("{origin}{MISSING_IMAGE_PATH}") => {
            Err("intentional visual fixture image failure".into())
        }
        _ => Err(format!(
            "unknown Venture visual fixture URL {requested_url}"
        )),
    }
}

pub fn capture(origin: &str) -> Result<VisualFixtureCapture, String> {
    let theme = mosaic_html_theme();
    let measurer = DeterministicText;
    let shaper = DeterministicText;
    let metrics = DeterministicText;
    let resolver = DeterministicText;
    let pipeline = BrowserPagePipeline::new(
        &theme,
        HtmlPaintViewport::new(VIEWPORT_WIDTH, VIEWPORT_HEIGHT, 1.0),
        &measurer,
        &shaper,
        &metrics,
        &resolver,
    );
    let page_url = format!("{}{FIXTURE_PATH}", origin.trim_end_matches('/'));
    let page = pipeline
        .load(&page_url, &|url: &str| fixture_response(origin, url))
        .map_err(|error| error.to_string())?;
    capture_page(page)
}

pub fn capture_page(page: BrowserPage) -> Result<VisualFixtureCapture, String> {
    let geometry = geometry_snapshot(&page);
    let mut viewport = BrowserViewport::new(page, VIEWPORT_HEIGHT);
    let initial = screenshot(&viewport.viewport_scene())?;
    let image_scroll_offset_y = geometry
        .elements
        .iter()
        .find(|element| element.id == "image-row")
        .map(|element| viewport.set_scroll_offset_y(element.rect.y - 16.0))
        .unwrap_or_default();
    let image = screenshot(&viewport.viewport_scene())?;
    let scroll_offset_y = geometry
        .elements
        .iter()
        .find(|element| element.id == "scroll-anchor")
        .map(|element| viewport.set_scroll_offset_y(element.rect.y - 16.0))
        .unwrap_or_else(|| viewport.scroll_command(BrowserScrollCommand::DocumentEnd));
    let scrolled = screenshot(&viewport.viewport_scene())?;
    Ok(VisualFixtureCapture {
        geometry,
        initial,
        image,
        scrolled,
        image_scroll_offset_y,
        scroll_offset_y,
    })
}

fn geometry_snapshot(page: &BrowserPage) -> GeometrySnapshot {
    let mut elements = Vec::new();
    collect_named_geometry(&page.paint.positioned, 0.0, 0.0, &mut elements);
    let content_height = page.paint.scene.height;
    GeometrySnapshot {
        viewport_width: VIEWPORT_WIDTH,
        viewport_height: VIEWPORT_HEIGHT,
        content_height,
        max_scroll_y: (content_height - VIEWPORT_HEIGHT).max(0.0),
        elements,
        links: page.paint.links.clone(),
    }
}

fn collect_named_geometry(
    node: &PositionedNode,
    parent_x: f64,
    parent_y: f64,
    output: &mut Vec<NamedGeometry>,
) {
    let x = parent_x + node.x;
    let y = parent_y + node.y;
    if let Some(id) = node.id.as_deref().and_then(required_id) {
        output.push(NamedGeometry {
            id,
            rect: RectFixture::from_node(node, x, y),
        });
    }
    for child in &node.children {
        collect_named_geometry(child, x, y, output);
    }
}

fn required_id(id: &str) -> Option<&'static str> {
    REQUIRED_ELEMENT_IDS
        .iter()
        .copied()
        .find(|required| *required == id)
}

fn screenshot(scene: &PaintScene) -> Result<ScreenshotSnapshot, String> {
    let full = paint_vm_cairo::render(scene).map_err(|error| format!("{error:?}"))?;
    let mut structural_scene = scene.clone();
    mask_platform_text(&mut structural_scene.instructions);
    let structural =
        paint_vm_cairo::render(&structural_scene).map_err(|error| format!("{error:?}"))?;
    Ok(ScreenshotSnapshot {
        full_probe: probe(&full),
        structural_probe: probe(&structural),
        full,
        structural,
    })
}

fn mask_platform_text(instructions: &mut Vec<PaintInstruction>) {
    instructions.retain_mut(|instruction| match instruction {
        PaintInstruction::Text(_) | PaintInstruction::GlyphRun(_) => false,
        PaintInstruction::Group(group) => {
            mask_platform_text(&mut group.children);
            true
        }
        PaintInstruction::Layer(layer) => {
            mask_platform_text(&mut layer.children);
            true
        }
        PaintInstruction::Clip(clip) => {
            mask_platform_text(&mut clip.children);
            true
        }
        _ => true,
    });
}

fn probe(pixels: &PixelContainer) -> RgbaProbe {
    probe_rgba(pixels.width, pixels.height, &pixels.data)
        .expect("PixelContainer dimensions must match its RGBA storage")
}

/// Summarize a host-rendered RGBA frame with portable visual evidence.
///
/// Native hosts intentionally use their platform text rasterizers, so their
/// acceptance tests compare dimensions and representative color populations
/// instead of carrying backend-specific golden files.
pub fn probe_rgba(width: u32, height: u32, data: &[u8]) -> Result<RgbaProbe, String> {
    let expected_len = width as usize * height as usize * 4;
    if data.len() != expected_len {
        return Err(format!(
            "RGBA frame is {} bytes; expected {expected_len} for {width}x{height}",
            data.len()
        ));
    }
    let mut background_pixels = 0;
    let mut ink_pixels = 0;
    let mut blue_pixels = 0;
    let mut magenta_pixels = 0;
    let mut cyan_pixels = 0;
    for pixel in data.as_chunks::<4>().0 {
        let [r, g, b, a] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        background_pixels += usize::from(a == 255 && r == 192 && g == 192 && b == 192);
        ink_pixels += usize::from(a > 0 && r < 96 && g < 96 && b < 96);
        blue_pixels += usize::from(a > 0 && b > 120 && b > r.saturating_add(40));
        magenta_pixels += usize::from(a == 255 && r > 220 && g < 40 && b > 220);
        cyan_pixels += usize::from(a == 255 && r < 40 && g > 220 && b > 220);
    }
    Ok(RgbaProbe {
        width,
        height,
        hash: fnv1a64(data),
        background_pixels,
        ink_pixels,
        blue_pixels,
        magenta_pixels,
        cyan_pixels,
    })
}

fn checker_gif() -> Vec<u8> {
    let mut pixels = PixelContainer::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            let offset = ((y * 4 + x) * 4) as usize;
            let magenta = (x + y) % 2 == 0;
            pixels.data[offset..offset + 4].copy_from_slice(if magenta {
                &[255, 0, 255, 255]
            } else {
                &[0, 255, 255, 255]
            });
        }
    }
    encode_gif(&pixels)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn approximately(left: f64, right: f64) -> bool {
    (left - right).abs() <= 0.01
}

fn same_rect(left: &RectFixture, right: &RectFixture) -> bool {
    approximately(left.x, right.x)
        && approximately(left.y, right.y)
        && approximately(left.width, right.width)
        && approximately(left.height, right.height)
}

const BASELINE_CONTENT_HEIGHT: f64 = 539.2;
const BASELINE_ELEMENTS: [(&str, RectFixture); 8] = [
    ("masthead", rect(16.0, 22.0, 208.0, 57.6)),
    ("mixed-inline", rect(16.0, 85.6, 208.0, 78.4)),
    ("image-row", rect(16.0, 170.0, 208.0, 64.8)),
    ("decoded-image", rect(107.0, 170.0, 32.0, 24.0)),
    ("fallback-image", rect(16.0, 202.4, 54.0, 24.0)),
    ("preformatted", rect(16.0, 240.8, 208.0, 62.4)),
    ("scroll-anchor", rect(16.0, 309.2, 208.0, 78.4)),
    ("tail", rect(16.0, 478.0, 208.0, 39.2)),
];
const BASELINE_LINKS: [(&str, RectFixture); 7] = [
    ("/next.html", rect(142.0, 105.2, 28.0, 19.6)),
    ("/next.html", rect(16.0, 124.8, 175.0, 19.6)),
    ("/next.html", rect(16.0, 144.4, 175.0, 19.6)),
    ("/chapter.html", rect(16.0, 309.2, 154.0, 19.6)),
    ("/chapter.html", rect(16.0, 328.8, 154.0, 19.6)),
    ("/chapter.html", rect(16.0, 348.4, 147.0, 19.6)),
    ("/chapter.html", rect(16.0, 368.0, 56.0, 19.6)),
];

const fn rect(x: f64, y: f64, width: f64, height: f64) -> RectFixture {
    RectFixture {
        x,
        y,
        width,
        height,
    }
}

const REQUIRED_ELEMENT_IDS: [&str; 8] = [
    "masthead",
    "mixed-inline",
    "image-row",
    "decoded-image",
    "fallback-image",
    "preformatted",
    "scroll-anchor",
    "tail",
];

#[derive(Clone, Debug)]
pub struct DeterministicFont;

#[derive(Clone, Copy, Debug, Default)]
pub struct DeterministicText;

impl TextMeasurer for DeterministicText {
    fn measure(&self, text: &str, font: &FontSpec, max_width: Option<f64>) -> MeasureResult {
        let width = text.chars().count() as f64 * font.size * 0.5;
        let constrained = max_width.unwrap_or(width).max(font.size * 0.5);
        let lines = (width / constrained).ceil().max(1.0);
        MeasureResult {
            width: width.min(constrained),
            height: lines * font.size * font.line_height,
            baseline: font.size * 0.8,
            line_count: lines as u32,
        }
    }
}

impl FontResolver for DeterministicText {
    type Handle = DeterministicFont;

    fn resolve(&self, query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
        if query.family_names.is_empty() {
            Err(FontResolutionError::EmptyQuery)
        } else {
            Ok(DeterministicFont)
        }
    }
}

impl FontMetrics for DeterministicText {
    type Handle = DeterministicFont;

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
        "Venture Deterministic".into()
    }
}

impl TextShaper for DeterministicText {
    type Handle = DeterministicFont;

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
        let advance = size * 0.5;
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
            font_ref: "sans".into(),
        }))
    }

    fn font_ref(&self, _: &Self::Handle) -> String {
        "sans".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representative_page_captures_initial_and_scrolled_visuals() {
        let capture = capture("http://venture.test").expect("capture fixture");
        capture.assert_valid();
        eprintln!("{}", capture.describe());
    }

    #[test]
    fn fixture_router_serves_html_image_and_intentional_failure() {
        let origin = "http://venture.test";
        let page = fixture_response(origin, "http://venture.test/visual.html").unwrap();
        assert_eq!(page.status, 200);
        assert_eq!(page.body, FIXTURE_HTML.as_bytes());
        let image = fixture_response(origin, "http://venture.test/checker.gif").unwrap();
        assert_eq!(image.media_type.as_deref(), Some("image/gif"));
        assert!(fixture_response(origin, "http://venture.test/missing.gif").is_err());
    }

    #[test]
    fn rgba_probe_rejects_malformed_host_frames() {
        assert!(probe_rgba(2, 2, &[0; 15]).is_err());
        assert_eq!(
            probe_rgba(1, 1, &[255, 0, 255, 255])
                .unwrap()
                .magenta_pixels,
            1
        );
    }
}
