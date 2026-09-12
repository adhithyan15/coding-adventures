//! Runnable macOS host for the Venture browser.
//!
//! The host deliberately stays thin: shared crates own navigation, fetching,
//! HTML parsing, layout, paint composition, scrolling, and hit-testing. This
//! crate selects native CoreText services, creates an AppKit Metal window, and
//! presents the loaded viewport.

#[cfg(target_vendor = "apple")]
use browser_bookmarks_file::{default_bookmark_path, FileBookmarkRepository};
use html_to_layout::mosaic_html_theme;
use html_to_paint::HtmlPaintViewport;
use layout_text_measure_native::NativeMeasurer;
use std::fmt;
#[cfg(target_vendor = "apple")]
use std::{cell::RefCell, rc::Rc};
use text_native::{NativeMetrics, NativeResolver, NativeShaper};
use venture_browser_core::{
    BrowserLoadError, BrowserNavigation, BrowserNavigationUpdate, BrowserPagePipeline,
    BrowserResourceFetcher, BrowserScrollCommand, BrowserSession, BrowserSubresourceCompletion,
    BrowserSubresourceUpdate, ControlKey,
};
use window_core::{ElementState, Key, NamedKey, PointerButton, WindowError, WindowEvent};

#[cfg(target_vendor = "apple")]
use venture_browser_core::{
    BookmarkRepository, BrowserChromeEvent, BrowserChromeProps, BrowserCommandError,
    BrowserFetchRequest, BrowserHostController, BrowserHostEffect, BrowserHostEventOutcome,
    BrowserScrollMetrics, HostFileSelection, HttpBrowserFetcher, MemoryBookmarkRepository,
};

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
fn activate_control_at<F>(
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
        .activate_control_and_submit(viewport_x, viewport_y, &pipeline, fetcher)?
        .is_some())
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
    if session
        .activate_control_and_submit(viewport_x, viewport_y, &pipeline, fetcher)?
        .is_some()
    {
        return Ok(true);
    }
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
            let mut should_repaint = match control_input_session(
                &mut session,
                &event,
                DEFAULT_WINDOW_WIDTH,
                DEFAULT_WINDOW_HEIGHT,
                &HttpBrowserFetcher::default(),
            ) {
                Ok(changed) => changed,
                Err(error) => {
                    eprintln!("venture-browser-macos: form input failed: {error}");
                    false
                }
            };
            if !should_repaint {
                should_repaint = scroll_session(&mut session, &event)
                    || keyboard_scroll_session(&mut session, &event);
            }
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

/// Route normalized native keyboard and text events into retained controls.
/// Editing policy stays in `BrowserSession`; this adapter only translates
/// window key identities and supplies the current page pipeline.
pub fn control_input_session<F>(
    session: &mut BrowserSession,
    event: &WindowEvent,
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
    match event {
        WindowEvent::TextInput { text, .. } => {
            Ok(session.control_text_input(text, &pipeline).is_some())
        }
        WindowEvent::Key {
            key: Key::Named(NamedKey::Tab),
            state: ElementState::Pressed,
            modifiers,
            ..
        } if !modifiers.control && !modifiers.alt && !modifiers.meta => {
            Ok(session.focus_control(modifiers.shift, &pipeline).is_some())
        }
        WindowEvent::Key {
            key: Key::Named(key),
            state: ElementState::Pressed,
            modifiers,
            ..
        } if !modifiers.control && !modifiers.alt && !modifiers.meta => {
            let key = match key {
                NamedKey::Backspace => ControlKey::Backspace,
                NamedKey::ArrowLeft => ControlKey::ArrowLeft,
                NamedKey::ArrowRight => ControlKey::ArrowRight,
                NamedKey::ArrowUp => ControlKey::ArrowUp,
                NamedKey::ArrowDown => ControlKey::ArrowDown,
                NamedKey::Home => ControlKey::Home,
                NamedKey::End => ControlKey::End,
                NamedKey::Enter => ControlKey::Enter,
                NamedKey::Space => ControlKey::Space,
                _ => return Ok(false),
            };
            Ok(session
                .control_key_down_with_shift_and_submit(key, modifiers.shift, &pipeline, fetcher)?
                .is_some())
        }
        _ => Ok(false),
    }
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
    let command = match key {
        NamedKey::ArrowUp => BrowserScrollCommand::LineUp,
        NamedKey::ArrowDown => BrowserScrollCommand::LineDown,
        NamedKey::PageUp => BrowserScrollCommand::PageUp,
        NamedKey::PageDown => BrowserScrollCommand::PageDown,
        NamedKey::Space if modifiers.shift => BrowserScrollCommand::PageUp,
        NamedKey::Space => BrowserScrollCommand::PageDown,
        NamedKey::Home => BrowserScrollCommand::DocumentStart,
        NamedKey::End => BrowserScrollCommand::DocumentEnd,
        _ => return false,
    };
    let Some(viewport) = session.viewport_mut() else {
        return false;
    };
    let previous_offset = viewport.scroll_state().offset_y();
    viewport.scroll_command(command);
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

/// Commit a native navigation before inline image requests complete.
pub fn begin_navigate_session<F>(
    session: &mut BrowserSession,
    navigation: BrowserNavigation,
    width: f64,
    height: f64,
    document_fetcher: &F,
) -> Result<BrowserNavigationUpdate, BrowserLoadError>
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
    session.begin_execute(navigation, &pipeline, document_fetcher)
}

