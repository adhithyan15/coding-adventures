// conduit_native — Python C extension for the Conduit web framework.
// (crate-level attribute must come first; doc comment follows)
#![allow(non_snake_case)] // Python C API uses PascalCase names

/// conduit_native — Python C extension for the Conduit web framework.
///
/// This crate is the Python mirror of the Ruby `conduit_native` (WEB02).
/// It exposes the `web-core` HTTP engine to Python using the same hook
/// protocol: before/after filters, not_found, and error handlers are
/// registered at init time; Rust dispatches them to Python by calling
/// methods on the `NativeServer` Python object (the `owner`).
///
/// ## GIL management
///
/// Python's GIL is analogous to Ruby's GVL. The same discipline applies:
/// - `server_serve` releases the GIL before blocking (`PyEval_SaveThread`)
///   and restores it on return (`PyEval_RestoreThread`).
/// - Every Python callout acquires the GIL from the Rust web-core thread
///   (`PyGILState_Ensure`) and releases it after (`PyGILState_Release`).
///
/// ## Protocol
///
/// Python dispatch methods return either:
///   - `None`       → no short-circuit; Rust continues
///   - `[s, h, b]` → use this response (s=int, h=[[name,val],...], b=str)
///
/// `HaltException` is caught by the Python dispatch methods before returning
/// to Rust. Rust never sees Python exceptions directly.
///
/// ## Module API
///
/// ```python
/// import conduit_native
/// capsule = conduit_native.server_new(owner, app, host, port, max_connections)
/// conduit_native.server_serve(capsule)
/// conduit_native.server_stop(capsule)
/// conduit_native.server_running(capsule)    # → bool
/// conduit_native.server_local_host(capsule) # → str
/// conduit_native.server_local_port(capsule) # → int
/// conduit_native.server_dispose(capsule)
/// ```

