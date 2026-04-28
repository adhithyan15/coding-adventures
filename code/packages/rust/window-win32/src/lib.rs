//! # window-win32
//!
//! Win32 desktop window backend for `window-core`.
//!
//! This crate now includes a concrete Win32 host implementation for native
//! rendering workflows. It validates `window-core` attributes, creates a basic
//! top-level `HWND`, and hosts a message loop with an optional paint callback.
//!
//! The paint callback is intentionally minimal for now: it is wired to a fixed
//! `WM_PAINT` path so renderers can plug in their native drawing routines
//! without duplicating the full HWND creation boilerplate.

#[cfg(target_os = "windows")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

use window_core::{
    LogicalSize, MountTarget, PhysicalSize, RenderTarget, SurfacePreference, Win32RenderTarget,
    Window, WindowAttributes, WindowBackend, WindowError, WindowId,
};

pub const VERSION: &str = "0.1.0";

#[cfg(target_os = "windows")]
use windows::core::{w, PCWSTR};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetStockObject, InvalidateRect, HBRUSH, PAINTSTRUCT, WHITE_BRUSH,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW, PostQuitMessage,
    RegisterClassW, SetWindowTextW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, HMENU, IDC_ARROW, SW_HIDE, SW_SHOW, WM_CREATE, WM_DESTROY, WM_NCDESTROY,
    WM_PAINT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

/// Which Win32 host family the renderer should expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Win32SurfaceChoice {
    /// A normal `HWND`-backed presentation host.
    Hwnd,
}

#[cfg(target_os = "windows")]
type RawWindowHandle = HWND;
#[cfg(not(target_os = "windows"))]
type RawWindowHandle = usize;

/// A created Win32 window host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Win32Window {
    id: WindowId,
    logical_size: LogicalSize,
    physical_size: PhysicalSize,
    #[allow(dead_code)]
    scale_factor: f64,
    hwnd: RawWindowHandle,
}

impl Win32Window {
    /// Return the raw Win32 window handle.
    #[cfg(target_os = "windows")]
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Return the stored raw handle placeholder on non-Windows targets.
    #[cfg(not(target_os = "windows"))]
    pub fn hwnd(&self) -> usize {
        self.hwnd
    }
}

impl Window for Win32Window {
    fn id(&self) -> WindowId {
        self.id
    }

    fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    fn physical_size(&self) -> window_core::PhysicalSize {
        self.physical_size
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn request_redraw(&self) -> Result<(), WindowError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let _ = InvalidateRect(self.hwnd, None, false);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        Err(WindowError::UnsupportedPlatform(
            "Win32 windows are only available on Windows",
        ))
    }

    fn set_title(&self, _title: &str) -> Result<(), WindowError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let title = wide_null(_title);
            let _ = SetWindowTextW(self.hwnd, PCWSTR(title.as_ptr()));
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        Err(WindowError::UnsupportedPlatform(
            "Win32 windows are only available on Windows",
        ))
    }

    fn set_visible(&self, _visible: bool) -> Result<(), WindowError> {
        #[cfg(target_os = "windows")]
        unsafe {
            let _ = if _visible {
                ShowWindow(self.hwnd, SW_SHOW)
            } else {
                ShowWindow(self.hwnd, SW_HIDE)
            };
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        Err(WindowError::UnsupportedPlatform(
            "Win32 windows are only available on Windows",
        ))
    }

    fn render_target(&self) -> RenderTarget {
        #[cfg(target_os = "windows")]
        let hwnd = self.hwnd.0 as usize;
        #[cfg(not(target_os = "windows"))]
        let hwnd = self.hwnd;

        RenderTarget::Win32(Win32RenderTarget { hwnd })
    }
}

/// Per-window callback invoked from `WM_PAINT`.
#[cfg(target_os = "windows")]
pub type WindowPaintCallback = unsafe extern "system" fn(HWND, isize);
#[cfg(not(target_os = "windows"))]
pub type WindowPaintCallback = unsafe extern "system" fn(usize, isize);

#[derive(Debug, Clone, Copy)]
#[cfg(target_os = "windows")]
struct PaintHandler {
    callback: Option<WindowPaintCallback>,
    user_data: isize,
}