/// Repaint a retained native page after one scheduler completion.
pub fn complete_session_subresource(
    session: &mut BrowserSession,
    completion: BrowserSubresourceCompletion,
    width: f64,
    height: f64,
) -> BrowserSubresourceUpdate {
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
    session.complete_subresource(completion, &pipeline)
}

/// The concrete adapter behind Venture's generated Mosaic SwiftUI shell.
///
/// Chrome is still authored by MIL/MLL/MSL and navigation is still reduced by
/// `venture-browser-core`; this type only joins that shared state to the native
/// Metal content surface required by the generated `MosaicHost` seam.
#[cfg(target_vendor = "apple")]
struct OwnedFetcher(Box<dyn BrowserResourceFetcher>);

#[cfg(target_vendor = "apple")]
impl BrowserResourceFetcher for OwnedFetcher {
    fn fetch(&self, url: &str) -> Result<venture_browser_core::BrowserFetchResponse, String> {
        self.0.fetch(url)
    }

    fn fetch_request(
        &self,
        request: &BrowserFetchRequest,
    ) -> Result<venture_browser_core::BrowserFetchResponse, String> {
        self.0.fetch_request(request)
    }
}

#[cfg(target_vendor = "apple")]
pub struct MacBrowserHost {
    controller: BrowserHostController,
    bookmarks: Box<dyn BookmarkRepository>,
    fetcher: OwnedFetcher,
    width: f64,
    height: f64,
}

#[cfg(target_vendor = "apple")]
impl MacBrowserHost {
    pub fn new(start_url: &str, width: f64, height: f64) -> Result<Self, BrowserCommandError> {
        let path = default_bookmark_path()?;
        Self::new_with_fetcher_and_bookmarks(
            start_url,
            width,
            height,
            Box::new(HttpBrowserFetcher::default()),
            Box::new(FileBookmarkRepository::new(path)),
        )
    }

    pub fn new_with_fetcher(
        start_url: &str,
        width: f64,
        height: f64,
        fetcher: Box<dyn BrowserResourceFetcher>,
    ) -> Result<Self, BrowserLoadError> {
        let fetcher = OwnedFetcher(fetcher);
        let session = load_initial_session(start_url, width, height, &fetcher)?;
        Ok(Self {
            controller: BrowserHostController::new(session),
            bookmarks: Box::new(MemoryBookmarkRepository::default()),
            fetcher,
            width,
            height,
        })
    }

