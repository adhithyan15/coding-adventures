//! Runnable macOS host for the Venture browser.
//!
//! The host deliberately stays thin: shared crates own navigation, fetching,
//! HTML parsing, layout, paint composition, scrolling, and hit-testing. This
//! crate selects native CoreText services, creates an AppKit Metal window, and
//! presents the loaded viewport.

use html_to_layout::mosaic_html_theme;
use html_to_paint::HtmlPaintViewport;
use layout_text_measure_native::NativeMeasurer;
use std::fmt;
#[cfg(target_vendor = "apple")]
use std::{cell::RefCell, rc::Rc};
use text_native::{NativeMetrics, NativeResolver, NativeShaper};
use venture_browser_core::{
    BrowserLoadError, BrowserNavigation, BrowserPagePipeline, BrowserResourceFetcher,
    BrowserSession,
};
use window_core::WindowError;
#[cfg(target_vendor = "apple")]
use window_core::WindowEvent;

#[cfg(target_vendor = "apple")]
use venture_browser_core::HttpBrowserFetcher;

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_START_URL: &str = "http://info.cern.ch/";
pub const DEFAULT_WINDOW_WIDTH: f64 = 1024.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 720.0;

#[derive(Debug)]
pub enum MacBrowserError {
    UnsupportedPlatform,
    Load(BrowserLoadError),
    Window(WindowError),
    MissingMetalLayer,
    Paint(String),
    MissingViewport,
}

impl fmt::Display for MacBrowserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(
                    formatter,
                    "the Venture macOS host requires an Apple platform"
                )
            }
            Self::Load(error) => write!(formatter, "page load failed: {error}"),
            Self::Window(error) => write!(formatter, "window setup failed: {error}"),
            Self::MissingMetalLayer => {
                write!(
                    formatter,
                    "AppKit did not attach the requested CAMetalLayer"
                )
            }
            Self::Paint(message) => write!(formatter, "Metal presentation failed: {message}"),
            Self::MissingViewport => write!(formatter, "page load completed without a viewport"),
        }
    }
}

impl std::error::Error for MacBrowserError {}

impl From<BrowserLoadError> for MacBrowserError {
    fn from(error: BrowserLoadError) -> Self {
        Self::Load(error)
    }
}

impl From<WindowError> for MacBrowserError {
    fn from(error: WindowError) -> Self {
        Self::Window(error)
    }
}

pub fn window_title(page_title: Option<&str>, final_url: &str) -> String {
    match page_title.map(str::trim).filter(|title| !title.is_empty()) {
        Some(page_title) => format!("{page_title} — Venture — {final_url}"),
        None => format!("Venture — {final_url}"),
    }
}

/// Load the first page with native text measurement and shaping.
///
/// Keeping the fetcher injectable makes the same acceptance path deterministic
/// in tests while the executable uses `HttpBrowserFetcher`.
pub fn load_initial_session<F>(
    start_url: &str,
    width: f64,
    height: f64,
    fetcher: &F,
) -> Result<BrowserSession, BrowserLoadError>
where
    F: BrowserResourceFetcher,
{
    let theme = mosaic_html_theme();
    let measurer = NativeMeasurer::new();
    let shaper = NativeShaper::new();
    let metrics = NativeMetrics::new();
    let resolver = NativeResolver::new();
    let pipeline = BrowserPagePipeline::new(
        &theme,
        HtmlPaintViewport::new(width, height, 1.0),
        &measurer,
        &shaper,
        &metrics,
        &resolver,
    );
    let mut session = BrowserSession::new(start_url, height);
    session.execute(
        BrowserNavigation::Navigate(start_url.to_string()),
        &pipeline,
        fetcher,
    )?;
    Ok(session)
}

#[cfg(target_vendor = "apple")]
pub fn run(start_url: &str) -> Result<(), MacBrowserError> {
    run_with_termination(start_url, None)
}

/// Launch the native browser host and terminate it after a short interval.
///
/// This exercises the real AppKit + `CAMetalLayer` presentation path in
/// automated smoke runs while normal `run` remains interactive until close.
#[cfg(target_vendor = "apple")]
pub fn run_for_smoke(start_url: &str, seconds: f64) -> Result<(), MacBrowserError> {
    run_with_termination(start_url, Some(seconds.max(0.0)))
}

