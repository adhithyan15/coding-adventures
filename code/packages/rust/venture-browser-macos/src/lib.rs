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
use window_core::{ElementState, Key, NamedKey, PointerButton, WindowError, WindowEvent};

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

/// Activate the link at a viewport coordinate through the native page
/// pipeline.
///
/// Returns `true` only when a link was hit and a replacement page loaded.
pub fn activate_link_at<F>(
    session: &mut BrowserSession,
    viewport_x: f64,
    viewport_y: f64,
    width: f64,
    height: f64,
    fetcher: &F,
) -> Result<bool, BrowserLoadError>
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
    Ok(session
        .activate_link(viewport_x, viewport_y, &pipeline, fetcher)?
        .is_some())
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
    use window_core::{LogicalSize, SurfacePreference, Window, WindowBuilder};

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
        let mut pointer_position = None;
        move |event| {
            let mut session = session.borrow_mut();
            let mut should_repaint = scroll_session(&mut session, &event)
                || keyboard_scroll_session(&mut session, &event);
            if let Some(navigation) = navigation_shortcut(&event) {
                match navigate_session(
                    &mut session,
                    navigation,
                    DEFAULT_WINDOW_WIDTH,
                    DEFAULT_WINDOW_HEIGHT,
                    &HttpBrowserFetcher::default(),
                ) {
                    Ok(true) => {
                        should_repaint = true;
                        if let Some(viewport) = session.viewport() {
                            let page = viewport.page();
                            let title =
                                window_title(page.document.title.as_deref(), &page.final_url);
                            if let Err(error) = window.set_title(&title) {
                                eprintln!(
                                    "venture-browser-macos: window title update failed: {error}"
                                );
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        eprintln!("venture-browser-macos: history navigation failed: {error}");
                    }
                }
            }
            if let Some((x, y)) = pointer_link_activation(&mut pointer_position, &event) {
                match activate_link_at(
                    &mut session,
                    x,
                    y,
                    DEFAULT_WINDOW_WIDTH,
                    DEFAULT_WINDOW_HEIGHT,
                    &HttpBrowserFetcher::default(),
                ) {
                    Ok(true) => {
                        should_repaint = true;
                        if let Some(viewport) = session.viewport() {
                            let page = viewport.page();
                            let title =
                                window_title(page.document.title.as_deref(), &page.final_url);
                            if let Err(error) = window.set_title(&title) {
                                eprintln!(
                                    "venture-browser-macos: window title update failed: {error}"
                                );
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        eprintln!("venture-browser-macos: link navigation failed: {error}");
                    }
                }
            }
            if should_repaint {
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

/// Apply host-level named scrolling keys to the current browser viewport.
pub fn keyboard_scroll_session(session: &mut BrowserSession, event: &WindowEvent) -> bool {
    let WindowEvent::Key {
        key: Key::Named(key),
        state: ElementState::Pressed,
        modifiers,
        ..
    } = event
    else {
        return false;
    };
    if modifiers.control || modifiers.alt || modifiers.meta {
        return false;
    }
    let Some(viewport) = session.viewport_mut() else {
        return false;
    };
    let previous_offset = viewport.scroll_state().offset_y();
    let viewport_height = viewport.scroll_state().viewport_height();
    match key {
        NamedKey::ArrowUp => {
            viewport.scroll_by(-40.0);
        }
        NamedKey::ArrowDown => {
            viewport.scroll_by(40.0);
        }
        NamedKey::PageUp => {
            viewport.scroll_by(-viewport_height * 0.9);
        }
        NamedKey::PageDown => {
            viewport.scroll_by(viewport_height * 0.9);
        }
        NamedKey::Space if modifiers.shift => {
            viewport.scroll_by(-viewport_height * 0.9);
        }
        NamedKey::Space => {
            viewport.scroll_by(viewport_height * 0.9);
        }
        NamedKey::Home => {
            viewport.set_scroll_offset_y(0.0);
        }
        NamedKey::End => {
            let max_offset = viewport.scroll_state().max_offset_y();
            viewport.set_scroll_offset_y(max_offset);
        }
        _ => return false,
    }
    viewport.scroll_state().offset_y() != previous_offset
}

/// Map macOS browser-history shortcuts into host-neutral navigation commands.
pub fn navigation_shortcut(event: &WindowEvent) -> Option<BrowserNavigation> {
    let WindowEvent::Key {
        key: Key::Named(key),
        state: ElementState::Pressed,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if !modifiers.meta || modifiers.shift || modifiers.control || modifiers.alt {
        return None;
    }
    match key {
        NamedKey::ArrowLeft => Some(BrowserNavigation::Back),
        NamedKey::ArrowRight => Some(BrowserNavigation::Forward),
        _ => None,
    }
}

/// Execute one native navigation command through the shared page pipeline.
///
/// Returns `true` only when the command selected and loaded a history entry.
pub fn navigate_session<F>(
    session: &mut BrowserSession,
    navigation: BrowserNavigation,
    width: f64,
    height: f64,
    fetcher: &F,
) -> Result<bool, BrowserLoadError>
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
    Ok(session.execute(navigation, &pipeline, fetcher)?.is_some())
}

/// Track pointer movement and identify a primary-button link activation.
///
/// Activation happens on release so dragging away from a link can update the
/// final viewport coordinate before navigation.
pub fn pointer_link_activation(
    pointer_position: &mut Option<(f64, f64)>,
    event: &WindowEvent,
) -> Option<(f64, f64)> {
    match event {
        WindowEvent::PointerMoved { x, y, .. } => {
            *pointer_position = Some((*x, *y));
            None
        }
        WindowEvent::PointerButton {
            button: PointerButton::Primary,
            state: ElementState::Released,
            ..
        } => *pointer_position,
        _ => None,
    }
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

    use window_core::ModifiersState;

    #[cfg(target_vendor = "apple")]
    use venture_browser_core::BrowserFetchResponse;
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

    #[test]
    fn primary_release_uses_the_latest_pointer_position() {
        let mut pointer_position = None;
        let moved = WindowEvent::PointerMoved {
            window_id: WindowId(1),
            x: 12.5,
            y: 48.0,
        };
        let pressed = WindowEvent::PointerButton {
            window_id: WindowId(1),
            button: PointerButton::Primary,
            state: ElementState::Pressed,
        };
        let released = WindowEvent::PointerButton {
            window_id: WindowId(1),
            button: PointerButton::Primary,
            state: ElementState::Released,
        };

        assert_eq!(pointer_link_activation(&mut pointer_position, &moved), None);
        assert_eq!(
            pointer_link_activation(&mut pointer_position, &pressed),
            None
        );
        assert_eq!(
            pointer_link_activation(&mut pointer_position, &released),
            Some((12.5, 48.0))
        );
    }

    #[test]
    fn command_arrows_map_to_history_navigation_only_on_press() {
        let key_event = |key, state, modifiers| WindowEvent::Key {
            window_id: WindowId(1),
            key: Key::Named(key),
            state,
            modifiers,
            text: None,
        };
        let command = ModifiersState {
            meta: true,
            ..ModifiersState::default()
        };

        assert_eq!(
            navigation_shortcut(&key_event(
                NamedKey::ArrowLeft,
                ElementState::Pressed,
                command
            )),
            Some(BrowserNavigation::Back)
        );
        assert_eq!(
            navigation_shortcut(&key_event(
                NamedKey::ArrowRight,
                ElementState::Pressed,
                command
            )),
            Some(BrowserNavigation::Forward)
        );
        assert_eq!(
            navigation_shortcut(&key_event(
                NamedKey::ArrowLeft,
                ElementState::Released,
                command
            )),
            None
        );
        assert_eq!(
            navigation_shortcut(&key_event(
                NamedKey::ArrowLeft,
                ElementState::Pressed,
                ModifiersState {
                    meta: true,
                    shift: true,
                    ..ModifiersState::default()
                }
            )),
            None
        );
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn named_keys_scroll_and_clamp_the_native_viewport() {
        let fetcher = |url: &str| {
            let paragraphs = (0..80)
                .map(|index| format!("<p>Keyboard Venture paragraph {index}</p>"))
                .collect::<String>();
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                format!("<title>Keyboard</title>{paragraphs}").into_bytes(),
            ))
        };
        let mut session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("native browser pipeline should load tall canned HTML");
        let key_event = |key, modifiers| WindowEvent::Key {
            window_id: WindowId(1),
            key: Key::Named(key),
            state: ElementState::Pressed,
            modifiers,
            text: None,
        };

        assert!(keyboard_scroll_session(
            &mut session,
            &key_event(NamedKey::ArrowDown, ModifiersState::default())
        ));
        assert_eq!(
            session
                .viewport()
                .expect("viewport should remain loaded")
                .scroll_state()
                .offset_y(),
            40.0
        );
        assert!(keyboard_scroll_session(
            &mut session,
            &key_event(NamedKey::End, ModifiersState::default())
        ));
        let max_offset = session
            .viewport()
            .expect("viewport should remain loaded")
            .scroll_state()
            .max_offset_y();
        assert_eq!(
            session
                .viewport()
                .expect("viewport should remain loaded")
                .scroll_state()
                .offset_y(),
            max_offset
        );
        assert!(keyboard_scroll_session(
            &mut session,
            &key_event(NamedKey::Home, ModifiersState::default())
        ));
        assert_eq!(
            session
                .viewport()
                .expect("viewport should remain loaded")
                .scroll_state()
                .offset_y(),
            0.0
        );
        assert!(!keyboard_scroll_session(
            &mut session,
            &key_event(
                NamedKey::ArrowDown,
                ModifiersState {
                    meta: true,
                    ..ModifiersState::default()
                }
            )
        ));
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn repeated_link_activation_reloads_the_native_viewport_and_history() {
        let fetcher = |url: &str| {
            let html = match url {
                "http://example.test/" => "<title>Start</title><p><a href='next.html'>Next</a></p>",
                "http://example.test/next.html" => {
                    "<title>Next</title><p><a href='final.html'>Final</a></p>"
                }
                "http://example.test/final.html" => "<title>Final</title><p>Done</p>",
                other => return Err(format!("unexpected URL: {other}")),
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                html.as_bytes().to_vec(),
            ))
        };
        let mut session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("initial native page should load");

        for expected_url in [
            "http://example.test/next.html",
            "http://example.test/final.html",
        ] {
            let link = session
                .viewport()
                .and_then(|viewport| viewport.page().paint.links.first())
                .cloned()
                .expect("current page should expose a link");
            assert!(activate_link_at(
                &mut session,
                link.x + link.width / 2.0,
                link.y + link.height / 2.0,
                320.0,
                180.0,
                &fetcher,
            )
            .expect("link activation should load"));
            assert_eq!(session.history().current_url(), Some(expected_url));
        }

        assert_eq!(session.history().back_stack().len(), 2);
        assert_eq!(
            session
                .viewport()
                .and_then(|viewport| viewport.page().document.title.as_deref()),
            Some("Final")
        );
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn command_arrows_reload_back_and_forward_history_entries() {
        let fetcher = |url: &str| {
            let html = match url {
                "http://example.test/" => "<title>Start</title><p><a href='next.html'>Next</a></p>",
                "http://example.test/next.html" => "<title>Next</title><p>Done</p>",
                other => return Err(format!("unexpected URL: {other}")),
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                html.as_bytes().to_vec(),
            ))
        };
        let mut session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("initial native page should load");
        let link = session
            .viewport()
            .and_then(|viewport| viewport.page().paint.links.first())
            .cloned()
            .expect("initial page should expose a link");
        assert!(activate_link_at(
            &mut session,
            link.x + link.width / 2.0,
            link.y + link.height / 2.0,
            320.0,
            180.0,
            &fetcher,
        )
        .expect("link activation should load"));

        assert!(navigate_session(
            &mut session,
            BrowserNavigation::Back,
            320.0,
            180.0,
            &fetcher,
        )
        .expect("back should load"));
        assert_eq!(
            session
                .viewport()
                .and_then(|viewport| viewport.page().document.title.as_deref()),
            Some("Start")
        );
        assert!(navigate_session(
            &mut session,
            BrowserNavigation::Forward,
            320.0,
            180.0,
            &fetcher,
        )
        .expect("forward should load"));
        assert_eq!(
            session
                .viewport()
                .and_then(|viewport| viewport.page().document.title.as_deref()),
            Some("Next")
        );
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