    pub fn new_with_fetcher_and_bookmarks(
        start_url: &str,
        width: f64,
        height: f64,
        fetcher: Box<dyn BrowserResourceFetcher>,
        mut bookmarks: Box<dyn BookmarkRepository>,
    ) -> Result<Self, BrowserCommandError> {
        let fetcher = OwnedFetcher(fetcher);
        let catalog = bookmarks.load()?;
        let mut session = load_initial_session(start_url, width, height, &fetcher)?;
        session.replace_bookmarks(catalog);
        Ok(Self {
            controller: BrowserHostController::new(session),
            bookmarks,
            fetcher,
            width,
            height,
        })
    }

    pub fn props(&self) -> BrowserChromeProps {
        self.controller.props()
    }

    pub fn handle_event(&mut self, event: BrowserChromeEvent) -> Result<bool, BrowserCommandError> {
        Ok(self.handle_event_with_effect(event)?.changed)
    }

    pub fn handle_event_with_effect(
        &mut self,
        event: BrowserChromeEvent,
    ) -> Result<BrowserHostEventOutcome, BrowserCommandError> {
        let width = self.width;
        let height = self.height;
        let fetcher = &self.fetcher;
        self.controller.handle_event_with_effect(
            event,
            self.bookmarks.as_mut(),
            |session, navigation| navigate_session(session, navigation, width, height, fetcher),
        )
    }

    pub fn scroll_by(&mut self, delta_y: f64) -> bool {
        self.controller.scroll_by(delta_y)
    }

    pub fn scroll_command(&mut self, command: BrowserScrollCommand) -> bool {
        self.controller.scroll_command(command)
    }

    pub fn scroll_metrics(&self) -> Option<BrowserScrollMetrics> {
        self.controller.scroll_metrics()
    }

    pub fn scroll_to(&mut self, offset_y: f64) -> bool {
        self.controller.scroll_to(offset_y)
    }

    pub fn activate_link(&mut self, x: f64, y: f64) -> Result<bool, BrowserLoadError> {
        let width = self.width;
        let height = self.height;
        let fetcher = &self.fetcher;
        if activate_control_at(self.controller.session_mut(), x, y, width, height, fetcher)? {
            self.controller.synchronize_session_state();
            return Ok(true);
        }
        self.controller.activate_link(x, y, |session, navigation| {
            navigate_session(session, navigation, width, height, fetcher)
        })
    }