#[cfg(target_os = "windows")]
static PAINT_HANDLERS: OnceLock<Mutex<HashMap<isize, PaintHandler>>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn paint_handlers() -> &'static Mutex<HashMap<isize, PaintHandler>> {
    PAINT_HANDLERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Backend shell for Win32 validation and native creation.
#[derive(Debug, Default)]
pub struct Win32Backend {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    next_id: u64,
}

impl Win32Backend {
    /// Construct a new backend shell.
    pub const fn new() -> Self {
        Self { next_id: 0 }
    }

    /// Human-readable backend name.
    pub const fn backend_name(&self) -> &'static str {
        "win32"
    }

    /// Choose the Win32 host surface implied by the renderer preference.
    pub fn choose_surface(
        &self,
        preference: SurfacePreference,
    ) -> Result<Win32SurfaceChoice, WindowError> {
        match preference {
            SurfacePreference::Default | SurfacePreference::Direct2D | SurfacePreference::Cairo => {
                Ok(Win32SurfaceChoice::Hwnd)
            }
            SurfacePreference::Metal => Err(WindowError::UnsupportedConfiguration(
                "Metal is an Apple renderer and cannot target Win32",
            )),
            SurfacePreference::Canvas2D => Err(WindowError::UnsupportedConfiguration(
                "Canvas2D is a browser renderer and cannot target Win32",
            )),
        }
    }

    /// Validate a `window-core` request against Win32 expectations.
    pub fn validate_attributes(
        &self,
        attributes: &WindowAttributes,
    ) -> Result<Win32SurfaceChoice, WindowError> {
        attributes.validate()?;
        if attributes.mount_target != MountTarget::Native {
            return Err(WindowError::UnsupportedConfiguration(
                "Win32 windows must use MountTarget::Native",
            ));
        }
        self.choose_surface(attributes.preferred_surface)
    }

    /// Create a real Win32 window host and optionally wire a paint callback.
    #[cfg(target_os = "windows")]
    pub fn create_native_window(
        &mut self,
        attributes: WindowAttributes,
        on_paint: Option<WindowPaintCallback>,
        user_data: isize,
    ) -> Result<Win32Window, WindowError> {
        self.validate_attributes(&attributes)?;

        let _ = self.choose_surface(attributes.preferred_surface)?;
        self.next_id += 1;

        let id = WindowId(self.next_id);
        let class_name = w!("CodingAdventuresWindowCoreWin32");
        ensure_window_class(class_name)?;
        let size = attributes.initial_size.to_physical(1.0)?;
        let title = wide_null(&attributes.title);
        let instance = unsafe { GetModuleHandleW(None).expect("GetModuleHandleW failed") };
        let hwnd = unsafe {
            CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                size.width.max(1) as i32,
                size.height.max(1) as i32,
                HWND::default(),
                HMENU::default(),
                instance,
                None,
            )
        };
        let hwnd = match hwnd {
            Ok(hwnd) => hwnd,
            Err(error) => {
                return Err(WindowError::backend(format!(
                    "failed to create a Win32 window: {error}"
                )))
            }
        };

        if hwnd.is_invalid() {
            return Err(WindowError::backend(
                "failed to create a Win32 window for the requested attributes",
            ));
        }

        register_paint_handler(hwnd, on_paint, user_data);
        if attributes.visible {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
        }

        Ok(Win32Window {
            id,
            logical_size: attributes.initial_size,
            physical_size: size,
            scale_factor: 1.0,
            hwnd,
        })
    }

    /// Non-Windows compatibility: keep the contract explicit until wired for the host.
    #[cfg(not(target_os = "windows"))]
    pub fn create_native_window(
        &mut self,
        _attributes: WindowAttributes,
        _on_paint: Option<WindowPaintCallback>,
        _user_data: isize,
    ) -> Result<Win32Window, WindowError> {
        Err(WindowError::UnsupportedPlatform(
            "Win32 windows are only available on Windows",
        ))
    }

    /// Run a blocking native Win32 message loop. Returns once `WM_QUIT` is received.
    #[cfg(target_os = "windows")]
    pub fn run(&self) -> Result<(), WindowError> {
        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        loop {
            unsafe {
                let has_message = GetMessageW(&mut msg, HWND::default(), 0, 0);
                if !has_message.as_bool() {
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }
    }

    /// Non-Windows compatibility path for API completeness.
    #[cfg(not(target_os = "windows"))]
    pub fn run(&self) -> Result<(), WindowError> {
        Err(WindowError::UnsupportedPlatform(
            "Win32 message loops are only available on Windows",
        ))
    }
}

#[cfg(target_os = "windows")]
fn ensure_window_class(class_name: PCWSTR) -> Result<(), WindowError> {
    static REGISTERED: OnceLock<bool> = OnceLock::new();
    if REGISTERED.get().is_some() {
        return Ok(());
    }
    let h_instance = unsafe { GetModuleHandleW(None).expect("GetModuleHandleW failed") };
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(win32_window_proc),
        hInstance: h_instance.into(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
        hbrBackground: unsafe { HBRUSH(GetStockObject(WHITE_BRUSH).0) },
        lpszClassName: class_name,
        ..Default::default()
    };
    let already_registered = unsafe { RegisterClassW(&wc) != 0 };
    if !already_registered {
        return Err(WindowError::backend(
            "failed to register Win32 window class",
        ));
    }
    let _ = REGISTERED.set(true);
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_paint_handler(hwnd: HWND, callback: Option<WindowPaintCallback>, user_data: isize) {
    let key = hwnd.0 as isize;
    let mut handlers = paint_handlers().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(callback) = callback {
        handlers.insert(
            key,
            PaintHandler {
                callback: Some(callback),
                user_data,
            },
        );
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn win32_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => LRESULT(0),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            BeginPaint(hwnd, &mut ps);
            invoke_paint_handler(hwnd);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            unregister_paint_handler(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            unregister_paint_handler(hwnd);
            DefWindowProcW(hwnd, msg, wparam, LPARAM(0))
        }
        _ => DefWindowProcW(hwnd, msg, wparam, _lparam),
    }
}

#[cfg(target_os = "windows")]
fn unregister_paint_handler(hwnd: HWND) {
    let key = hwnd.0 as isize;
    let mut handlers = paint_handlers().lock().unwrap_or_else(|e| e.into_inner());
    handlers.remove(&key);
}

#[cfg(target_os = "windows")]
fn invoke_paint_handler(hwnd: HWND) {
    let key = hwnd.0 as isize;
    let handlers = paint_handlers().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(handler) = handlers.get(&key) {
        if let Some(callback) = handler.callback {
            unsafe { callback(hwnd, handler.user_data) };
        }
    }
}

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl WindowBackend for Win32Backend {
    type Window = Win32Window;

    fn backend_name(&self) -> &'static str {
        self.backend_name()
    }

    fn create_window(&mut self, attributes: WindowAttributes) -> Result<Self::Window, WindowError> {
        self.create_native_window(attributes, None, 0)
    }

    fn pump_events(&mut self) -> Result<Vec<window_core::WindowEvent>, WindowError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_direct2d_and_cairo_share_the_hwnd_host() {
        let backend = Win32Backend::new();
        assert_eq!(
            backend.choose_surface(SurfacePreference::Default).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
        assert_eq!(
            backend.choose_surface(SurfacePreference::Direct2D).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
        assert_eq!(
            backend.choose_surface(SurfacePreference::Cairo).unwrap(),
            Win32SurfaceChoice::Hwnd
        );
    }

    #[test]
    fn win32_rejects_browser_mount_targets() {
        let backend = Win32Backend::new();
        let attributes = WindowAttributes {
            mount_target: MountTarget::ElementId("app".to_string()),
            ..WindowAttributes::default()
        };

        let err = backend.validate_attributes(&attributes).unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration("Win32 windows must use MountTarget::Native")
        );
    }

    #[test]
    fn win32_rejects_metal_requests() {
        let backend = Win32Backend::new();
        let err = backend
            .choose_surface(SurfacePreference::Metal)
            .unwrap_err();
        assert_eq!(
            err,
            WindowError::UnsupportedConfiguration(
                "Metal is an Apple renderer and cannot target Win32"
            )
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn create_native_window_is_not_available_off_windows() {
        let mut backend = Win32Backend::new();
        let err = backend
            .create_native_window(
                WindowAttributes {
                    initial_size: LogicalSize::new(640.0, 480.0),
                    preferred_surface: SurfacePreference::Direct2D,
                    ..WindowAttributes::default()
                },
                None,
                0,
            )
            .unwrap_err();

        assert_eq!(
            err,
            WindowError::UnsupportedPlatform("Win32 windows are only available on Windows")
        );
    }
}
