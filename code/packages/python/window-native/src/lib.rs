//! Python native window extension built on `python-bridge`.
//!
//! The first slice stays intentionally small:
//!
//! - create one native window through Rust
//! - track it in a process-local registry
//! - expose handle-based module functions to Python
//! - let the pure-Python wrapper present a friendlier API

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_long, c_ulonglong};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use python_bridge::*;
#[cfg(target_vendor = "apple")]
use window_appkit::{AppKitBackend, AppKitWindow};
use window_core::{
    LogicalSize, MountTarget, RenderTarget, SurfacePreference, Window, WindowAttributes,
    WindowBackend, WindowError,
};

#[cfg(target_vendor = "apple")]
#[derive(Debug)]
enum NativeWindow {
    AppKit(AppKitWindow),
}

#[cfg(not(target_vendor = "apple"))]
#[derive(Debug)]
enum NativeWindow {}

#[allow(non_snake_case)]
extern "C" {
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyLong_AsUnsignedLongLong(obj: PyObjectPtr) -> c_ulonglong;
    fn PyLong_FromUnsignedLongLong(value: c_ulonglong) -> PyObjectPtr;
    fn PyTuple_GetItem(tuple: PyObjectPtr, pos: isize) -> PyObjectPtr;
    fn PyObject_IsTrue(obj: PyObjectPtr) -> c_int;
}

struct WindowRegistry {
    next_handle: u64,
    windows: HashMap<u64, NativeWindow>,
    #[cfg(target_vendor = "apple")]
    backend: AppKitBackend,
}

impl Default for WindowRegistry {
    fn default() -> Self {
        Self {
            next_handle: 0,
            windows: HashMap::new(),
            #[cfg(target_vendor = "apple")]
            backend: AppKitBackend::new(),
        }
    }
}

static WINDOWS: OnceLock<Mutex<WindowRegistry>> = OnceLock::new();
static WINDOW_ERROR_CLASS: OnceLock<usize> = OnceLock::new();

fn registry() -> &'static Mutex<WindowRegistry> {
    WINDOWS.get_or_init(|| Mutex::new(WindowRegistry::default()))
}

unsafe fn window_error_class() -> PyObjectPtr {
    WINDOW_ERROR_CLASS
        .get()
        .map(|value| *value as PyObjectPtr)
        .unwrap_or_else(|| runtime_error_class())
}

unsafe fn set_window_error(error: impl Into<String>) -> PyObjectPtr {
    set_error(window_error_class(), &error.into());
    ptr::null_mut()
}

unsafe fn parse_arg_u64(args: PyObjectPtr, index: isize) -> Option<u64> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    Some(PyLong_AsUnsignedLongLong(arg) as u64)
}

unsafe fn parse_arg_i32(args: PyObjectPtr, index: isize) -> Option<i32> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    Some(PyLong_AsLong(arg) as i32)
}

unsafe fn parse_arg_bool(args: PyObjectPtr, index: isize) -> Option<bool> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    Some(PyObject_IsTrue(arg) != 0)
}

unsafe fn py_u64(value: u64) -> PyObjectPtr {
    PyLong_FromUnsignedLongLong(value as c_ulonglong)
}

unsafe fn tuple2_f64(a: f64, b: f64) -> PyObjectPtr {
    let tuple = PyTuple_New(2);
    PyTuple_SetItem(tuple, 0, f64_to_py(a));
    PyTuple_SetItem(tuple, 1, f64_to_py(b));
    tuple
}

unsafe fn tuple2_u32(a: u32, b: u32) -> PyObjectPtr {
    let tuple = PyTuple_New(2);
    PyTuple_SetItem(tuple, 0, py_u64(a as u64));
    PyTuple_SetItem(tuple, 1, py_u64(b as u64));
    tuple
}

fn map_surface(surface: i32) -> Result<SurfacePreference, WindowError> {
    match surface {
        0 => Ok(SurfacePreference::Default),
        1 => Ok(SurfacePreference::Metal),
        2 => Ok(SurfacePreference::Direct2D),
        3 => Ok(SurfacePreference::Cairo),
        4 => Ok(SurfacePreference::Canvas2D),
        _ => Err(WindowError::InvalidAttributes(
            "preferred_surface must be a known SurfacePreference value",
        )),
    }
}