    pub fn control_key_down(
        &mut self,
        key: ControlKey,
        shift: bool,
    ) -> Result<bool, BrowserLoadError> {
        let theme = mosaic_html_theme();
        let measurer = NativeMeasurer::new();
        let shaper = NativeShaper::new();
        let metrics = NativeMetrics::new();
        let resolver = NativeResolver::new();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(self.width, self.height, 1.0),
            &measurer,
            &shaper,
            &metrics,
            &resolver,
        );
        let changed = self
            .controller
            .session_mut()
            .control_key_down_with_shift_and_submit(key, shift, &pipeline, &self.fetcher)?
            .is_some();
        if changed {
            self.controller.synchronize_session_state();
        }
        Ok(changed)
    }

    pub fn control_text_input(&mut self, text: &str) -> bool {
        let theme = mosaic_html_theme();
        let measurer = NativeMeasurer::new();
        let shaper = NativeShaper::new();
        let metrics = NativeMetrics::new();
        let resolver = NativeResolver::new();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(self.width, self.height, 1.0),
            &measurer,
            &shaper,
            &metrics,
            &resolver,
        );
        let changed = self
            .controller
            .session_mut()
            .control_text_input(text, &pipeline)
            .is_some();
        if changed {
            self.controller.synchronize_session_state();
        }
        changed
    }

    pub fn control_copy(&self) -> Option<String> {
        self.controller.session().control_copy()
    }

    pub fn control_cut(&mut self) -> Option<String> {
        let theme = mosaic_html_theme();
        let measurer = NativeMeasurer::new();
        let shaper = NativeShaper::new();
        let metrics = NativeMetrics::new();
        let resolver = NativeResolver::new();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(self.width, self.height, 1.0),
            &measurer,
            &shaper,
            &metrics,
            &resolver,
        );
        let payload = self.controller.session_mut().control_cut(&pipeline);
        if payload.is_some() {
            self.controller.synchronize_session_state();
        }
        payload
    }

    pub fn control_paste(&mut self, text: &str) -> bool {
        self.control_text_input(text)
    }

    pub fn advance_caret_blink(&mut self, elapsed_ms: u64) -> bool {
        self.controller
            .session_mut()
            .control_advance_caret_blink(elapsed_ms)
    }

    pub fn ime_candidate_rect_json(&mut self) -> Option<String> {
        let rect = self.controller.session_mut().focused_ime_candidate_rect()?;
        Some(format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            rect.x, rect.y, rect.width, rect.height
        ))
    }

    pub fn file_picker_request_json(&self) -> Option<String> {
        self.controller
            .session()
            .focused_file_picker_request()
            .map(|request| request.to_host_json())
    }

    pub fn control_file_selected(
        &mut self,
        key: &str,
        file: HostFileSelection,
        append: bool,
    ) -> bool {
        if self
            .controller
            .session()
            .controls()
            .file_state(key)
            .is_none()
        {
            return false;
        }
        let mut files = if append {
            self.controller
                .session()
                .controls()
                .selected_files(key)
                .unwrap_or_default()
                .to_vec()
        } else {
            Vec::new()
        };
        files.push(file);
        let theme = mosaic_html_theme();
        let measurer = NativeMeasurer::new();
        let shaper = NativeShaper::new();
        let metrics = NativeMetrics::new();
        let resolver = NativeResolver::new();
        let pipeline = BrowserPagePipeline::new(
            &theme,
            HtmlPaintViewport::new(self.width, self.height, 1.0),
            &measurer,
            &shaper,
            &metrics,
            &resolver,
        );
        let changed = self
            .controller
            .session_mut()
            .control_files_selected(key, files, &pipeline)
            .is_some();
        if changed {
            self.controller.synchronize_session_state();
        }
        changed
    }

    pub fn update_hover(&mut self, x: f64, y: f64) -> bool {
        self.controller.update_hover(x, y)
    }

    pub fn begin_navigation(
        &mut self,
        navigation: BrowserNavigation,
    ) -> Result<BrowserNavigationUpdate, BrowserLoadError> {
        begin_navigate_session(
            self.controller.session_mut(),
            navigation,
            self.width,
            self.height,
            &self.fetcher,
        )
    }

    pub fn complete_subresource(
        &mut self,
        completion: BrowserSubresourceCompletion,
    ) -> BrowserSubresourceUpdate {
        complete_session_subresource(
            self.controller.session_mut(),
            completion,
            self.width,
            self.height,
        )
    }

    /// Reflow the retained page for a new logical content-surface size.
    /// The HTML document and navigation history stay in the shared session;
    /// only layout, paint, image placement, and scroll bounds are recomputed.
    pub fn resize(&mut self, width: f64, height: f64) -> bool {
        self.controller.clear_hover();
        let width = finite_positive_or(width, self.width);
        let height = finite_positive_or(height, self.height);
        if self.width == width && self.height == height {
            return false;
        }

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
        let reflowed = self
            .controller
            .session_mut()
            .reflow(&pipeline, &self.fetcher, height)
            .is_some();
        self.width = width;
        self.height = height;
        reflowed
    }

    pub fn render_to_layer(&self, layer: objc_bridge::Id) -> Result<(), MacBrowserError> {
        let viewport = self
            .controller
            .session()
            .viewport()
            .ok_or(MacBrowserError::MissingViewport)?;
        paint_metal::render_to_metal_layer(&viewport.viewport_scene(), layer)
            .map_err(|error| MacBrowserError::Paint(error.to_string()))
    }
}

