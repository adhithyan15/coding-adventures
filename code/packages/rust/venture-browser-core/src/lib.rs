//! Host-neutral navigation and page-loading orchestration for Venture.
//!
//! The platform shell owns windows, events, font implementations, and the
//! final paint backend. This crate composes the shared network, HTML, layout,
//! paint, and image-resource seams into one synchronous page load.

use coding_adventures_html_parser::{parse_html, BrowserDocument, BrowserRenderTree};
use html_to_layout::HtmlTheme;
use html_to_paint::{
    html_render_tree_to_paint, resolve_scene_image_resources_with_mosaic_fallback, FetchedImage,
    HtmlImageResourceError, HtmlPaintOutput, HtmlPaintViewport,
};
use http1_client::HttpClient;
use layout_ir::TextMeasurer;
use std::fmt;
use text_interfaces::{FontMetrics, FontResolver, TextShaper};

pub const VERSION: &str = "0.1.0";

/// In-memory browser navigation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationHistory {
    home_url: String,
    back_stack: Vec<String>,
    current_url: Option<String>,
    forward_stack: Vec<String>,
}

impl NavigationHistory {
    pub fn new(home_url: impl Into<String>) -> Self {
        Self {
            home_url: home_url.into(),
            back_stack: Vec::new(),
            current_url: None,
            forward_stack: Vec::new(),
        }
    }

    pub fn with_current(home_url: impl Into<String>, current_url: impl Into<String>) -> Self {
        let mut history = Self::new(home_url);
        history.current_url = Some(current_url.into());
        history
    }

    pub fn home_url(&self) -> &str {
        &self.home_url
    }

    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    pub fn back_stack(&self) -> &[String] {
        &self.back_stack
    }

    pub fn forward_stack(&self) -> &[String] {
        &self.forward_stack
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// Record a new navigation and clear stale forward history.
    pub fn navigate(&mut self, url: impl Into<String>) -> &str {
        if let Some(current) = self.current_url.take() {
            self.back_stack.push(current);
        }
        self.current_url = Some(url.into());
        self.forward_stack.clear();
        self.current_url.as_deref().unwrap_or("")
    }

    pub fn back(&mut self) -> Option<&str> {
        let previous = self.back_stack.pop()?;
        if let Some(current) = self.current_url.replace(previous) {
            self.forward_stack.push(current);
        }
        self.current_url()
    }

    pub fn forward(&mut self) -> Option<&str> {
        let next = self.forward_stack.pop()?;
        if let Some(current) = self.current_url.replace(next) {
            self.back_stack.push(current);
        }
        self.current_url()
    }

    pub fn home(&mut self) -> &str {
        self.navigate(self.home_url.clone())
    }

    /// Return the URL that a host should fetch again without changing history.
    pub fn reload(&self) -> Option<&str> {
        self.current_url()
    }

    /// Replace the current entry after a redirect without creating history.
    pub fn replace_current(&mut self, final_url: impl Into<String>) -> Option<&str> {
        self.current_url.as_ref()?;
        self.current_url = Some(final_url.into());
        self.current_url()
    }
}

/// One resource returned by a browser-owned transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFetchResponse {
    pub final_url: String,
    pub status: u16,
    pub media_type: Option<String>,
    pub body: Vec<u8>,
}

impl BrowserFetchResponse {
    pub fn new(
        final_url: impl Into<String>,
        status: u16,
        media_type: Option<String>,
        body: Vec<u8>,
    ) -> Self {
        Self {
            final_url: final_url.into(),
            status,
            media_type,
            body,
        }
    }
}

/// Replaceable transport boundary for page and inline-image bytes.
pub trait BrowserResourceFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String>;
}

impl<F> BrowserResourceFetcher for F
where
    F: Fn(&str) -> Result<BrowserFetchResponse, String>,
{
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        self(url)
    }
}

/// Concrete HTTP/1.0 transport adapter for a Venture host.
#[derive(Clone, Debug, Default)]
pub struct HttpBrowserFetcher {
    pub client: HttpClient,
}

impl HttpBrowserFetcher {
    pub fn new(client: HttpClient) -> Self {
        Self { client }
    }
}

impl BrowserResourceFetcher for HttpBrowserFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        let response = self.client.get(url).map_err(|error| error.to_string())?;
        let media_type = response.head.header("Content-Type").map(ToOwned::to_owned);
        Ok(BrowserFetchResponse::new(
            response.final_url,
            response.head.status,
            media_type,
            response.body,
        ))
    }
}

/// A loaded HTML document ready for a platform paint backend.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserPage {
    pub requested_url: String,
    pub final_url: String,
    pub status: u16,
    pub source: String,
    pub document: BrowserDocument,
    pub render_tree: BrowserRenderTree,
    pub paint: HtmlPaintOutput,
    pub image_failures: Vec<HtmlImageResourceError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserLoadError {
    Fetch { url: String, message: String },
    HttpStatus { url: String, status: u16 },
    UnsupportedMediaType { url: String, media_type: String },
    Parse { url: String, message: String },
}