#[cfg(target_vendor = "apple")]
fn run_with_termination(
    start_url: &str,
    terminate_after: Option<f64>,
) -> Result<(), MacBrowserError> {
    use window_appkit::AppKitBackend;
    use window_core::{LogicalSize, SurfacePreference, WindowBuilder};

    let session = load_initial_session(
        start_url,
        DEFAULT_WINDOW_WIDTH,
        DEFAULT_WINDOW_HEIGHT,
        &HttpBrowserFetcher::default(),
    )?;
    let viewport = session.viewport().ok_or(MacBrowserError::MissingViewport)?;
    let scene = viewport.viewport_scene();
    let page = viewport.page();
    let title = window_title(page.document.title.as_deref(), &page.final_url);

    let mut backend = AppKitBackend::new();
    let window = WindowBuilder::new()
        .title(title)
        .initial_size(LogicalSize::new(
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
        ))
        .preferred_surface(SurfacePreference::Metal)
        .build_with(&mut backend)?;
    let layer = window
        .appkit_target()
        .metal_layer
        .ok_or(MacBrowserError::MissingMetalLayer)?;
    paint_metal::render_to_metal_layer(&scene, layer as objc_bridge::Id)
        .map_err(|error| MacBrowserError::Paint(error.to_string()))?;

    let session = Rc::new(RefCell::new(session));
    window.set_event_handler({
        let session = Rc::clone(&session);
        move |event| {
            let mut session = session.borrow_mut();
            if scroll_session(&mut session, &event) {
                if let Some(viewport) = session.viewport() {
                    let scene = viewport.viewport_scene();
                    if let Err(error) =
                        paint_metal::render_to_metal_layer(&scene, layer as objc_bridge::Id)
                    {
                        eprintln!("venture-browser-macos: Metal repaint failed: {error}");
                    }
                }
            }
        }
    })?;

    if let Some(seconds) = terminate_after {
        backend.terminate_after(seconds)?;
    }
    backend.run()?;
    Ok(())
}

/// Apply a normalized scroll event to the current browser viewport.
///
/// Returns `true` only when the clamped offset changed and the host should
/// repaint.
pub fn scroll_session(session: &mut BrowserSession, event: &WindowEvent) -> bool {
    let WindowEvent::Scroll { delta_y, .. } = event else {
        return false;
    };
    let Some(viewport) = session.viewport_mut() else {
        return false;
    };
    let previous_offset = viewport.scroll_state().offset_y();
    viewport.scroll_by(*delta_y);
    viewport.scroll_state().offset_y() != previous_offset
}

#[cfg(not(target_vendor = "apple"))]
pub fn run(_start_url: &str) -> Result<(), MacBrowserError> {
    Err(MacBrowserError::UnsupportedPlatform)
}

#[cfg(not(target_vendor = "apple"))]
pub fn run_for_smoke(_start_url: &str, _seconds: f64) -> Result<(), MacBrowserError> {
    Err(MacBrowserError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_vendor = "apple")]
    use venture_browser_core::BrowserFetchResponse;
    #[cfg(target_vendor = "apple")]
    use window_core::WindowId;

    #[test]
    fn window_title_includes_page_and_final_url() {
        assert_eq!(
            window_title(Some("World Wide Web"), "http://info.cern.ch/"),
            "World Wide Web — Venture — http://info.cern.ch/"
        );
        assert_eq!(
            window_title(Some("  "), "http://info.cern.ch/"),
            "Venture — http://info.cern.ch/"
        );
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn canned_html_reaches_coretext_and_metal_pixels() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                b"<title>Venture</title><h1>World Wide Web</h1>\
                  <p><a href='next.html'>What's out there?</a></p>"
                    .to_vec(),
            ))
        };
        let session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("native browser pipeline should load canned HTML");
        let scene = session
            .viewport()
            .expect("loaded session should have a viewport")
            .viewport_scene();

        assert!(
            format!("{:?}", scene.instructions).contains("coretext:"),
            "native glyph runs should preserve CoreText font bindings"
        );
        let pixels = paint_metal::render(&scene);
        assert_eq!(pixels.width, 320);
        assert_eq!(pixels.height, 180);
        assert!(pixels
            .data
            .chunks_exact(4)
            .any(|pixel| pixel != [192, 192, 192, 255]));
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn normalized_wheel_event_scrolls_and_reprojects_the_loaded_viewport() {
        let fetcher = |url: &str| {
            let paragraphs = (0..80)
                .map(|index| format!("<p>Scrollable Venture paragraph {index}</p>"))
                .collect::<String>();
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                format!("<title>Scroll</title>{paragraphs}").into_bytes(),
            ))
        };
        let mut session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("native browser pipeline should load tall canned HTML");
        let before = session
            .viewport()
            .expect("loaded session should have a viewport")
            .viewport_scene();
        let event = WindowEvent::Scroll {
            window_id: WindowId(1),
            delta_x: 0.0,
            delta_y: 96.0,
        };

        assert!(scroll_session(&mut session, &event));
        let viewport = session
            .viewport()
            .expect("scrolling should preserve the viewport");
        assert_eq!(viewport.scroll_state().offset_y(), 96.0);
        assert_ne!(viewport.viewport_scene(), before);
    }

    #[cfg(not(target_vendor = "apple"))]
    #[test]
    fn runtime_rejects_non_apple_hosts() {
        assert!(matches!(
            run(DEFAULT_START_URL),
            Err(MacBrowserError::UnsupportedPlatform)
        ));
    }
}
