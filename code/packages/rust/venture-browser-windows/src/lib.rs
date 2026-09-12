//! Native page-content bridge for Venture's Mosaic-generated WinUI shell.
//!
//! Mosaic remains the sole owner of browser chrome. This crate joins the
//! host-neutral Venture session and chrome reducer to a Direct2D pixel surface
//! that the package-owned XAML adapter mounts in the generated `HostSurface`.

use browser_bookmarks_file::{default_bookmark_path, FileBookmarkRepository};
use html_to_layout::mosaic_html_theme;
use html_to_paint::HtmlPaintViewport;
use layout_text_measure_native::NativeMeasurer;
use text_native::{NativeMetrics, NativeResolver, NativeShaper};
use venture_browser_core::{
    BookmarkRepository, BrowserChromeEvent, BrowserChromeProps, BrowserCommandError,
    BrowserFetchRequest, BrowserFetchResponse, BrowserHostController, BrowserHostEventOutcome,
    BrowserLoadError, BrowserNavigation, BrowserNavigationUpdate, BrowserPagePipeline,
    BrowserResourceFetcher, BrowserScrollCommand, BrowserScrollMetrics, BrowserSession,
    BrowserSubresourceCompletion, BrowserSubresourceUpdate, ControlKey,
    ControlSuggestionPickerAction, HostFileSelection, HttpBrowserFetcher, MemoryBookmarkRepository,
};

#[cfg(any(target_os = "windows", test))]
use venture_browser_core::BrowserHostEffect;

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_START_URL: &str = "http://info.cern.ch/";
pub const DEFAULT_VIEWPORT_WIDTH: f64 = 1024.0;
pub const DEFAULT_VIEWPORT_HEIGHT: f64 = 640.0;

fn execute_navigation<F>(
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

fn activate_control<F>(
    session: &mut BrowserSession,
    x: f64,
    y: f64,
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
        .activate_control_and_submit(x, y, &pipeline, fetcher)?
        .is_some())
}

fn route_control_key<F>(
    session: &mut BrowserSession,
    key: ControlKey,
    shift: bool,
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
        .control_key_down_with_shift_and_submit(key, shift, &pipeline, fetcher)?
        .is_some())
}

fn route_control_text(session: &mut BrowserSession, text: &str, width: f64, height: f64) -> bool {
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
    session.control_text_input(text, &pipeline).is_some()
}

struct OwnedFetcher(Box<dyn BrowserResourceFetcher>);

impl BrowserResourceFetcher for OwnedFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        self.0.fetch(url)
    }

    fn fetch_request(&self, request: &BrowserFetchRequest) -> Result<BrowserFetchResponse, String> {
        self.0.fetch_request(request)
    }
}

/// One browser session shared by generated chrome and the Direct2D surface.
pub struct WindowsBrowserHost {
    controller: BrowserHostController,
    bookmarks: Box<dyn BookmarkRepository>,
    fetcher: OwnedFetcher,
    width: f64,
    height: f64,
}