fn render_target_kind(window: &NativeWindow) -> &'static str {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => match inner.render_target() {
                RenderTarget::AppKit(_) => "appkit",
                RenderTarget::Win32(_) => "win32",
                RenderTarget::BrowserCanvas(_) => "browser-canvas",
                RenderTarget::Wayland(_) => "wayland",
                RenderTarget::X11(_) => "x11",
            },
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_id(window: &NativeWindow) -> u64 {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.id().0,
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_scale_factor(window: &NativeWindow) -> f64 {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.scale_factor(),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_logical_size(window: &NativeWindow) -> LogicalSize {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.logical_size(),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_physical_size(window: &NativeWindow) -> window_core::PhysicalSize {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.physical_size(),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_request_redraw(window: &NativeWindow) -> Result<(), WindowError> {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.request_redraw(),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        match *window {}
    }
}

fn window_set_title(window: &NativeWindow, title: &str) -> Result<(), WindowError> {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.set_title(title),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = title;
        match *window {}
    }
}

fn window_set_visible(window: &NativeWindow, visible: bool) -> Result<(), WindowError> {
    #[cfg(target_vendor = "apple")]
    {
        match window {
            NativeWindow::AppKit(inner) => inner.set_visible(visible),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = visible;
        match *window {}
    }
}

unsafe fn with_window<T>(
    handle: u64,
    f: impl FnOnce(&NativeWindow) -> T,
) -> Result<T, WindowError> {
    let guard = registry()
        .lock()
        .map_err(|_| WindowError::backend("window registry lock was poisoned"))?;
    let window = guard
        .windows
        .get(&handle)
        .ok_or(WindowError::backend("window handle is invalid or already closed"))?;
    Ok(f(window))
}

unsafe fn with_window_result<T>(
    handle: u64,
    f: impl FnOnce(&NativeWindow) -> Result<T, WindowError>,
) -> Result<T, WindowError> {
    let guard = registry()
        .lock()
        .map_err(|_| WindowError::backend("window registry lock was poisoned"))?;
    let window = guard
        .windows
        .get(&handle)
        .ok_or(WindowError::backend("window handle is invalid or already closed"))?;
    f(window)
}

unsafe extern "C" fn py_create_window(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let title = match parse_arg_str(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "_create_window() requires title, width, height, preferred_surface, visible, resizable, decorations, transparent",
            );
            return ptr::null_mut();
        }
    };
    let width = match parse_arg_f64(args, 1) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() width must be numeric");
            return ptr::null_mut();
        }
    };
    let height = match parse_arg_f64(args, 2) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() height must be numeric");
            return ptr::null_mut();
        }
    };
    let initial_size = match LogicalSize::new(width, height).validate() {
        Ok(size) => size,
        Err(error) => return set_window_error(error.to_string()),
    };
    let preferred_surface = match parse_arg_i32(args, 3) {
        Some(value) => match map_surface(value) {
            Ok(surface) => surface,
            Err(error) => return set_window_error(error.to_string()),
        },
        None => {
            set_error(type_error_class(), "_create_window() preferred_surface must be an int");
            return ptr::null_mut();
        }
    };
    let visible = match parse_arg_bool(args, 4) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() visible must be truthy or falsy");
            return ptr::null_mut();
        }
    };
    let resizable = match parse_arg_bool(args, 5) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() resizable must be truthy or falsy");
            return ptr::null_mut();
        }
    };
    let decorations = match parse_arg_bool(args, 6) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() decorations must be truthy or falsy");
            return ptr::null_mut();
        }
    };
    let transparent = match parse_arg_bool(args, 7) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_create_window() transparent must be truthy or falsy");
            return ptr::null_mut();
        }
    };

    let attributes = WindowAttributes {
        title,
        initial_size,
        min_size: None,
        max_size: None,
        visible,
        resizable,
        decorations,
        transparent,
        preferred_surface,
        mount_target: MountTarget::Native,
    };
    if let Err(error) = attributes.validate() {
        return set_window_error(error.to_string());
    }

    let mut guard = match registry().lock() {
        Ok(value) => value,
        Err(_) => {
            return set_window_error("window registry lock was poisoned");
        }
    };

    #[cfg(target_vendor = "apple")]
    {
        match guard.backend.create_window(attributes) {
            Ok(window) => {
                guard.next_handle += 1;
                let handle = guard.next_handle;
                guard.windows.insert(handle, NativeWindow::AppKit(window));
                py_u64(handle)
            }
            Err(error) => set_window_error(error.to_string()),
        }
    }

    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = attributes;
        set_window_error(
            "window-native Python bridge is currently only wired for AppKit on Apple platforms",
        )
    }
}

