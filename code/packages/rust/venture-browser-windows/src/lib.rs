//! Native page-content bridge for Venture's Mosaic-generated WinUI shell.
//!
//! Mosaic remains the sole owner of browser chrome. This crate joins the
//! host-neutral Venture session and chrome reducer to a Direct2D pixel surface
//! that the package-owned XAML adapter mounts in the generated `HostSurface`.

use html_to_layout::mosaic_html_theme;
use html_to_paint::HtmlPaintViewport;
use layout_text_measure_native::NativeMeasurer;
use text_native::{NativeMetrics, NativeResolver, NativeShaper};
use venture_browser_core::{
    BrowserChromeEvent, BrowserChromeProps, BrowserFetchResponse, BrowserHostController,
    BrowserLoadError, BrowserNavigation, BrowserPagePipeline, BrowserResourceFetcher,
    BrowserScrollCommand, BrowserScrollMetrics, BrowserSession, HttpBrowserFetcher,
};

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

struct OwnedFetcher(Box<dyn BrowserResourceFetcher>);

impl BrowserResourceFetcher for OwnedFetcher {
    fn fetch(&self, url: &str) -> Result<BrowserFetchResponse, String> {
        self.0.fetch(url)
    }
}

/// One browser session shared by generated chrome and the Direct2D surface.
pub struct WindowsBrowserHost {
    controller: BrowserHostController,
    fetcher: OwnedFetcher,
    width: f64,
    height: f64,
}

impl WindowsBrowserHost {
    pub fn new(start_url: &str, width: f64, height: f64) -> Result<Self, BrowserLoadError> {
        Self::new_with_fetcher(
            start_url,
            width,
            height,
            Box::new(HttpBrowserFetcher::default()),
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
            fetcher,
            width,
            height,
        })
    }

    pub fn props(&self) -> BrowserChromeProps {
        self.controller.props()
    }

    pub fn handle_event(&mut self, event: BrowserChromeEvent) -> Result<bool, BrowserLoadError> {
        let width = self.width;
        let height = self.height;
        let fetcher = &self.fetcher;
        self.controller.handle_event(event, |session, navigation| {
            execute_navigation(session, navigation, width, height, fetcher)
        })
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
        self.controller.activate_link(x, y, |session, navigation| {
            execute_navigation(session, navigation, width, height, fetcher)
        })
    }

    pub fn update_hover(&mut self, x: f64, y: f64) -> bool {
        self.controller.update_hover(x, y)
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
        for pixel in pixels.data.chunks_exact_mut(4) {
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

    fn response(host: &WindowsBrowserHost, error: Option<&str>) -> *mut c_char {
        let props = host.props();
        let error = error
            .map(|message| format!(",\"error\":{}", json_string(message)))
            .unwrap_or_default();
        let value = format!(
            "{{\"props\":{{\"address\":{},\"page-title\":{},\"status-text\":{},\"back-disabled\":{},\"forward-disabled\":{},\"navigation-disabled\":false}}{error}}}",
            json_string(&props.address),
            json_string(&props.page_title),
            json_string(&props.status_text),
            props.back_disabled,
            props.forward_disabled,
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
            .map(|host| response(host, None))
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
            return response(host, Some("missing Mosaic event name"));
        };
        let event = match name.as_str() {
            "onBack" => Some(BrowserChromeEvent::Back),
            "onForward" => Some(BrowserChromeEvent::Forward),
            "onHome" => Some(BrowserChromeEvent::Home),
            "onReload" => Some(BrowserChromeEvent::Reload),
            "onNavigate" => Some(BrowserChromeEvent::Navigate),
            "onAddressChange" => string_arg(value).map(BrowserChromeEvent::AddressChange),
            _ => None,
        };
        let Some(event) = event else {
            return response(host, Some("unknown or malformed Mosaic event"));
        };
        match catch_unwind(AssertUnwindSafe(|| host.handle_event(event))) {
            Ok(Ok(_)) => response(host, None),
            Ok(Err(error)) => response(host, Some(&error.to_string())),
            Err(_) => response(host, Some("Venture event handler panicked")),
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