impl WindowsBrowserHost {
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
        let mut session = BrowserSession::new(start_url, height);
        execute_navigation(
            &mut session,
            BrowserNavigation::Navigate(start_url.to_string()),
            width,
            height,
            &fetcher,
        )?;
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
        let mut session = BrowserSession::new(start_url, height);
        execute_navigation(
            &mut session,
            BrowserNavigation::Navigate(start_url.to_string()),
            width,
            height,
            &fetcher,
        )?;
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
            |session, navigation| execute_navigation(session, navigation, width, height, fetcher),
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
        if activate_control(self.controller.session_mut(), x, y, width, height, fetcher)? {
            self.controller.synchronize_session_state();
            return Ok(true);
        }
        self.controller.activate_link(x, y, |session, navigation| {
            execute_navigation(session, navigation, width, height, fetcher)
        })
    }

    pub fn control_key_down(
        &mut self,
        key: ControlKey,
        shift: bool,
    ) -> Result<bool, BrowserLoadError> {
        let changed = route_control_key(
            self.controller.session_mut(),
            key,
            shift,
            self.width,
            self.height,
            &self.fetcher,
        )?;
        if changed {
            self.controller.synchronize_session_state();
        }
        Ok(changed)
    }

    pub fn control_text_input(&mut self, text: &str) -> bool {
        let changed =
            route_control_text(self.controller.session_mut(), text, self.width, self.height);
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

    pub fn suggestion_state_json(&self) -> Option<String> {
        self.controller
            .session()
            .focused_control_suggestion_state()
            .map(|state| state.to_host_json())
    }

    pub fn suggestion_query(&mut self, query: &str, limit: usize) -> bool {
        let Some(key) = self
            .controller
            .session()
            .controls()
            .focused_key()
            .map(str::to_owned)
        else {
            return false;
        };
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
        self.controller
            .session_mut()
            .open_control_suggestions(&key, query, limit, &pipeline)
            .is_some()
    }

    pub fn suggestion_action(&mut self, name: &str, index: usize) -> bool {
        let action = match name {
            "previous" => ControlSuggestionPickerAction::MovePrevious,
            "next" => ControlSuggestionPickerAction::MoveNext,
            "commit-active" => ControlSuggestionPickerAction::CommitActive,
            "commit-index" => ControlSuggestionPickerAction::CommitIndex(index),
            "cancel" => ControlSuggestionPickerAction::Cancel,
            _ => return false,
        };
        let Some(key) = self
            .controller
            .session()
            .controls()
            .focused_key()
            .map(str::to_owned)
        else {
            return false;
        };
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
            .control_suggestion_picker_action(&key, action, &pipeline)
            .is_some();
        if changed {
            self.controller.synchronize_session_state();
        }
        changed
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

    /// Commit navigation before inline images and return scheduler effects.
    pub fn begin_navigation(
        &mut self,
        navigation: BrowserNavigation,
    ) -> Result<BrowserNavigationUpdate, BrowserLoadError> {
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
        self.controller
            .session_mut()
            .begin_execute(navigation, &pipeline, &self.fetcher)
    }

    /// Apply one scheduler completion and report whether the surface repaints.
    pub fn complete_subresource(
        &mut self,
        completion: BrowserSubresourceCompletion,
    ) -> BrowserSubresourceUpdate {
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
        self.controller
            .session_mut()
            .complete_subresource(completion, &pipeline)
    }

    /// Reflow the retained page for a new logical content-surface size.
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

    #[cfg(target_os = "windows")]
    fn render_bgra(&self) -> Option<(u32, u32, Vec<u8>)> {
        let scene = self.controller.session().viewport()?.viewport_scene();
        let mut pixels = paint_vm_direct2d::render(&scene);
        for pixel in pixels.data.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        Some((pixels.width, pixels.height, pixels.data))
    }
}

#[cfg(target_os = "windows")]
mod ffi {
    use super::*;
    use std::ffi::{c_char, CStr, CString};
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
        host: &WindowsBrowserHost,
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
    pub extern "C" fn venture_browser_windows_new(
        start_url: *const c_char,
        width: f64,
        height: f64,
    ) -> *mut WindowsBrowserHost {
        let Some(start_url) = string_arg(start_url) else {
            return std::ptr::null_mut();
        };
        catch_unwind(|| WindowsBrowserHost::new(&start_url, width, height))
            .ok()
            .and_then(Result::ok)
            .map(Box::new)
            .map(Box::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_free(host: *mut WindowsBrowserHost) {
        if !host.is_null() {
            drop(Box::from_raw(host));
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_apply_props(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .map(|host| response(host, None, None))
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_handle_event(
        host: *mut WindowsBrowserHost,
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
        match catch_unwind(AssertUnwindSafe(|| host.handle_event_with_effect(event))) {
            Ok(Ok(outcome)) => response(host, outcome.effect.as_ref(), None),
            Ok(Err(error)) => response(host, None, Some(&error.to_string())),
            Err(_) => response(host, None, Some("Venture event handler panicked")),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_scroll(
        host: *mut WindowsBrowserHost,
        delta_y: f64,
    ) -> u8 {
        host.as_mut()
            .map(|host| host.scroll_by(delta_y) as u8)
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_scroll_command(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_control_key(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_control_text(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_control_copy(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .and_then(WindowsBrowserHost::control_copy)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_control_cut(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_mut()
            .and_then(WindowsBrowserHost::control_cut)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_control_paste(
        host: *mut WindowsBrowserHost,
        text: *const c_char,
    ) -> u8 {
        string_arg(text)
            .and_then(|text| host.as_mut().map(|host| host.control_paste(&text) as u8))
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_caret_tick(
        host: *mut WindowsBrowserHost,
        elapsed_ms: u64,
    ) -> u8 {
        host.as_mut()
            .map(|host| host.advance_caret_blink(elapsed_ms) as u8)
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_ime_candidate_rect(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_mut()
            .and_then(WindowsBrowserHost::ime_candidate_rect_json)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_file_picker_request(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .and_then(WindowsBrowserHost::file_picker_request_json)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_suggestion_state(
        host: *mut WindowsBrowserHost,
    ) -> *mut c_char {
        host.as_ref()
            .and_then(WindowsBrowserHost::suggestion_state_json)
            .and_then(|value| CString::new(value).ok())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_suggestion_query(
        host: *mut WindowsBrowserHost,
        query: *const c_char,
        limit: usize,
    ) -> u8 {
        string_arg(query)
            .and_then(|query| {
                host.as_mut()
                    .map(|host| host.suggestion_query(&query, limit) as u8)
            })
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_suggestion_action(
        host: *mut WindowsBrowserHost,
        name: *const c_char,
        index: usize,
    ) -> u8 {
        string_arg(name)
            .and_then(|name| {
                host.as_mut()
                    .map(|host| host.suggestion_action(&name, index) as u8)
            })
            .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_control_file(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_activate_link(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_update_hover(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_scroll_metrics(
        host: *mut WindowsBrowserHost,
        offset_y: *mut f64,
        viewport_height: *mut f64,
        content_height: *mut f64,
        max_offset_y: *mut f64,
    ) -> u8 {
        catch_unwind(AssertUnwindSafe(|| {
            let Some(metrics) = host.as_ref().and_then(WindowsBrowserHost::scroll_metrics) else {
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
    pub unsafe extern "C" fn venture_browser_windows_scroll_to(
        host: *mut WindowsBrowserHost,
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
    pub unsafe extern "C" fn venture_browser_windows_resize(
        host: *mut WindowsBrowserHost,
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

    /// Render BGRA8 pixels for WinUI's `WriteableBitmap`.
    ///
    /// The required byte count is returned for both probe and copy calls. A
    /// null or undersized output buffer is never written.
    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_render_bgra(
        host: *mut WindowsBrowserHost,
        output: *mut u8,
        capacity: usize,
        width: *mut u32,
        height: *mut u32,
    ) -> usize {
        catch_unwind(AssertUnwindSafe(|| {
            let Some(host) = host.as_ref() else {
                return 0;
            };
            let Some((pixel_width, pixel_height, pixels)) = host.render_bgra() else {
                return 0;
            };
            if !width.is_null() {
                *width = pixel_width;
            }
            if !height.is_null() {
                *height = pixel_height;
            }
            if !output.is_null() && capacity >= pixels.len() {
                std::ptr::copy_nonoverlapping(pixels.as_ptr(), output, pixels.len());
            }
            pixels.len()
        }))
        .unwrap_or(0)
    }

    #[no_mangle]
    pub unsafe extern "C" fn venture_browser_windows_string_free(value: *mut c_char) {
        if !value.is_null() {
            drop(CString::from_raw(value));
        }
    }
}

fn finite_positive_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback.max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    use venture_browser_visual_fixtures::{fixture_response, probe_rgba, FIXTURE_PATH};

    fn page(url: &str, title: &str, body: &str) -> BrowserFetchResponse {
        BrowserFetchResponse::new(
            url,
            200,
            Some("text/html; charset=utf-8".to_string()),
            format!("<html><head><title>{title}</title></head><body>{body}</body></html>")
                .into_bytes(),
        )
    }

    #[test]
    fn generated_chrome_and_content_share_one_browser_session() {
        let fetcher = |url: &str| match url {
            "http://example.test/" => Ok(page(
                url,
                "Home",
                "<a href='http://example.test/next'>Next</a>",
            )),
            "http://example.test/next" => Ok(page(url, "Next", "done")),
            _ => Err(format!("unexpected URL {url}")),
        };
        let mut host = WindowsBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("initial page loads");

        assert_eq!(host.props().page_title, "Home");
        let source = host
            .handle_event_with_effect(BrowserChromeEvent::ViewSource)
            .expect("view source succeeds");
        let Some(BrowserHostEffect::OpenAuxiliaryDocument(document)) = source.effect else {
            panic!("view source must request an auxiliary document");
        };
        assert!(document.html.contains("&lt;title&gt;Home&lt;/title&gt;"));
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
        assert_eq!(host.props().status_text, "http://example.test/next");
        assert!(!host.update_hover(f64::NAN, f64::NAN));
        assert_eq!(host.props().status_text, "Ready");
        host.handle_event(BrowserChromeEvent::AddressChange(
            "http://example.test/next".to_string(),
        ))
        .expect("address draft updates");
        assert!(host
            .handle_event(BrowserChromeEvent::Navigate)
            .expect("navigation succeeds"));
        let props = host.props();
        assert_eq!(props.address, "http://example.test/next");
        assert_eq!(props.page_title, "Next");
        assert!(!props.back_disabled);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn production_direct2d_bridge_renders_the_real_page_visual_fixture() {
        let origin = "http://venture.test";
        let start_url = format!("{origin}{FIXTURE_PATH}");
        let fetcher = move |url: &str| fixture_response(origin, url);
        let mut host =
            WindowsBrowserHost::new_with_fetcher(&start_url, 240.0, 120.0, Box::new(fetcher))
                .expect("visual fixture loads through the production Direct2D bridge");
        let metrics = host.scroll_metrics().expect("fixture should scroll");
        let mut saw_link = false;
        let mut saw_image = false;
        let mut offset = 0.0;
        loop {
            host.scroll_to(offset);
            let scene = host
                .controller
                .session()
                .viewport()
                .unwrap()
                .viewport_scene();
            let pixels = paint_vm_direct2d::render(&scene);
            let probe = probe_rgba(pixels.width, pixels.height, &pixels.data)
                .expect("valid Direct2D RGBA frame");
            saw_link |= probe.blue_pixels > 0;
            saw_image |= probe.magenta_pixels > 0 && probe.cyan_pixels > 0;
            if offset >= metrics.max_offset_y {
                break;
            }
            offset = (offset + 48.0).min(metrics.max_offset_y);
        }
        assert!(saw_link, "Direct2D frames lost wrapped-link paint");
        assert!(saw_image, "Direct2D frames lost decoded image paint");
    }

    #[test]
    fn semantic_keyboard_scroll_commands_drive_the_windows_session() {
        let body = (0..80)
            .map(|index| format!("<p>Keyboard Venture paragraph {index}</p>"))
            .collect::<String>();
        let fetcher = move |url: &str| match url {
            "http://example.test/" => Ok(page(url, "Keyboard", &body)),
            _ => Err(format!("unexpected URL {url}")),
        };
        let mut host = WindowsBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("initial page loads");

        assert!(host.scroll_command(BrowserScrollCommand::LineDown));
        assert!(host.scroll_command(BrowserScrollCommand::DocumentEnd));
        assert!(host.scroll_command(BrowserScrollCommand::DocumentStart));
        assert!(!host.scroll_command(BrowserScrollCommand::DocumentStart));
    }

    #[test]
    fn native_control_input_routes_through_retained_editor_state() {
        let fetcher = |url: &str| match url {
            "http://example.test/" => Ok(page(
                url,
                "Controls",
                "<input id='q' value='venture'><button>Go</button>",
            )),
            _ => Err(format!("unexpected URL {url}")),
        };
        let mut host = WindowsBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("initial page loads");
        let input = host
            .controller
            .session()
            .viewport()
            .unwrap()
            .page()
            .paint
            .controls[0]
            .clone();

        assert!(host.activate_link(input.x + 1.0, input.y + 1.0).unwrap());
        assert!(host.control_key_down(ControlKey::Home, false).unwrap());
        assert!(host.control_text_input("native-"));
        assert_eq!(
            host.controller.session().controls().controls()[0].value,
            "native-venture"
        );
        assert!(host.control_key_down(ControlKey::Home, false).unwrap());
        assert!(host.control_key_down(ControlKey::ArrowRight, true).unwrap());
        assert_eq!(host.control_copy().as_deref(), Some("n"));
        assert_eq!(host.control_cut().as_deref(), Some("n"));
        assert!(host.control_paste("N"));
        assert_eq!(
            host.controller.session().controls().controls()[0].value,
            "Native-venture"
        );
    }

    #[test]
    fn native_resize_reflows_without_refetching_the_document() {
        use std::cell::Cell;
        use std::rc::Rc;

        let fetches = Rc::new(Cell::new(0usize));
        let observed_fetches = Rc::clone(&fetches);
        let body = (0..40)
            .map(|index| format!("<p>Resizable Venture paragraph {index} wraps.</p>"))
            .collect::<String>();
        let fetcher = move |url: &str| {
            observed_fetches.set(observed_fetches.get() + 1);
            match url {
                "http://example.test/" => Ok(page(url, "Resize", &body)),
                _ => Err(format!("unexpected URL {url}")),
            }
        };
        let mut host = WindowsBrowserHost::new_with_fetcher(
            "http://example.test/",
            320.0,
            180.0,
            Box::new(fetcher),
        )
        .expect("initial page loads");

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
}