#[cfg(target_vendor = "apple")]
mod mosaic_ffi {
    use super::*;
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn string_arg(value: *const c_char) -> Option<String> {
        if value.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(value) }
            .to_str()
            .ok()
            .map(str::to_string)
    }

    fn json_string(value: &str) -> String {
        let mut out = String::with_capacity(value.len() + 2);
        out.push('"');
        for ch in value.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                ch if ch.is_control() => {
                    use std::fmt::Write;
                    let _ = write!(out, "\\u{:04x}", ch as u32);
                }
                ch => out.push(ch),
            }
        }
        out.push('"');
        out
    }

    fn effect_json(effect: &BrowserHostEffect) -> String {
        match effect {
            BrowserHostEffect::OpenAuxiliaryDocument(document) => format!(
                "{{\"type\":\"open-auxiliary-document\",\"document\":{{\"kind\":{},\"address\":{},\"title\":{},\"html\":{}}}}}",
                json_string(document.kind.name()),
                json_string(&document.address),
                json_string(&document.title),
                json_string(&document.html),
            ),
        }
    }

    fn response(
        host: &MacBrowserHost,
        effect: Option<&BrowserHostEffect>,
        error: Option<&str>,
    ) -> *mut c_char {
        let props = host.props();
        let effect = effect
            .map(|effect| format!(",\"effect\":{}", effect_json(effect)))
            .unwrap_or_default();
        let error = error
            .map(|message| format!(",\"error\":{}", json_string(message)))
            .unwrap_or_default();
        let value = format!(
            "{{\"props\":{{\"address\":{},\"page-title\":{},\"status-text\":{},\"back-disabled\":{},\"forward-disabled\":{},\"bookmark-label\":{},\"bookmark-disabled\":{},\"view-source-disabled\":{},\"navigation-disabled\":false}}{effect}{error}}}",
            json_string(&props.address),
            json_string(&props.page_title),
            json_string(&props.status_text),
            props.back_disabled,
            props.forward_disabled,
            json_string(&props.bookmark_label),
            props.bookmark_disabled,
            props.view_source_disabled,
        );
        CString::new(value)
            .expect("JSON response contains no NUL")
            .into_raw()
    }

    #[no_mangle]
    pub extern "C" fn venture_browser_macos_new(
        start_url: *const c_char,
        width: f64,
        height: f64,
    ) -> *mut MacBrowserHost {
        let Some(start_url) = string_arg(start_url) else {
            return std::ptr::null_mut();
        };
        catch_unwind(|| MacBrowserHost::new(&start_url, width, height))
            .ok()
            .and_then(Result::ok)
            .map(Box::new)
            .map(Box::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_free(host: *mut MacBrowserHost) {
        if !host.is_null() {
            drop(Box::from_raw(host));
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_apply_props(
        host: *mut MacBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .map(|host| response(host, None, None))
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_handle_event(
        host: *mut MacBrowserHost,
        name: *const c_char,
        value: *const c_char,
    ) -> *mut c_char {
        let Some(host) = host.as_mut() else {
            return std::ptr::null_mut();
        };
        let Some(name) = string_arg(name) else {
            return response(host, None, Some("missing Mosaic event name"));
        };
        let event = match name.as_str() {
            "onBack" => Some(BrowserChromeEvent::Back),
            "onForward" => Some(BrowserChromeEvent::Forward),
            "onHome" => Some(BrowserChromeEvent::Home),
            "onReload" => Some(BrowserChromeEvent::Reload),
            "onToggleBookmark" => Some(BrowserChromeEvent::ToggleBookmark),
            "onViewSource" => Some(BrowserChromeEvent::ViewSource),
            "onNavigate" => Some(BrowserChromeEvent::Navigate),
            "onAddressChange" => string_arg(value).map(BrowserChromeEvent::AddressChange),
            _ => None,
        };
        let Some(event) = event else {
            return response(host, None, Some("unknown or malformed Mosaic event"));
        };
        let result = catch_unwind(AssertUnwindSafe(|| host.handle_event_with_effect(event)));
        match result {
            Ok(Ok(outcome)) => response(host, outcome.effect.as_ref(), None),
            Ok(Err(error)) => response(host, None, Some(&error.to_string())),
            Err(_) => response(host, None, Some("Venture event handler panicked")),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_scroll(
        host: *mut MacBrowserHost,
        delta_y: f64,
    ) -> u8 {
        host.as_mut()
            .map(|host| host.scroll_by(delta_y) as u8)
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_scroll_command(
        host: *mut MacBrowserHost,
        name: *const c_char,
    ) -> u8 {
        let Some(command) = string_arg(name)
            .as_deref()
            .and_then(BrowserScrollCommand::from_name)
        else {
            return 0;
        };
        host.as_mut()
            .map(|host| host.scroll_command(command) as u8)
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_key(
        host: *mut MacBrowserHost,
        name: *const c_char,
        shift: u8,
    ) -> u8 {
        let Some(key) = string_arg(name).as_deref().and_then(ControlKey::from_name) else {
            return 0;
        };
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .and_then(|host| host.control_key_down(key, shift != 0).ok())
                .unwrap_or(false) as u8
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_text(
        host: *mut MacBrowserHost,
        text: *const c_char,
    ) -> u8 {
        let Some(text) = string_arg(text) else {
            return 0;
        };
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .map(|host| host.control_text_input(&text) as u8)
                .unwrap_or(0)
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_copy(
        host: *mut MacBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .and_then(MacBrowserHost::control_copy)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_cut(
        host: *mut MacBrowserHost,
    ) -> *mut c_char {
        host.as_mut()
            .and_then(MacBrowserHost::control_cut)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_paste(
        host: *mut MacBrowserHost,
        text: *const c_char,
    ) -> u8 {
        string_arg(text)
            .and_then(|text| host.as_mut().map(|host| host.control_paste(&text) as u8))
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_caret_tick(
        host: *mut MacBrowserHost,
        elapsed_ms: u64,
    ) -> u8 {
        host.as_mut()
            .map(|host| host.advance_caret_blink(elapsed_ms) as u8)
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_ime_candidate_rect(
        host: *mut MacBrowserHost,
    ) -> *mut c_char {
        host.as_mut()
            .and_then(MacBrowserHost::ime_candidate_rect_json)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_file_picker_request(
        host: *mut MacBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .and_then(MacBrowserHost::file_picker_request_json)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_control_file(
        host: *mut MacBrowserHost,
        key: *const c_char,
        opaque_id: *const c_char,
        name: *const c_char,
        media_type: *const c_char,
        bytes: *const u8,
        length: usize,
        append: u8,
    ) -> u8 {
        if bytes.is_null() && length != 0 {
            return 0;
        }
        let Some(key) = string_arg(key) else {
            return 0;
        };
        let Some(opaque_id) = string_arg(opaque_id) else {
            return 0;
        };
        let Some(name) = string_arg(name) else {
            return 0;
        };
        let payload = if length == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec()
        };
        host.as_mut()
            .map(|host| {
                host.control_file_selected(
                    &key,
                    HostFileSelection::new(opaque_id, name, string_arg(media_type), payload),
                    append != 0,
                ) as u8
            })
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_activate_link(
        host: *mut MacBrowserHost,
        x: f64,
        y: f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .and_then(|host| host.activate_link(x, y).ok())
                .unwrap_or(false) as u8
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_update_hover(
        host: *mut MacBrowserHost,
        x: f64,
        y: f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .map(|host| host.update_hover(x, y) as u8)
                .unwrap_or(0)
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_scroll_metrics(
        host: *mut MacBrowserHost,
        offset_y: *mut f64,
        viewport_height: *mut f64,
        content_height: *mut f64,
        max_offset_y: *mut f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            let Some(metrics) = host.as_ref().and_then(MacBrowserHost::scroll_metrics) else {
                return 0;
            };
            if !offset_y.is_null() {
                *offset_y = metrics.offset_y;
            }
            if !viewport_height.is_null() {
                *viewport_height = metrics.viewport_height;
            }
            if !content_height.is_null() {
                *content_height = metrics.content_height;
            }
            if !max_offset_y.is_null() {
                *max_offset_y = metrics.max_offset_y;
            }
            1
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_scroll_to(
        host: *mut MacBrowserHost,
        offset_y: f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .map(|host| host.scroll_to(offset_y) as u8)
                .unwrap_or(0)
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_resize(
        host: *mut MacBrowserHost,
        width: f64,
        height: f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            host.as_mut()
                .map(|host| host.resize(width, height) as u8)
                .unwrap_or(0)
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_macos_render(
        host: *mut MacBrowserHost,
        metal_layer: *mut c_void,
    ) -> u8 {
        if metal_layer.is_null() {
            return 0;
        }
        catch_unwind(AssertUnwindSafe(|| {
            host.as_ref()
                .map(|host| {
                    host.render_to_layer(metal_layer.cast::<objc_bridge::Object>())
                        .is_ok() as u8
                })
                .unwrap_or(0)
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_string_free(value: *mut c_char) {
        if !value.is_null() {
            drop(CString::from_raw(value));
        }
    }
}

#[cfg(target_vendor = "apple")]
fn finite_positive_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback.max(1.0)
    }
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
    #[cfg(target_os = "macos")]
    use venture_browser_visual_fixtures::{fixture_response, probe_rgba, FIXTURE_PATH};
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
    fn normalized_native_events_drive_retained_form_editing() {
        let fetcher = |url: &str| {
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                b"<input id='q' value='venture'><button>Go</button>".to_vec(),
            ))
        };
        let mut session = load_initial_session("http://example.test/", 320.0, 180.0, &fetcher)
            .expect("form fixture loads");
        let key_event = |key, shift| WindowEvent::Key {
            window_id: WindowId(1),
            key: Key::Named(key),
            state: ElementState::Pressed,
            modifiers: ModifiersState {
                shift,
                ..ModifiersState::default()
            },
            text: None,
        };

        assert!(control_input_session(
            &mut session,
            &key_event(NamedKey::Tab, false),
            320.0,
            180.0,
            &fetcher,
        )
        .unwrap());
        assert!(control_input_session(
            &mut session,
            &key_event(NamedKey::Home, false),
            320.0,
            180.0,
            &fetcher,
        )
        .unwrap());
        assert!(control_input_session(
            &mut session,
            &WindowEvent::TextInput {
                window_id: WindowId(1),
                text: "native-".into(),
            },
            320.0,
            180.0,
            &fetcher,
        )
        .unwrap());
        assert_eq!(session.controls().controls()[0].value, "native-venture");
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
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| *pixel != [192, 192, 192, 255]));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn production_metal_bridge_renders_the_real_page_visual_fixture() {
        let origin = "http://venture.test";
        let start_url = format!("{origin}{FIXTURE_PATH}");
        let fetcher = move |url: &str| fixture_response(origin, url);
        let mut host =
            MacBrowserHost::new_with_fetcher(&start_url, 240.0, 120.0, Box::new(fetcher))
                .expect("visual fixture loads through the production Metal bridge");
        let metrics = host.scroll_metrics().expect("fixture should scroll");
        let mut saw_link = false;
        let mut saw_image = false;
        let mut probes = Vec::new();
        let mut offset = 0.0;
        loop {
            host.scroll_to(offset);
            let scene = host
                .controller
                .session()
                .viewport()
                .unwrap()
                .viewport_scene();
            let pixels = paint_metal::render(&scene);
            let probe = probe_rgba(pixels.width, pixels.height, &pixels.data)
                .expect("valid Metal RGBA frame");
            saw_link |= probe.blue_pixels > 0;
            saw_image |= probe.magenta_pixels > 0 && probe.cyan_pixels > 0;
            probes.push((offset, probe));
            if offset >= metrics.max_offset_y {
                break;
            }
            offset = (offset + 48.0).min(metrics.max_offset_y);
        }
        assert!(saw_link, "Metal frames lost wrapped-link paint");
        assert!(
            saw_image,
            "Metal frames lost decoded image paint: {probes:?}"
        );
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

    #[cfg(target_vendor = "apple")]
    #[test]
    fn mosaic_host_adapter_drives_shared_chrome_and_metal_page_state() {
        let fetcher = |url: &str| {
            let html = match url {
                "http://example.test/" => "<title>Start</title><p><a href='next.html'>Next</a></p>",
                "http://example.test/next.html" => {
                    "<title>Next</title><p>Generated chrome reached Rust</p>"
                }
                other => return Err(format!("unexpected URL: {other}")),
            };
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                html.as_bytes().to_vec(),
            ))
        };
        let mut host = MacBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("Mosaic host should load the initial page");

        assert_eq!(host.props().page_title, "Start");
        let source = host
            .handle_event_with_effect(BrowserChromeEvent::ViewSource)
            .expect("view source succeeds");
        let Some(BrowserHostEffect::OpenAuxiliaryDocument(document)) = source.effect else {
            panic!("view source must request an auxiliary document");
        };
        assert!(document.html.contains("&lt;title&gt;Start&lt;/title&gt;"));
        let link = host
            .controller
            .session()
            .viewport()
            .unwrap()
            .page()
            .paint
            .links[0]
            .clone();
        assert!(host.update_hover(link.x + link.width / 2.0, link.y + link.height / 2.0));
        assert_eq!(host.props().status_text, "http://example.test/next.html");
        assert!(!host.update_hover(f64::NAN, f64::NAN));
        assert_eq!(host.props().status_text, "Ready");
        host.handle_event(BrowserChromeEvent::AddressChange(
            "http://example.test/next.html".into(),
        ))
        .expect("address edit should update the shared draft");
        assert_eq!(host.props().address, "http://example.test/next.html");
        assert!(host
            .handle_event(BrowserChromeEvent::Navigate)
            .expect("generated Navigate event should load"));

        let props = host.props();
        assert_eq!(props.page_title, "Next");
        assert_eq!(props.status_text, "Ready");
        assert!(!props.back_disabled);
        let scene = host
            .controller
            .session()
            .viewport()
            .expect("host should retain a live native viewport")
            .viewport_scene();
        let pixels = paint_metal::render(&scene);
        assert!(pixels
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| *pixel != [192, 192, 192, 255]));
    }

    #[cfg(target_vendor = "apple")]
    #[test]
    fn mosaic_host_resize_reflows_without_refetching_the_document() {
        use std::cell::Cell;
        use std::rc::Rc;

        let fetches = Rc::new(Cell::new(0usize));
        let observed_fetches = Rc::clone(&fetches);
        let body = (0..40)
            .map(|index| format!("<p>Resizable Venture paragraph {index} wraps.</p>"))
            .collect::<String>();
        let fetcher = move |url: &str| {
            observed_fetches.set(observed_fetches.get() + 1);
            Ok(BrowserFetchResponse::new(
                url,
                200,
                Some("text/html; charset=utf-8".into()),
                format!("<title>Resize</title>{body}").into_bytes(),
            ))
        };
        let mut host = MacBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("Mosaic host should load the initial page");

        let metrics = host
            .scroll_metrics()
            .expect("scroll metrics should project");
        assert!(metrics.max_offset_y > 0.0);
        assert!(host.scroll_to(metrics.max_offset_y / 2.0));
        assert_eq!(
            host.scroll_metrics().unwrap().offset_y,
            metrics.max_offset_y / 2.0
        );

        assert_eq!(
            host.controller
                .session()
                .viewport()
                .unwrap()
                .viewport_scene()
                .width,
            320.0
        );
        assert!(host.resize(144.0, 96.0));
        let viewport = host
            .controller
            .session()
            .viewport()
            .expect("viewport remains loaded");
        assert_eq!(viewport.viewport_scene().width, 144.0);
        assert_eq!(viewport.viewport_scene().height, 96.0);
        assert_eq!(fetches.get(), 1, "resize must not refetch page HTML");
        assert!(!host.resize(144.0, 96.0));
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