impl fmt::Display for BrowserLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch { url, message } => write!(formatter, "failed to fetch {url}: {message}"),
            Self::HttpStatus { url, status } => {
                write!(formatter, "HTTP status {status} while loading {url}")
            }
            Self::UnsupportedMediaType { url, media_type } => {
                write!(formatter, "unsupported media type {media_type} for {url}")
            }
            Self::Parse { url, message } => write!(formatter, "failed to parse {url}: {message}"),
        }
    }
}

impl std::error::Error for BrowserLoadError {}

/// Reusable layout and font services for loading pages into paint scenes.
pub struct BrowserPagePipeline<'a, M, S, FM, R> {
    pub theme: &'a HtmlTheme,
    pub viewport: HtmlPaintViewport,
    pub measurer: &'a M,
    pub shaper: &'a S,
    pub metrics: &'a FM,
    pub resolver: &'a R,
}

impl<'a, M, S, FM, R> BrowserPagePipeline<'a, M, S, FM, R>
where
    M: TextMeasurer,
    S: TextShaper,
    FM: FontMetrics<Handle = S::Handle>,
    R: FontResolver<Handle = S::Handle>,
{
    pub fn new(
        theme: &'a HtmlTheme,
        viewport: HtmlPaintViewport,
        measurer: &'a M,
        shaper: &'a S,
        metrics: &'a FM,
        resolver: &'a R,
    ) -> Self {
        Self {
            theme,
            viewport,
            measurer,
            shaper,
            metrics,
            resolver,
        }
    }

    /// Fetch and compose one HTML page through a resolved `PaintScene`.
    ///
    /// Missing `Content-Type` is accepted for compatibility with early web
    /// servers. Explicit non-HTML content is left to standalone-image or
    /// raw-text host policies.
    pub fn load<F>(&self, requested_url: &str, fetcher: &F) -> Result<BrowserPage, BrowserLoadError>
    where
        F: BrowserResourceFetcher,
    {
        let response = fetcher
            .fetch(requested_url)
            .map_err(|message| BrowserLoadError::Fetch {
                url: requested_url.to_string(),
                message,
            })?;
        ensure_success(&response)?;
        ensure_html_media_type(&response)?;

        let source = String::from_utf8_lossy(&response.body).into_owned();
        let parsed = parse_html(&source).map_err(|error| BrowserLoadError::Parse {
            url: response.final_url.clone(),
            message: error.to_string(),
        })?;
        let document = BrowserDocument::from_document(&parsed);
        let render_tree =
            BrowserRenderTree::from_document_with_document_url(&parsed, &response.final_url);
        let mut paint = html_render_tree_to_paint(
            &render_tree,
            self.theme,
            self.viewport,
            self.measurer,
            self.shaper,
            self.metrics,
            self.resolver,
        );
        let image_resolution =
            resolve_scene_image_resources_with_mosaic_fallback(&paint.scene, &|url: &str| {
                let resource = fetcher.fetch(url)?;
                if !is_success(resource.status) {
                    return Err(format!("HTTP status {}", resource.status));
                }
                Ok(FetchedImage::new(resource.body, resource.media_type))
            });
        paint.scene = image_resolution.scene;

        Ok(BrowserPage {
            requested_url: requested_url.to_string(),
            final_url: response.final_url,
            status: response.status,
            source,
            document,
            render_tree,
            paint,
            image_failures: image_resolution.failures,
        })
    }
}

fn ensure_success(response: &BrowserFetchResponse) -> Result<(), BrowserLoadError> {
    if is_success(response.status) {
        Ok(())
    } else {
        Err(BrowserLoadError::HttpStatus {
            url: response.final_url.clone(),
            status: response.status,
        })
    }
}

fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