unsafe extern "C" fn py_close_window(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_close_window() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    let mut guard = match registry().lock() {
        Ok(value) => value,
        Err(_) => return set_window_error("window registry lock was poisoned"),
    };
    guard.windows.remove(&handle);
    py_none()
}

unsafe extern "C" fn py_window_id(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_id() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    match with_window(handle, window_id) {
        Ok(id) => py_u64(id),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_scale_factor(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_scale_factor() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    match with_window(handle, window_scale_factor) {
        Ok(value) => f64_to_py(value),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_logical_size(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_logical_size() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    match with_window(handle, window_logical_size) {
        Ok(size) => tuple2_f64(size.width, size.height),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_physical_size(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_physical_size() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    match with_window(handle, window_physical_size) {
        Ok(size) => tuple2_u32(size.width, size.height),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_request_redraw(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_request_redraw() requires a numeric handle");
            return ptr::null_mut();
        }
    };
    match with_window_result(handle, window_request_redraw) {
        Ok(()) => py_none(),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_set_title(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_set_title() requires handle and title");
            return ptr::null_mut();
        }
    };
    let title = match parse_arg_str(args, 1) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_set_title() title must be a string");
            return ptr::null_mut();
        }
    };
    match with_window_result(handle, |window| window_set_title(window, &title)) {
        Ok(()) => py_none(),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_set_visible(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_set_visible() requires handle and visible");
            return ptr::null_mut();
        }
    };
    let visible = match parse_arg_bool(args, 1) {
        Some(value) => value,
        None => {
            set_error(type_error_class(), "_window_set_visible() visible must be truthy or falsy");
            return ptr::null_mut();
        }
    };
    match with_window_result(handle, |window| window_set_visible(window, visible)) {
        Ok(()) => py_none(),
        Err(error) => set_window_error(error.to_string()),
    }
}

unsafe extern "C" fn py_window_render_target_kind(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let handle = match parse_arg_u64(args, 0) {
        Some(value) => value,
        None => {
            set_error(
                type_error_class(),
                "_window_render_target_kind() requires a numeric handle",
            );
            return ptr::null_mut();
        }
    };
    match with_window(handle, render_target_kind) {
        Ok(kind) => str_to_py(kind),
        Err(error) => set_window_error(error.to_string()),
    }
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_window_native() -> PyObjectPtr {
    let methods: &'static mut [PyMethodDef; 11] = Box::leak(Box::new([
        PyMethodDef {
            ml_name: b"_create_window\0".as_ptr() as *const c_char,
            ml_meth: Some(py_create_window),
            ml_flags: METH_VARARGS,
            ml_doc: b"Create a native window and return its internal handle.\0".as_ptr()
                as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_close_window\0".as_ptr() as *const c_char,
            ml_meth: Some(py_close_window),
            ml_flags: METH_VARARGS,
            ml_doc: b"Close a tracked native window handle.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_id\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_id),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return the stable window identifier.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_scale_factor\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_scale_factor),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return the window scale factor.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_logical_size\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_logical_size),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return the logical size tuple.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_physical_size\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_physical_size),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return the physical size tuple.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_request_redraw\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_request_redraw),
            ml_flags: METH_VARARGS,
            ml_doc: b"Request a redraw for one tracked window.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_set_title\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_set_title),
            ml_flags: METH_VARARGS,
            ml_doc: b"Set a window title.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_set_visible\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_set_visible),
            ml_flags: METH_VARARGS,
            ml_doc: b"Show or hide a window.\0".as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"_window_render_target_kind\0".as_ptr() as *const c_char,
            ml_meth: Some(py_window_render_target_kind),
            ml_flags: METH_VARARGS,
            ml_doc: b"Return the render target kind string.\0".as_ptr() as *const c_char,
        },
        method_def_sentinel(),
    ]));

    let def: &'static mut PyModuleDef = Box::leak(Box::new(PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0u8; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: b"window_native\0".as_ptr() as *const c_char,
        m_doc: b"Python native window bridge backed by Rust window-core/window-appkit.\0"
            .as_ptr() as *const c_char,
        m_size: -1,
        m_methods: methods.as_mut_ptr(),
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    }));

    let module = PyModule_Create2(def as *mut PyModuleDef, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    let window_error = new_exception("window_native", "WindowError", exception_class());
    if window_error.is_null() {
        return ptr::null_mut();
    }
    Py_IncRef(window_error);
    let _ = WINDOW_ERROR_CLASS.set(window_error as usize);
    module_add_object(module, "WindowError", window_error);
    module
}