use std::ffi::{c_char, c_long, c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use http_core::Header;
use python_bridge::{
    PyMethodDef, PyModuleDef, PyModuleDef_Base, PyObjectPtr,
    METH_VARARGS, PYTHON_API_VERSION,
    Py_DecRef, Py_IncRef, PyModule_Create2,
    PyLong_FromLong, PyDict_New, PyDict_SetItem,
    PyList_New, PyList_SetItem, PyList_GetItem, PyList_Size,
    PyTuple_GetItem, PyTuple_New, PyTuple_SetItem,
    PyErr_SetString, PyErr_Clear,
    PyObject_GetAttrString,
    str_to_py, str_from_py, py_none, py_true, py_false,
    runtime_error_class,
    method_def_sentinel,
};
use web_core::{WebApp, WebRequest, WebResponse, WebServer};

// ─────────────────────────────────────────────────────────────────────────────
// GIL management — not in python-bridge; declared inline (stable C API)
// ─────────────────────────────────────────────────────────────────────────────
//
// Python's thread-state machinery:
//
//   PyEval_SaveThread   → release the GIL; returns a *mut PyThreadState.
//   PyEval_RestoreThread→ re-acquire the GIL from a saved thread state.
//   PyGILState_Ensure   → acquire the GIL from ANY OS thread.
//   PyGILState_Release  → release what PyGILState_Ensure acquired.
//
// We use Save/Restore around server_serve() (the long-running blocking call)
// and Ensure/Release around each Python dispatch call from web-core threads.

#[allow(non_snake_case)]
extern "C" {
    fn PyEval_SaveThread() -> *mut c_void;
    fn PyEval_RestoreThread(state: *mut c_void);
    fn PyGILState_Ensure() -> i32;
    fn PyGILState_Release(state: i32);

    // Call a callable with a tuple of arguments (borrowed refs for both).
    // Returns a new reference, or NULL on error.
    fn PyObject_CallObject(callable: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr;

    // PyCapsule — wraps a raw C pointer with a destructor.
    fn PyCapsule_New(
        pointer: *mut c_void,
        name: *const c_char,
        destructor: Option<unsafe extern "C" fn(PyObjectPtr)>,
    ) -> PyObjectPtr;
    fn PyCapsule_GetPointer(capsule: PyObjectPtr, name: *const c_char) -> *mut c_void;

    // Extract C long from Python int. Returns -1 on error.
    fn PyLong_AsLong(o: PyObjectPtr) -> c_long;

    // Non-null if a Python exception is currently set.
    fn PyErr_Occurred() -> PyObjectPtr;

    // Fetch the active exception into three out-params (type, value, traceback).
    fn PyErr_Fetch(
        ptype: *mut PyObjectPtr,
        pvalue: *mut PyObjectPtr,
        ptb: *mut PyObjectPtr,
    );

    // Normalize a (type, value, tb) triple into the canonical instance form.
    fn PyErr_NormalizeException(
        ptype: *mut PyObjectPtr,
        pvalue: *mut PyObjectPtr,
        ptb: *mut PyObjectPtr,
    );

    // Borrow the UTF-8 bytes from a Python str (no allocation; lifetime = object).
    fn PyUnicode_AsUTF8(o: PyObjectPtr) -> *const c_char;

    // str(obj) → Python str (new reference).
    fn PyObject_Str(obj: PyObjectPtr) -> PyObjectPtr;
}

// ─────────────────────────────────────────────────────────────────────────────
// Platform alias
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PlatformWebServer = WebServer<transport_platform::bsd::KqueueTransportPlatform>;

#[cfg(target_os = "linux")]
type PlatformWebServer = WebServer<transport_platform::linux::EpollTransportPlatform>;

#[cfg(target_os = "windows")]
type PlatformWebServer = WebServer<transport_platform::windows::WindowsTransportPlatform>;

// ─────────────────────────────────────────────────────────────────────────────
// PyOwner — Arc-wrapped reference to the NativeServer Python object
// ─────────────────────────────────────────────────────────────────────────────
//
// `PyOwner` holds a strong reference (via Py_IncRef at creation) to the Python
// `NativeServer` instance. Rust dispatch closures clone the `Arc<PyOwner>` so
// each closure can independently call Python methods.
//
// `Send + Sync` is safe here: we only dereference the pointer while holding
// the GIL (via PyGILState_Ensure before every Python call).

struct PyOwner(PyObjectPtr);

unsafe impl Send for PyOwner {}
unsafe impl Sync for PyOwner {}

impl Drop for PyOwner {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // Must hold GIL to decref a Python object.
            let gil = unsafe { PyGILState_Ensure() };
            unsafe { Py_DecRef(self.0) };
            unsafe { PyGILState_Release(gil) };
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Server state — owned by a PyCapsule
// ─────────────────────────────────────────────────────────────────────────────

struct PyConduitServer {
    server: Option<PlatformWebServer>,
    #[allow(dead_code)] // kept alive so PyOwner::drop runs at capsule destruction
    owner: Arc<PyOwner>,
    running: Arc<AtomicBool>,
}

const CAPSULE_NAME: &[u8] = b"conduit_native.server\0";

unsafe extern "C" fn capsule_destructor(capsule: PyObjectPtr) {
    let ptr = PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char);
    if !ptr.is_null() {
        // Reconstructing the Box triggers PyOwner::drop (calls Py_DecRef on owner)
        // and PlatformWebServer::drop (joins worker threads). Both are safe from
        // the destructor context since Python holds the GIL here.
        drop(Box::from_raw(ptr as *mut PyConduitServer));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: call a method on `owner` with N arguments
// ─────────────────────────────────────────────────────────────────────────────
//
// Builds a Python tuple of the given args, calls `owner.method_name(*args)`,
// and returns the result (new reference) or NULL on error.
//
// Reference counting:
//   - `owner` is borrowed (not stolen); we do not Py_DecRef it.
//   - Each arg in `args_owned` is owned by the caller and transferred to the
//     tuple via PyTuple_SetItem (which steals references). Callers must NOT
//     Py_DecRef the args after calling this function.
//   - The result is a new reference; callers must Py_DecRef it when done.

unsafe fn call_owner_method(
    owner: PyObjectPtr,
    method_cstr: &CStr,
    args_owned: &[PyObjectPtr],
) -> PyObjectPtr {
    let method_attr = PyObject_GetAttrString(owner, method_cstr.as_ptr());
    if method_attr.is_null() {
        // Clean up owned args since we can't call the method.
        for &arg in args_owned {
            Py_DecRef(arg);
        }
        return ptr::null_mut();
    }

    let tuple = PyTuple_New(args_owned.len() as isize);
    for (i, &arg) in args_owned.iter().enumerate() {
        // PyTuple_SetItem steals the reference to `arg`, so we must NOT
        // Py_DecRef the elements individually — the tuple owns them now.
        PyTuple_SetItem(tuple, i as isize, arg);
    }

    let result = PyObject_CallObject(method_attr, tuple);
    Py_DecRef(method_attr);
    Py_DecRef(tuple); // also decrefs each contained item (which were stolen above)
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// server_new(owner, app, host, port, max_connections) → capsule
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_new(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let owner_py   = PyTuple_GetItem(args, 0);
    let app_py     = PyTuple_GetItem(args, 1);
    let host_py    = PyTuple_GetItem(args, 2);
    let port_py    = PyTuple_GetItem(args, 3);
    let max_py     = PyTuple_GetItem(args, 4);

    if [owner_py, app_py, host_py, port_py, max_py].iter().any(|p| p.is_null()) {
        PyErr_SetString(runtime_error_class(), c"server_new requires (owner, app, host, port, max_connections)".as_ptr());
        return ptr::null_mut();
    }

    let host = match str_from_py(host_py) {
        Some(s) => s,
        None => {
            PyErr_SetString(runtime_error_class(), c"host must be a string".as_ptr());
            return ptr::null_mut();
        }
    };

    let port_raw = PyLong_AsLong(port_py);
    if port_raw < 0 || port_raw > 65535 {
        if PyErr_Occurred().is_null() {
            PyErr_SetString(runtime_error_class(), c"port must be 0–65535".as_ptr());
        }
        return ptr::null_mut();
    }
    let port = port_raw as u16;

    let max_raw = PyLong_AsLong(max_py);
    if max_raw < 0 {
        if PyErr_Occurred().is_null() {
            PyErr_SetString(runtime_error_class(), c"max_connections must be non-negative".as_ptr());
        }
        return ptr::null_mut();
    }
    let max_connections = max_raw as usize;

    // Hold a strong reference to the NativeServer Python object so Rust
    // closures can call back into it.
    Py_IncRef(owner_py);
    let owner = Arc::new(PyOwner(owner_py));

    // --- Route registration ---

    let routes_py = PyObject_GetAttrString(app_py, c"routes".as_ptr());
    if routes_py.is_null() {
        PyErr_SetString(runtime_error_class(), c"app must have a routes attribute".as_ptr());
        return ptr::null_mut();
    }
    let route_count = PyList_Size(routes_py);
    if route_count < 0 {
        Py_DecRef(routes_py);
        PyErr_SetString(runtime_error_class(), c"app.routes must be a list".as_ptr());
        return ptr::null_mut();
    }

    let mut web_app = WebApp::new();

    for i in 0..route_count {
        let route = PyList_GetItem(routes_py, i); // borrowed

        let method_attr = PyObject_GetAttrString(route, c"method".as_ptr());
        let method = str_from_py(method_attr);
        if !method_attr.is_null() { Py_DecRef(method_attr); }
        let method = match method {
            Some(m) => m,
            None => {
                Py_DecRef(routes_py);
                PyErr_SetString(runtime_error_class(), c"route.method must be a string".as_ptr());
                return ptr::null_mut();
            }
        };

        let pattern_attr = PyObject_GetAttrString(route, c"pattern".as_ptr());
        let pattern = str_from_py(pattern_attr);
        if !pattern_attr.is_null() { Py_DecRef(pattern_attr); }
        let pattern = match pattern {
            Some(p) => p,
            None => {
                Py_DecRef(routes_py);
                PyErr_SetString(runtime_error_class(), c"route.pattern must be a string".as_ptr());
                return ptr::null_mut();
            }
        };

        let owner_cap = Arc::clone(&owner);
        let route_index = i as usize;
        web_app.add(&method, &pattern, move |req: &WebRequest| {
            dispatch_route_to_python(&owner_cap, route_index, req)
        });
    }
    Py_DecRef(routes_py);

    // --- Hook registration ---

    // Before-routing fires for every request before route lookup — including
    // unmatched paths. This matches Sinatra semantics; before filters act as
    // middleware that run even when no route exists (e.g. maintenance mode).
    let before_py = PyObject_GetAttrString(app_py, c"before_filters".as_ptr());
    let has_before = !before_py.is_null() && {
        let n = PyList_Size(before_py);
        Py_DecRef(before_py);
        PyErr_Clear(); // clear any error from PyList_Size on non-list
        n > 0
    };
    if has_before {
        let owner_cap = Arc::clone(&owner);
        web_app.before_routing(move |req: &WebRequest| -> Option<WebResponse> {
            dispatch_before_to_python(&owner_cap, req)
        });
    }

    // After-handler fires after the matched route handler — for side effects.
    let after_py = PyObject_GetAttrString(app_py, c"after_filters".as_ptr());
    let has_after = !after_py.is_null() && {
        let n = PyList_Size(after_py);
        Py_DecRef(after_py);
        PyErr_Clear();
        n > 0
    };
    if has_after {
        let owner_cap = Arc::clone(&owner);
        web_app.after_handler(move |req: &WebRequest, resp: WebResponse| -> WebResponse {
            dispatch_after_to_python(&owner_cap, req, resp)
        });
    }

    // Not-found hook: registered only when a custom handler is set.
    let not_found_py = PyObject_GetAttrString(app_py, c"not_found_handler".as_ptr());
    let has_not_found = !not_found_py.is_null() && {
        let is_none = is_py_none(not_found_py);
        Py_DecRef(not_found_py);
        !is_none
    };
    if has_not_found {
        let owner_cap = Arc::clone(&owner);
        web_app.on_not_found(move |req: &WebRequest| -> WebResponse {
            dispatch_not_found_to_python(&owner_cap, req)
        });
    }

    // Error handling: Python exceptions from route handlers are caught inside
    // dispatch_route_to_python while already holding the GIL. There is no
    // on_handler_error hook registration needed (that hook is for Rust panics).

    // --- Bind server ---

    let running = Arc::new(AtomicBool::new(false));
    let mut options = embeddable_http_server::HttpServerOptions::default();
    options.tcp.max_connections = max_connections;

    let server = match bind_server(&host, port, options, Arc::new(web_app)) {
        Ok(s) => s,
        Err(e) => {
            let msg = CString::new(format!("failed to start Conduit server: {e}"))
                .unwrap_or_else(|_| CString::new("server bind failed").unwrap());
            PyErr_SetString(runtime_error_class(), msg.as_ptr());
            return ptr::null_mut();
        }
    };

    let state = Box::new(PyConduitServer { server: Some(server), owner, running });
    PyCapsule_New(
        Box::into_raw(state) as *mut c_void,
        CAPSULE_NAME.as_ptr() as *const c_char,
        Some(capsule_destructor),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// server_serve / stop / dispose / running / local_host / local_port
// ─────────────────────────────────────────────────────────────────────────────

unsafe extern "C" fn server_serve(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_serve(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }

    let server_ptr = match (*state).server.as_mut() {
        Some(s) => s as *mut PlatformWebServer,
        None => {
            PyErr_SetString(runtime_error_class(), c"server is closed".as_ptr());
            return ptr::null_mut();
        }
    };
    let running = Arc::clone(&(*state).running);

    // Release GIL before blocking so other Python threads (Ctrl-C handler, etc.)
    // can run. web-core's serve() uses kqueue/epoll internally.
    let thread_state = PyEval_SaveThread();
    running.store(true, Ordering::SeqCst);
    let result = (*server_ptr).serve();
    running.store(false, Ordering::SeqCst);
    PyEval_RestoreThread(thread_state);

    match result {
        Ok(()) => py_none(),
        Err(e) => {
            let msg = CString::new(format!("Conduit serve error: {e}"))
                .unwrap_or_else(|_| CString::new("server error").unwrap());
            PyErr_SetString(runtime_error_class(), msg.as_ptr());
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn server_stop(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_stop(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }
    match (*state).server.as_ref() {
        Some(s) => { s.stop_handle().stop(); py_none() }
        None => {
            PyErr_SetString(runtime_error_class(), c"server is closed".as_ptr());
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn server_dispose(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_dispose(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }
    if (*state).running.load(Ordering::SeqCst) {
        PyErr_SetString(runtime_error_class(), c"cannot dispose a running server; stop first".as_ptr());
        return ptr::null_mut();
    }
    (*state).server.take();
    py_none()
}

unsafe extern "C" fn server_running(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_running(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }
    if (*state).running.load(Ordering::SeqCst) { py_true() } else { py_false() }
}

unsafe extern "C" fn server_local_host(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_local_host(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }
    match (*state).server.as_ref() {
        Some(s) => str_to_py(&s.local_addr().ip().to_string()),
        None => {
            PyErr_SetString(runtime_error_class(), c"server is closed".as_ptr());
            ptr::null_mut()
        }
    }
}

unsafe extern "C" fn server_local_port(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let capsule = PyTuple_GetItem(args, 0);
    if capsule.is_null() {
        PyErr_SetString(runtime_error_class(), c"server_local_port(capsule)".as_ptr());
        return ptr::null_mut();
    }
    let state = get_state(capsule);
    if state.is_null() { return ptr::null_mut(); }
    match (*state).server.as_ref() {
        Some(s) => PyLong_FromLong(s.local_addr().port() as c_long),
        None => {
            PyErr_SetString(runtime_error_class(), c"server is closed".as_ptr());
            ptr::null_mut()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dispatch functions — called from web-core threads
// ─────────────────────────────────────────────────────────────────────────────

fn dispatch_route_to_python(
    owner: &Arc<PyOwner>,
    route_index: usize,
    req: &WebRequest,
) -> WebResponse {
    let gil = unsafe { PyGILState_Ensure() };
    let result = unsafe { dispatch_route_in_gil(owner.0, route_index, req) };
    unsafe { PyGILState_Release(gil) };
    result
}

unsafe fn dispatch_route_in_gil(
    owner: PyObjectPtr,
    route_index: usize,
    req: &WebRequest,
) -> WebResponse {
    // build_env() and PyLong_FromLong() return new references (owned by us).
    // call_owner_method() steals them (they become owned by the tuple).
    let env     = build_env(req);
    let idx_py  = PyLong_FromLong(route_index as c_long);

    let result = call_owner_method(owner, c"native_dispatch_route", &[idx_py, env]);

    if result.is_null() {
        // Python raised an exception — extract message, call error handler.
        let error_msg = extract_exception_message();
        return call_error_handler_in_gil(owner, req, &error_msg);
    }

    let resp = parse_response_list(result);
    Py_DecRef(result);
    match resp {
        Some(r) => r,
        None => WebResponse::ok(b""),
    }
}

fn dispatch_before_to_python(
    owner: &Arc<PyOwner>,
    req: &WebRequest,
) -> Option<WebResponse> {
    let gil = unsafe { PyGILState_Ensure() };
    let result = unsafe { before_in_gil(owner.0, req) };
    unsafe { PyGILState_Release(gil) };
    result
}

unsafe fn before_in_gil(owner: PyObjectPtr, req: &WebRequest) -> Option<WebResponse> {
    let env = build_env(req);
    let result = call_owner_method(owner, c"native_run_before_filters", &[env]);
    if result.is_null() {
        let _ = extract_exception_message();
        return Some(WebResponse::internal_error("before filter raised an exception"));
    }
    let none_ptr = get_none_addr();
    if result as usize == none_ptr {
        Py_DecRef(result);
        return None;
    }
    let resp = parse_response_list(result);
    Py_DecRef(result);
    Some(resp.unwrap_or_else(|| WebResponse::internal_error("invalid before-filter response")))
}

fn dispatch_after_to_python(
    owner: &Arc<PyOwner>,
    req: &WebRequest,
    resp: WebResponse,
) -> WebResponse {
    let gil = unsafe { PyGILState_Ensure() };
    let result = unsafe { after_in_gil(owner.0, req, &resp) };
    unsafe { PyGILState_Release(gil) };
    result.unwrap_or(resp)
}

unsafe fn after_in_gil(
    owner: PyObjectPtr,
    req: &WebRequest,
    resp: &WebResponse,
) -> Option<WebResponse> {
    let env         = build_env(req);
    let response_py = web_response_to_py_list(resp);
    let result = call_owner_method(owner, c"native_run_after_filters", &[env, response_py]);
    if result.is_null() {
        let _ = extract_exception_message();
        return None;
    }
    let resp = parse_response_list(result);
    Py_DecRef(result);
    resp
}

fn dispatch_not_found_to_python(owner: &Arc<PyOwner>, req: &WebRequest) -> WebResponse {
    let gil = unsafe { PyGILState_Ensure() };
    let result = unsafe { not_found_in_gil(owner.0, req) };
    unsafe { PyGILState_Release(gil) };
    result.unwrap_or_else(WebResponse::not_found)
}

unsafe fn not_found_in_gil(owner: PyObjectPtr, req: &WebRequest) -> Option<WebResponse> {
    let env = build_env(req);
    let result = call_owner_method(owner, c"native_run_not_found", &[env]);
    if result.is_null() {
        let _ = extract_exception_message();
        return Some(WebResponse::internal_error("not_found handler raised an exception"));
    }
    let none_ptr = get_none_addr();
    if result as usize == none_ptr {
        Py_DecRef(result);
        return None;
    }
    let resp = parse_response_list(result);
    Py_DecRef(result);
    resp
}

/// Called while already holding the GIL after a route handler exception.
unsafe fn call_error_handler_in_gil(
    owner: PyObjectPtr,
    req: &WebRequest,
    error_msg: &str,
) -> WebResponse {
    let env    = build_env(req);
    let err_py = str_to_py(error_msg);
    let result = call_owner_method(owner, c"native_run_error_handler", &[env, err_py]);
    if result.is_null() {
        let _ = extract_exception_message();
        return WebResponse::internal_error(error_msg);
    }
    let none_ptr = get_none_addr();
    if result as usize == none_ptr {
        Py_DecRef(result);
        return WebResponse::internal_error(error_msg);
    }
    let resp = parse_response_list(result);
    Py_DecRef(result);
    resp.unwrap_or_else(|| WebResponse::internal_error(error_msg))
}

// ─────────────────────────────────────────────────────────────────────────────
// env dict builder
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn build_env(request: &WebRequest) -> PyObjectPtr {
    let env = PyDict_New();

    dict_set_str(env, "REQUEST_METHOD", request.method());
    dict_set_str(env, "PATH_INFO", request.path());
    let (_, query) = split_target(request.http.target());
    dict_set_str(env, "QUERY_STRING", query);

    let route_params = str_str_map_to_dict(&request.route_params);
    dict_set(env, "conduit.route_params", route_params);
    Py_DecRef(route_params);

    let query_params = str_str_map_to_dict(&request.query_params);
    dict_set(env, "conduit.query_params", query_params);
    Py_DecRef(query_params);

    let headers_dict = build_headers_dict(&request.http.head.headers);
    dict_set(env, "conduit.headers", headers_dict);
    Py_DecRef(headers_dict);

    let body_str = String::from_utf8_lossy(request.body());
    dict_set_str(env, "conduit.body", &body_str);

    dict_set_str(
        env,
        "SERVER_PROTOCOL",
        &format!("HTTP/{}.{}", request.http.head.version.major, request.http.head.version.minor),
    );

    dict_set_str(env, "REMOTE_ADDR", &request.peer_addr().ip().to_string());

    let remote_port = PyLong_FromLong(request.peer_addr().port() as c_long);
    dict_set(env, "REMOTE_PORT", remote_port);
    Py_DecRef(remote_port);

    dict_set_str(env, "SERVER_NAME", &request.http.connection.local_addr.ip().to_string());

    let server_port = PyLong_FromLong(request.http.connection.local_addr.port() as c_long);
    dict_set(env, "SERVER_PORT", server_port);
    Py_DecRef(server_port);

    if let Some(cl) = request.content_length() {
        let v = PyLong_FromLong(cl as c_long);
        dict_set(env, "conduit.content_length", v);
        Py_DecRef(v);
    }
    if let Some(ct) = request.content_type() {
        dict_set_str(env, "conduit.content_type", ct);
    }

    for header in &request.http.head.headers {
        dict_set_str(env, &header_env_key(&header.name), &header.value);
    }

    env
}

// ─────────────────────────────────────────────────────────────────────────────
// Response helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a Python `[status, [[name,val],...], body]` list into a WebResponse.
/// Returns None if the structure is invalid or if the value is not a 3-list.
unsafe fn parse_response_list(value: PyObjectPtr) -> Option<WebResponse> {
    let len = PyList_Size(value);
    if len != 3 {
        PyErr_Clear();
        return None;
    }

    let status_py  = PyList_GetItem(value, 0);
    let headers_py = PyList_GetItem(value, 1);
    let body_py    = PyList_GetItem(value, 2);

    let status_long = PyLong_AsLong(status_py);
    if status_long < 0 || status_long > 65535 { return None; }
    let status = status_long as u16;

    let headers = parse_header_pairs(headers_py)?;
    let body    = str_from_py(body_py)?.into_bytes();

    Some(WebResponse { status, headers, body })
}

unsafe fn web_response_to_py_list(resp: &WebResponse) -> PyObjectPtr {
    let outer = PyList_New(3);
    PyList_SetItem(outer, 0, PyLong_FromLong(resp.status as c_long));

    let headers = PyList_New(resp.headers.len() as isize);
    for (i, (name, val)) in resp.headers.iter().enumerate() {
        let pair = PyList_New(2);
        PyList_SetItem(pair, 0, str_to_py(name));
        PyList_SetItem(pair, 1, str_to_py(val));
        PyList_SetItem(headers, i as isize, pair);
    }
    PyList_SetItem(outer, 1, headers);
    PyList_SetItem(outer, 2, str_to_py(&String::from_utf8_lossy(&resp.body)));
    outer
}

unsafe fn parse_header_pairs(value: PyObjectPtr) -> Option<Vec<(String, String)>> {
    let len = PyList_Size(value);
    if len < 0 { PyErr_Clear(); return Some(vec![]); }
    let mut headers = Vec::new();
    for i in 0..len {
        let pair = PyList_GetItem(value, i);
        if PyList_Size(pair) != 2 { return None; }
        let name = str_from_py(PyList_GetItem(pair, 0))?;
        let val  = str_from_py(PyList_GetItem(pair, 1))?;
        headers.push((name, val));
    }
    Some(headers)
}

// ─────────────────────────────────────────────────────────────────────────────
// Exception extraction
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn extract_exception_message() -> String {
    let mut ptype:  PyObjectPtr = ptr::null_mut();
    let mut pvalue: PyObjectPtr = ptr::null_mut();
    let mut ptb:    PyObjectPtr = ptr::null_mut();
    PyErr_Fetch(&mut ptype, &mut pvalue, &mut ptb);
    PyErr_NormalizeException(&mut ptype, &mut pvalue, &mut ptb);

    let msg = if !pvalue.is_null() {
        let s = PyObject_Str(pvalue);
        let text = if !s.is_null() {
            let ptr = PyUnicode_AsUTF8(s);
            let r = if !ptr.is_null() {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            } else {
                "route handler raised an exception".to_string()
            };
            Py_DecRef(s);
            r
        } else {
            "route handler raised an exception".to_string()
        };
        text
    } else {
        "route handler raised an exception".to_string()
    };

    if !ptype.is_null()  { Py_DecRef(ptype); }
    if !pvalue.is_null() { Py_DecRef(pvalue); }
    if !ptb.is_null()    { Py_DecRef(ptb); }
    PyErr_Clear();
    msg
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility helpers
// ─────────────────────────────────────────────────────────────────────────────

unsafe fn get_state(capsule: PyObjectPtr) -> *mut PyConduitServer {
    PyCapsule_GetPointer(capsule, CAPSULE_NAME.as_ptr() as *const c_char)
        as *mut PyConduitServer
}

unsafe fn dict_set_str(dict: PyObjectPtr, key: &str, value: &str) {
    let k = str_to_py(key);
    let v = str_to_py(value);
    PyDict_SetItem(dict, k, v);
    Py_DecRef(k);
    Py_DecRef(v);
}

unsafe fn dict_set(dict: PyObjectPtr, key: &str, value: PyObjectPtr) {
    let k = str_to_py(key);
    PyDict_SetItem(dict, k, value);
    Py_DecRef(k);
}

unsafe fn str_str_map_to_dict(map: &std::collections::HashMap<String, String>) -> PyObjectPtr {
    let d = PyDict_New();
    for (key, val) in map {
        let k = str_to_py(key);
        let v = str_to_py(val);
        PyDict_SetItem(d, k, v);
        Py_DecRef(k);
        Py_DecRef(v);
    }
    d
}

unsafe fn build_headers_dict(headers: &[Header]) -> PyObjectPtr {
    let d = PyDict_New();
    for h in headers {
        let k = str_to_py(&h.name.to_ascii_lowercase());
        let v = str_to_py(&h.value);
        PyDict_SetItem(d, k, v);
        Py_DecRef(k);
        Py_DecRef(v);
    }
    d
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn header_env_key(name: &str) -> String {
    let normalized = name.replace('-', "_").to_ascii_uppercase();
    match normalized.as_str() {
        "CONTENT_TYPE" | "CONTENT_LENGTH" => normalized,
        _ => format!("HTTP_{normalized}"),
    }
}

/// Check if a Python object is the `None` singleton by pointer comparison.
/// Py_None is a singleton — comparing addresses is both correct and efficient.
unsafe fn is_py_none(obj: PyObjectPtr) -> bool {
    let none = py_none();   // new reference to Py_None
    let is_none = obj as usize == none as usize;
    Py_DecRef(none);
    is_none
}

/// Return the address of Py_None without holding a reference.
/// Used to check the return value of dispatch calls without extra incref.
unsafe fn get_none_addr() -> usize {
    let none = py_none();
    let addr = none as usize;
    Py_DecRef(none);
    addr
}

// ─────────────────────────────────────────────────────────────────────────────
// Platform-specific server binding
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn bind_server(
    host: &str,
    port: u16,
    options: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_kqueue((host, port), options, app)
}

#[cfg(target_os = "linux")]
fn bind_server(
    host: &str,
    port: u16,
    options: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_epoll((host, port), options, app)
}

#[cfg(target_os = "windows")]
fn bind_server(
    host: &str,
    port: u16,
    options: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_windows((host, port), options, app)
}

// ─────────────────────────────────────────────────────────────────────────────
// Module methods table and PyInit_conduit_native
// ─────────────────────────────────────────────────────────────────────────────
//
// The methods array and module def use `static mut` because Python's C API
// requires mutable pointers. The m_methods field is set at runtime in PyInit_
// to avoid taking a mutable raw pointer to a static in a const context.

static mut MODULE_METHODS: [PyMethodDef; 8] = [
    PyMethodDef {
        ml_name: c"server_new".as_ptr(),
        ml_meth: Some(server_new),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_new(owner, app, host, port, max_connections) -> capsule".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_serve".as_ptr(),
        ml_meth: Some(server_serve),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_serve(capsule) -> None  # blocks; releases GIL".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_stop".as_ptr(),
        ml_meth: Some(server_stop),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_stop(capsule) -> None".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_dispose".as_ptr(),
        ml_meth: Some(server_dispose),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_dispose(capsule) -> None".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_running".as_ptr(),
        ml_meth: Some(server_running),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_running(capsule) -> bool".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_local_host".as_ptr(),
        ml_meth: Some(server_local_host),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_local_host(capsule) -> str".as_ptr(),
    },
    PyMethodDef {
        ml_name: c"server_local_port".as_ptr(),
        ml_meth: Some(server_local_port),
        ml_flags: METH_VARARGS,
        ml_doc: c"server_local_port(capsule) -> int".as_ptr(),
    },
    method_def_sentinel(),
];

static mut MODULE_DEF: PyModuleDef = PyModuleDef {
    m_base: PyModuleDef_Base {
        ob_base: [0u8; std::mem::size_of::<usize>() * 2],
        m_init: None,
        m_index: 0,
        m_copy: ptr::null_mut(),
    },
    m_name: c"conduit_native".as_ptr(),
    m_doc: c"Conduit native extension — Python C API bridge to web-core".as_ptr(),
    m_size: -1,
    m_methods: ptr::null_mut(), // set in PyInit_ below
    m_slots:   ptr::null_mut(),
    m_traverse: ptr::null_mut(),
    m_clear:   ptr::null_mut(),
    m_free:    ptr::null_mut(),
};

#[no_mangle]
pub unsafe extern "C" fn PyInit_conduit_native() -> PyObjectPtr {
    // Set m_methods here because we can't take a raw pointer to a mutable
    // static in a const context (required for static initializers).
    #[allow(static_mut_refs)]
    { MODULE_DEF.m_methods = MODULE_METHODS.as_mut_ptr(); }
    PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION)
}