fn ensure_html_media_type(response: &BrowserFetchResponse) -> Result<(), BrowserLoadError> {
    let Some(media_type) = response.media_type.as_deref() else {
        return Ok(());
    };
    let essence = media_type.split(';').next().unwrap_or("").trim();
    if essence.eq_ignore_ascii_case("text/html")
        || essence.eq_ignore_ascii_case("application/xhtml+xml")
    {
        Ok(())
    } else {
        Err(BrowserLoadError::UnsupportedMediaType {
            url: response.final_url.clone(),
            media_type: media_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use html_to_layout::mosaic_html_theme;
    use html_to_paint::HtmlPaintViewport;
    use image_codec_gif::encode_gif;
    use layout_ir::{FontSpec, MeasureResult};
    use paint_instructions::{ImageSrc, PaintInstruction, PixelContainer};
    use std::cell::RefCell;
    use text_interfaces::{
        Direction, FontQuery, FontResolutionError, Glyph, ShapeOptions, ShapedRun, ShapedText,
        ShapingError,
    };

    #[test]
    fn navigation_history_matches_back_forward_home_and_reload_model() {
        let mut history =
            NavigationHistory::with_current("http://home.test/", "http://example.test/a");
        history.navigate("http://example.test/b");
        history.navigate("http://example.test/c");

        assert_eq!(history.back(), Some("http://example.test/b"));
        assert_eq!(history.forward_stack(), &["http://example.test/c"]);
        assert_eq!(history.forward(), Some("http://example.test/c"));
        assert_eq!(history.reload(), Some("http://example.test/c"));

        history.back();
        history.navigate("http://example.test/d");
        assert!(history.forward_stack().is_empty());
        assert_eq!(history.home(), "http://home.test/");
        assert_eq!(
            history.replace_current("http://home.test/index.html"),
            Some("http://home.test/index.html")
        );
    }

    #[test]
    fn redirected_html_and_image_fetch_reach_cairo_pixels() {
        let mut source_pixels = PixelContainer::new(2, 2);
        source_pixels.fill(255, 0, 255, 255);
        let gif = encode_gif(&source_pixels);
        let requested = "http://example.test/start";
        let final_url = "http://example.test/guide/index.html";
        let fetched_urls = RefCell::new(Vec::new());
        let fetcher = |url: &str| {
            fetched_urls.borrow_mut().push(url.to_string());
            match url {
                "http://example.test/start" => Ok(BrowserFetchResponse::new(
                    final_url,
                    200,
                    Some("text/html; charset=utf-8".into()),
                    b"<title>Venture guide</title><h1>Venture</h1>\
                      <p><a href='next.html'>Next</a></p>\
                      <img src='logo.gif' alt='logo' width='20' height='20'>"
                        .to_vec(),
                )),
                "http://example.test/guide/logo.gif" => Ok(BrowserFetchResponse::new(
                    url,
                    200,
                    Some("image/gif".into()),
                    gif.clone(),
                )),
                _ => Err(format!("unexpected URL {url}")),
            }
        };

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(220.0, 100.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let page = pipeline
            .load(requested, &fetcher)
            .expect("canned navigation should load");

        assert_eq!(page.requested_url, requested);
        assert_eq!(page.final_url, final_url);
        assert_eq!(page.document.title.as_deref(), Some("Venture guide"));
        assert!(page.image_failures.is_empty());
        assert_eq!(page.paint.links.len(), 1);
        assert_eq!(
            page.paint.links[0].url,
            "http://example.test/guide/next.html"
        );
        assert!(page.paint.scene.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                PaintInstruction::Image(image)
                    if matches!(&image.src, ImageSrc::Pixels(pixels)
                        if pixels.width == 2 && pixels.height == 2)
            )
        }));
        assert_eq!(
            fetched_urls.into_inner(),
            vec![
                "http://example.test/start",
                "http://example.test/guide/logo.gif"
            ]
        );

        let pixels =
            paint_vm_cairo::render(&page.paint.scene).expect("page scene should rasterize");
        assert_eq!(pixels.width, 220);
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
    fn broken_inline_image_is_recoverable() {
        let fetcher = |url: &str| match url {
            "http://example.test/" => Ok(BrowserFetchResponse::new(
                url,
                200,
                None,
                b"<img src='missing.gif' alt='missing' width='40' height='20'>".to_vec(),
            )),
            _ => Err("offline".into()),
        };

        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let page = pipeline
            .load("http://example.test/", &fetcher)
            .expect("broken image should not fail page load");

        assert_eq!(page.image_failures.len(), 1);
        assert!(page.paint.scene.instructions.iter().all(|instruction| {
            !matches!(
                instruction,
                PaintInstruction::Image(image) if matches!(image.src, ImageSrc::Uri(_))
            )
        }));
    }

    #[test]
    fn rejects_http_errors_and_explicit_non_html_pages() {
        let theme = mosaic_html_theme();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(100.0, 40.0, 1.0),
            &MonoMeasurer,
            &FakeShaper,
            &FakeMetrics,
            &FakeResolver,
        );
        let not_found = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                404,
                Some("text/html".into()),
                Vec::new(),
            ))
        };
        assert_eq!(
            pipeline.load("http://example.test/missing", &not_found),
            Err(BrowserLoadError::HttpStatus {
                url: "http://example.test/missing".into(),
                status: 404,
            })
        );

        let image = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("image/gif".into()),
                Vec::new(),
            ))
        };
        assert_eq!(
            pipeline.load("http://example.test/logo.gif", &image),
            Err(BrowserLoadError::UnsupportedMediaType {
                url: "http://example.test/logo.gif".into(),
                media_type: "image/gif".into(),
            })
        );
    }

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
}
