//! # window-c
//!
//! C ABI wrapper over the shared native windowing contract.
//!
//! This crate exists for language ecosystems that already consume repository
//! owned C shims. The wrapper stays intentionally narrow:
//!
//! - accept plain C structs that mirror `window-core`
//! - create one native window on the current platform
//! - expose an opaque handle plus query/setter helpers
//! - report failures through a thread-local last-error string

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};

use window_appkit::AppKitBackend;
use window_core::{
    LogicalSize, MountTarget, PhysicalSize, RenderTarget, SurfacePreference, Window,
    WindowAttributes, WindowBackend, WindowError,
};

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("").expect("empty CString"));
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct window_c_logical_size_t {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct window_c_physical_size_t {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum window_c_surface_preference_t {
    WINDOW_C_SURFACE_DEFAULT = 0,
    WINDOW_C_SURFACE_METAL = 1,
    WINDOW_C_SURFACE_DIRECT2D = 2,
    WINDOW_C_SURFACE_CAIRO = 3,
    WINDOW_C_SURFACE_CANVAS2D = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum window_c_mount_target_kind_t {
    WINDOW_C_MOUNT_NATIVE = 0,
    WINDOW_C_MOUNT_BROWSER_BODY = 1,
    WINDOW_C_MOUNT_ELEMENT_ID = 2,
    WINDOW_C_MOUNT_QUERY_SELECTOR = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct window_c_mount_target_t {
    pub kind: u32,
    pub value: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct window_c_window_attributes_t {
    pub title: *const c_char,
    pub initial_size: window_c_logical_size_t,
    pub has_min_size: u8,
    pub min_size: window_c_logical_size_t,
    pub has_max_size: u8,
    pub max_size: window_c_logical_size_t,
    pub visible: u8,
    pub resizable: u8,
    pub decorations: u8,
    pub transparent: u8,
    pub preferred_surface: u32,
    pub mount_target: window_c_mount_target_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum window_c_render_target_kind_t {
    WINDOW_C_RENDER_TARGET_NONE = 0,
    WINDOW_C_RENDER_TARGET_APPKIT = 1,
    WINDOW_C_RENDER_TARGET_WIN32 = 2,
    WINDOW_C_RENDER_TARGET_BROWSER_CANVAS = 3,
    WINDOW_C_RENDER_TARGET_WAYLAND = 4,
    WINDOW_C_RENDER_TARGET_X11 = 5,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct window_c_appkit_render_target_t {
    pub ns_window: usize,
    pub ns_view: usize,
    pub metal_layer: usize,
    pub has_metal_layer: u8,
}

enum NativeWindow {
    AppKit(window_appkit::AppKitWindow),
}

#[repr(C)]
pub struct window_c_window_t {
    inner: NativeWindow,
}

#[no_mangle]
pub extern "C" fn window_c_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn window_c_is_appkit_available() -> u8 {
    #[cfg(target_vendor = "apple")]
    {
        1
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn window_c_create_window(
    attributes: *const window_c_window_attributes_t,
) -> *mut window_c_window_t {
    let Some(attributes) = attributes.as_ref() else {
        return fail_null("window_c_create_window requires a non-null attributes pointer");
    };

    let attributes = match convert_attributes(attributes) {
        Ok(value) => value,
        Err(error) => return fail_error(error),
    };

    #[cfg(target_vendor = "apple")]
    {
      let mut backend = AppKitBackend::new();
      match backend.create_window(attributes) {
          Ok(window) => {
              clear_last_error();
              Box::into_raw(Box::new(window_c_window_t {
                  inner: NativeWindow::AppKit(window),
              }))
          }
          Err(error) => fail_error(error),
      }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = attributes;
        fail_error(WindowError::UnsupportedPlatform(
            "window-c native window creation is currently only wired for AppKit",
        ))
    }
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_free(window: *mut window_c_window_t) {
    if window.is_null() {
        return;
    }
    drop(Box::from_raw(window));
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_id(window: *const window_c_window_t) -> u64 {
    with_window(window, |window| window.id().0).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_scale_factor(
    window: *const window_c_window_t,
) -> f64 {
    with_window(window, |window| window.scale_factor()).unwrap_or(0.0)
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_logical_size(
    window: *const window_c_window_t,
    out_size: *mut window_c_logical_size_t,
) -> u8 {
    if out_size.is_null() {
        return fail_bool("window_c_window_logical_size requires a non-null output pointer");
    }

    match with_window(window, |window| window.logical_size()) {
        Some(size) => {
            *out_size = window_c_logical_size_t::from(size);
            clear_last_error();
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_physical_size(
    window: *const window_c_window_t,
    out_size: *mut window_c_physical_size_t,
) -> u8 {
    if out_size.is_null() {
        return fail_bool("window_c_window_physical_size requires a non-null output pointer");
    }

    match with_window(window, |window| window.physical_size()) {
        Some(size) => {
            *out_size = window_c_physical_size_t::from(size);
            clear_last_error();
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_request_redraw(
    window: *const window_c_window_t,
) -> u8 {
    call_window(window, |window| window.request_redraw())
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_set_title(
    window: *const window_c_window_t,
    title: *const c_char,
) -> u8 {
    let title = match c_string(title, true) {
        Ok(Some(value)) => value,
        Ok(None) => "Coding Adventures Window".to_string(),
        Err(error) => return fail_bool_error(error),
    };
    call_window(window, |window| window.set_title(&title))
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_set_visible(
    window: *const window_c_window_t,
    visible: u8,
) -> u8 {
    call_window(window, |window| window.set_visible(visible != 0))
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_render_target_kind(
    window: *const window_c_window_t,
) -> window_c_render_target_kind_t {
    match with_window(window, |window| window.render_target()) {
        Some(RenderTarget::AppKit(_)) => window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_APPKIT,
        Some(RenderTarget::Win32(_)) => window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_WIN32,
        Some(RenderTarget::BrowserCanvas(_)) => {
            window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_BROWSER_CANVAS
        }
        Some(RenderTarget::Wayland(_)) => {
            window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_WAYLAND
        }
        Some(RenderTarget::X11(_)) => window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_X11,
        None => window_c_render_target_kind_t::WINDOW_C_RENDER_TARGET_NONE,
    }
}

#[no_mangle]
pub unsafe extern "C" fn window_c_window_render_target_appkit(
    window: *const window_c_window_t,
    out_target: *mut window_c_appkit_render_target_t,
) -> u8 {
    if out_target.is_null() {
        return fail_bool(
            "window_c_window_render_target_appkit requires a non-null output pointer",
        );
    }

    match with_window(window, |window| window.render_target()) {
        Some(RenderTarget::AppKit(target)) => {
            *out_target = window_c_appkit_render_target_t {
                ns_window: target.ns_window,
                ns_view: target.ns_view,
                metal_layer: target.metal_layer.unwrap_or(0),
                has_metal_layer: u8::from(target.metal_layer.is_some()),
            };
            clear_last_error();
            1
        }
        Some(_) => fail_bool("window does not expose an AppKit render target"),
        None => 0,
    }
}

fn convert_attributes(
    attributes: &window_c_window_attributes_t,
) -> Result<WindowAttributes, WindowError> {
    Ok(WindowAttributes {
        title: c_string(attributes.title, false)?.unwrap_or_else(|| {
            "Coding Adventures Window".to_string()
        }),
        initial_size: LogicalSize::from(attributes.initial_size),
        min_size: if attributes.has_min_size != 0 {
            Some(LogicalSize::from(attributes.min_size))
        } else {
            None
        },
        max_size: if attributes.has_max_size != 0 {
            Some(LogicalSize::from(attributes.max_size))
        } else {
            None
        },
        visible: attributes.visible != 0,
        resizable: attributes.resizable != 0,
        decorations: attributes.decorations != 0,
        transparent: attributes.transparent != 0,
        preferred_surface: map_surface(attributes.preferred_surface)?,
        mount_target: convert_mount_target(attributes.mount_target)?,
    })
}

fn convert_mount_target(
    target: window_c_mount_target_t,
) -> Result<MountTarget, WindowError> {
    match target.kind {
        value if value == window_c_mount_target_kind_t::WINDOW_C_MOUNT_NATIVE as u32 => {
            Ok(MountTarget::Native)
        }
        value if value == window_c_mount_target_kind_t::WINDOW_C_MOUNT_BROWSER_BODY as u32 => {
            Ok(MountTarget::BrowserBody)
        }
        value if value == window_c_mount_target_kind_t::WINDOW_C_MOUNT_ELEMENT_ID as u32 => {
            Ok(MountTarget::ElementId(
                c_string(target.value, true)?.unwrap_or_default(),
            ))
        }
        value if value == window_c_mount_target_kind_t::WINDOW_C_MOUNT_QUERY_SELECTOR as u32 => {
            Ok(MountTarget::QuerySelector(
                c_string(target.value, true)?.unwrap_or_default(),
            ))
        }
        _ => Err(WindowError::InvalidAttributes(
            "mount target kind must be a known window-c constant",
        )),
    }
}

fn map_surface(preference: u32) -> Result<SurfacePreference, WindowError> {
    match preference {
        value if value == window_c_surface_preference_t::WINDOW_C_SURFACE_DEFAULT as u32 => {
            Ok(SurfacePreference::Default)
        }
        value if value == window_c_surface_preference_t::WINDOW_C_SURFACE_METAL as u32 => {
            Ok(SurfacePreference::Metal)
        }
        value if value == window_c_surface_preference_t::WINDOW_C_SURFACE_DIRECT2D as u32 => {
            Ok(SurfacePreference::Direct2D)
        }
        value if value == window_c_surface_preference_t::WINDOW_C_SURFACE_CAIRO as u32 => {
            Ok(SurfacePreference::Cairo)
        }
        value if value == window_c_surface_preference_t::WINDOW_C_SURFACE_CANVAS2D as u32 => {
            Ok(SurfacePreference::Canvas2D)
        }
        _ => Err(WindowError::InvalidAttributes(
            "surface preference must be a known window-c constant",
        )),
    }
}

fn c_string(pointer: *const c_char, required: bool) -> Result<Option<String>, WindowError> {
    if pointer.is_null() {
        return if required {
            Err(WindowError::InvalidAttributes(
                "required C strings must not be null",
            ))
        } else {
            Ok(None)
        };
    }

    let value = unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map_err(|_| WindowError::InvalidAttributes("C strings must be valid UTF-8"))?;
    Ok(Some(value.to_string()))
}

unsafe fn with_window<T>(
    window: *const window_c_window_t,
    callback: impl FnOnce(&dyn Window) -> T,
) -> Option<T> {
    let Some(window) = window.as_ref() else {
        set_last_error("window pointer must not be null");
        return None;
    };

    let value = match &window.inner {
        NativeWindow::AppKit(inner) => callback(inner),
    };
    clear_last_error();
    Some(value)
}

unsafe fn call_window(
    window: *const window_c_window_t,
    callback: impl FnOnce(&dyn Window) -> Result<(), WindowError>,
) -> u8 {
    let Some(window) = window.as_ref() else {
        return fail_bool("window pointer must not be null");
    };

    let result = match &window.inner {
        NativeWindow::AppKit(inner) => callback(inner),
    };

    match result {
        Ok(()) => {
            clear_last_error();
            1
        }
        Err(error) => fail_bool_error(error),
    }
}

fn fail_null(message: impl Into<String>) -> *mut window_c_window_t {
    set_last_error(message);
    std::ptr::null_mut()
}

fn fail_error(error: WindowError) -> *mut window_c_window_t {
    set_last_error(error.to_string());
    std::ptr::null_mut()
}

fn fail_bool(message: impl Into<String>) -> u8 {
    set_last_error(message);
    0
}

fn fail_bool_error(error: WindowError) -> u8 {
    set_last_error(error.to_string());
    0
}

fn set_last_error(message: impl Into<String>) {
    let message = message.into().replace('\0', " ");
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = CString::new(message)
            .unwrap_or_else(|_| CString::new("window-c internal error").expect("valid CString"));
    });
}

fn clear_last_error() {
    set_last_error("");
}

impl From<window_c_logical_size_t> for LogicalSize {
    fn from(value: window_c_logical_size_t) -> Self {
        LogicalSize::new(value.width, value.height)
    }
}

impl From<LogicalSize> for window_c_logical_size_t {
    fn from(value: LogicalSize) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<PhysicalSize> for window_c_physical_size_t {
    fn from(value: PhysicalSize) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_attributes_preserves_bounds_and_flags() {
        let title = CString::new("FFI Window").unwrap();
        let selector = CString::new("#app").unwrap();
        let converted = convert_attributes(&window_c_window_attributes_t {
            title: title.as_ptr(),
            initial_size: window_c_logical_size_t {
                width: 640.0,
                height: 480.0,
            },
            has_min_size: 1,
            min_size: window_c_logical_size_t {
                width: 320.0,
                height: 240.0,
            },
            has_max_size: 1,
            max_size: window_c_logical_size_t {
                width: 1024.0,
                height: 768.0,
            },
            visible: 1,
            resizable: 0,
            decorations: 1,
            transparent: 0,
            preferred_surface: window_c_surface_preference_t::WINDOW_C_SURFACE_CANVAS2D as u32,
            mount_target: window_c_mount_target_t {
                kind: window_c_mount_target_kind_t::WINDOW_C_MOUNT_QUERY_SELECTOR as u32,
                value: selector.as_ptr(),
            },
        })
        .unwrap();

        assert_eq!(converted.title, "FFI Window");
        assert_eq!(converted.initial_size, LogicalSize::new(640.0, 480.0));
        assert_eq!(converted.min_size, Some(LogicalSize::new(320.0, 240.0)));
        assert_eq!(converted.max_size, Some(LogicalSize::new(1024.0, 768.0)));
        assert!(!converted.resizable);
        assert_eq!(converted.preferred_surface, SurfacePreference::Canvas2D);
        assert_eq!(
            converted.mount_target,
            MountTarget::QuerySelector("#app".to_string())
        );
    }

    #[test]
    fn element_and_selector_mount_targets_require_utf8() {
        let bytes = [0xff_u8, 0];
        let error = c_string(bytes.as_ptr() as *const c_char, true).unwrap_err();
        assert_eq!(
            error,
            WindowError::InvalidAttributes("C strings must be valid UTF-8")
        );
    }

    #[test]
    fn null_required_strings_are_rejected() {
        let error = c_string(std::ptr::null(), true).unwrap_err();
        assert_eq!(
            error,
            WindowError::InvalidAttributes("required C strings must not be null")
        );
    }

    #[test]
    fn invalid_surface_constants_are_rejected() {
        let error = map_surface(99).unwrap_err();
        assert_eq!(
            error,
            WindowError::InvalidAttributes(
                "surface preference must be a known window-c constant"
            )
        );
    }

    #[test]
    fn invalid_mount_target_constants_are_rejected() {
        let error = convert_mount_target(window_c_mount_target_t {
            kind: 77,
            value: std::ptr::null(),
        })
        .unwrap_err();
        assert_eq!(
            error,
            WindowError::InvalidAttributes(
                "mount target kind must be a known window-c constant"
            )
        );
    }
}
