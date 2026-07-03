//! # conduit-capi — a reusable C ABI for the Conduit web framework
//!
//! This crate exposes the Rust `conduit` facade (WEB08, over `web-core`) as a
//! plain C ABI so that **any** C-capable language can host a Conduit app without
//! re-implementing dispatch and marshaling against its own runtime. Swift (WEB12)
//! is the first consumer; C++ (WEB13), Go (WEB14), C# (WEB15), F# (WEB16), Dart
//! (WEB17), and Haskell (WEB18) reuse the very same surface.
//!
//! ## Why one C ABI instead of seven wrappers
//!
//! The managed-VM ports (Java JNI, Lua, Perl XS) each re-marshal requests and
//! re-audit the trust boundary against their host VM. The remaining ports are all
//! C-ABI-capable, so we cross the boundary **once**, correctly, here: header
//! injection defense, status clamping, UTF-8 validation, and panic isolation all
//! live in this crate.
//!
//! ## Dispatch model
//!
//! A handler is a C function pointer plus an opaque `ctx` (the host language boxes
//! its closure and hands us the pointer) and a `ctx_free` destructor we call when
//! the owning app/server is disposed. On each request we build a [`ConduitRequest`]
//! view, invoke the callback, and take ownership of the returned
//! [`ConduitResponse`] (a boxed `WebResponse`). NULL means "no response":
//! `continue` for a before-filter, or — for a route — route through the error
//! handler using the message the host stashed via [`conduit_capi_report_error`].
//!
//! ## Threading
//!
//! `embeddable-http-server` runs its reactor inline on the thread that calls
//! `serve()` (single `TcpRuntime`), so foreground serving dispatches on the
//! caller's thread — no lock required. `serve_background` spawns one OS thread.
//! Host closures must be thread-safe, which the facade's `Fn + Send + Sync` bound
//! already requires; the stored `ctx` is wrapped in a `Send + Sync` newtype.

#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use conduit::{Application, Server as ConduitServer};
use embeddable_http_server::HttpServerOptions;
use web_core::{WebRequest, WebResponse};

// ── Thread-local error channels ─────────────────────────────────────────────

thread_local! {
    /// Last error message for `conduit_last_error()`. Owned, NUL-terminated.
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
    /// Error message the host stashes (via `conduit_capi_report_error`) when a
    /// route handler fails for a non-halt reason, consumed by the error handler.
    static PENDING_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    let c = CString::new(msg.replace('\0', "")).unwrap_or_default();
    LAST_ERROR.with(|e| *e.borrow_mut() = c);
}

/// Host calls this from inside a handler before returning NULL to signal that the
/// route failed with `msg` (which the error handler then receives).
#[no_mangle]
pub unsafe extern "C" fn conduit_capi_report_error(msg: *const c_char) {
    let s = cstr_to_string(msg).unwrap_or_default();
    PENDING_ERROR.with(|e| *e.borrow_mut() = Some(s));
}

#[no_mangle]
pub extern "C" fn conduit_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// Build a `CString`, stripping any interior NUL bytes (defensive — HTTP tokens
/// shouldn't contain them, but a malformed input must not panic).
fn cstr(s: &str) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_| CString::new(s.replace('\0', "")).unwrap())
}

/// Convert a borrowed C string to an owned `String`, validating UTF-8. Returns
/// `None` for a null pointer or invalid UTF-8 (we never feed garbage downstream).
unsafe fn cstr_to_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok().map(|s| s.to_owned())
}

/// A header name/value is safe iff it carries no control bytes (`< 0x20` or
/// `0x7f`) and the name has no `:`. This blocks response-splitting and header
/// smuggling at the one boundary every host language shares.
fn header_safe(name: &str, value: &str) -> bool {
    let bad_name = |b: u8| b < 0x20 || b == 0x7f || b == b':';
    let bad_value = |b: u8| b < 0x20 || b == 0x7f;
    !name.is_empty() && !name.bytes().any(bad_name) && !value.bytes().any(bad_value)
}

/// Move a `T` across a thread boundary by asserting `Send`. Used only for the
/// owned `ConduitServer` handed to the background serve thread; the server's
/// stored callbacks are already `Send + Sync`.
struct AssertSend<T>(T);
unsafe impl<T> Send for AssertSend<T> {}

// ── Callback bundles (function pointer + opaque ctx + destructor) ────────────

type RawHandler = extern "C" fn(*mut c_void, *const ConduitRequest) -> *mut ConduitResponse;
type RawAfter =
    extern "C" fn(*mut c_void, *const ConduitRequest, *mut ConduitResponse) -> *mut ConduitResponse;
type RawCtxFree = extern "C" fn(*mut c_void);

struct HandlerCb {
    func: RawHandler,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
}
// SAFETY: the host guarantees `ctx` is safe to use/free from any single thread at
// a time (the facade's `Fn + Send + Sync` contract). We never alias it.
unsafe impl Send for HandlerCb {}
unsafe impl Sync for HandlerCb {}
impl Drop for HandlerCb {
    fn drop(&mut self) {
        if let Some(f) = self.free {
            f(self.ctx);
        }
    }
}
impl HandlerCb {
    /// Invoke the callback for `req`; `None` if it returned NULL.
    unsafe fn call(&self, req: &WebRequest, error_msg: Option<&str>) -> Option<WebResponse> {
        let creq = ConduitRequest::new(req, error_msg);
        let p = (self.func)(self.ctx, &creq as *const ConduitRequest);
        if p.is_null() {
            None
        } else {
            Some(Box::from_raw(p).inner)
        }
    }
}

struct AfterCb {
    func: RawAfter,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
}
unsafe impl Send for AfterCb {}
unsafe impl Sync for AfterCb {}
impl Drop for AfterCb {
    fn drop(&mut self) {
        if let Some(f) = self.free {
            f(self.ctx);
        }
    }
}
impl AfterCb {
    /// Run the transforming after-hook. Taking `&self` forces the surrounding
    /// closure to capture the whole (`Send + Sync`) `AfterCb` rather than its
    /// individual fields — edition-2021 disjoint capture would otherwise grab the
    /// raw `ctx` pointer, which is not `Sync`.
    unsafe fn call(&self, req: &WebRequest, resp: WebResponse) -> WebResponse {
        let creq = ConduitRequest::new(req, None);
        let cresp = Box::into_raw(Box::new(ConduitResponse::wrap(resp)));
        let p = (self.func)(self.ctx, &creq as *const ConduitRequest, cresp);
        if p.is_null() {
            WebResponse::internal_error("after hook returned null")
        } else {
            Box::from_raw(p).inner
        }
    }
}

type ErrorSlot = Arc<Mutex<Option<HandlerCb>>>;

// ── Opaque handles ──────────────────────────────────────────────────────────

/// The application handle. Holds the facade plus a shared error-handler slot the
/// route closures consult when a handler fails.
pub struct CapiApp {
    app: Application,
    error_handler: ErrorSlot,
}

/// The server handle.
pub struct CapiServer {
    server: Option<ConduitServer>,
    stop: tcp_runtime::StopHandle,
    port: u16,
    running: Arc<AtomicBool>,
    bg: Option<std::thread::JoinHandle<()>>,
    // Kept alive so the error handler's ctx is freed on disposal even if no route
    // closure references survive.
    _error_handler: ErrorSlot,
}
// SAFETY: every field is `Send`; the callbacks inside the server are `Send + Sync`.
unsafe impl Send for CapiServer {}

/// A per-dispatch, read-only view of the request. All accessors return pointers
/// valid only for the duration of the callback.
pub struct ConduitRequest {
    method: CString,
    path: CString,
    query_string: CString,
    content_type: CString,
    remote_addr: CString,
    error: CString,
    body: Vec<u8>,
    route_params: std::collections::HashMap<String, String>,
    query_params: std::collections::HashMap<String, String>,
    headers: std::collections::HashMap<String, String>, // lowercased names, first wins
    // Holds CStrings returned by param/query/header lookups so their pointers stay
    // valid for the whole callback. Single-threaded per dispatch.
    arena: RefCell<Vec<CString>>,
}

impl ConduitRequest {
    fn new(req: &WebRequest, error_msg: Option<&str>) -> Self {
        let target = req.http.head.target.as_str();
        let qs = target.find('?').map(|i| &target[i + 1..]).unwrap_or("");

        let mut headers = std::collections::HashMap::new();
        for h in &req.http.head.headers {
            headers
                .entry(h.name.to_ascii_lowercase())
                .or_insert_with(|| h.value.clone());
        }

        ConduitRequest {
            method: cstr(req.method()),
            path: cstr(req.path()),
            query_string: cstr(qs),
            content_type: cstr(req.content_type().unwrap_or("")),
            remote_addr: cstr(&req.peer_addr().ip().to_string()),
            error: cstr(error_msg.unwrap_or("")),
            body: req.body().to_vec(),
            route_params: req.route_params.clone(),
            query_params: req.query_params.clone(),
            headers,
            arena: RefCell::new(Vec::new()),
        }
    }

    fn lookup(&self, map: &std::collections::HashMap<String, String>, key: &str) -> *const c_char {
        match map.get(key) {
            Some(v) => {
                let c = cstr(v);
                let p = c.as_ptr();
                self.arena.borrow_mut().push(c);
                p
            }
            None => ptr::null(),
        }
    }
}

/// A response handle, wrapping an owned `WebResponse`. `hdr_arena` caches the
/// `CString`s returned by the header read accessors so their pointers stay valid.
pub struct ConduitResponse {
    inner: WebResponse,
    hdr_arena: RefCell<Vec<CString>>,
}

impl ConduitResponse {
    fn wrap(inner: WebResponse) -> Self {
        ConduitResponse {
            inner,
            hdr_arena: RefCell::new(Vec::new()),
        }
    }
}

// ── App lifecycle ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn conduit_app_new() -> *mut CapiApp {
    Box::into_raw(Box::new(CapiApp {
        app: Application::new(),
        error_handler: Arc::new(Mutex::new(None)),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_free(app: *mut CapiApp) {
    if !app.is_null() {
        drop(Box::from_raw(app));
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_set_setting(
    app: *mut CapiApp,
    key: *const c_char,
    value: *const c_char,
) {
    if app.is_null() {
        return;
    }
    if let (Some(k), Some(v)) = (cstr_to_string(key), cstr_to_string(value)) {
        (*app).app.set(k, v);
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_get_setting(
    app: *mut CapiApp,
    key: *const c_char,
) -> *mut c_char {
    if app.is_null() {
        return ptr::null_mut();
    }
    let Some(k) = cstr_to_string(key) else {
        return ptr::null_mut();
    };
    match (*app).app.setting(&k) {
        Some(v) => cstr(v).into_raw(),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_add_route(
    app: *mut CapiApp,
    method: *const c_char,
    pattern: *const c_char,
    func: RawHandler,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
) {
    if app.is_null() {
        return;
    }
    let app = &mut *app;
    let method = cstr_to_string(method).unwrap_or_default();
    let pattern = cstr_to_string(pattern).unwrap_or_default();
    let cb = HandlerCb { func, ctx, free };
    let errs = Arc::clone(&app.error_handler);
    app.app.route(method, &pattern, move |req| {
        PENDING_ERROR.with(|e| *e.borrow_mut() = None);
        match cb.call(req, None) {
            Some(resp) => resp,
            None => {
                let msg = PENDING_ERROR
                    .with(|e| e.borrow_mut().take())
                    .unwrap_or_else(|| "handler error".to_string());
                let slot = errs.lock().unwrap();
                match slot.as_ref() {
                    Some(ehcb) => ehcb
                        .call(req, Some(&msg))
                        .unwrap_or_else(|| WebResponse::internal_error(&msg)),
                    None => WebResponse::internal_error("Internal Server Error"),
                }
            }
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_add_before(
    app: *mut CapiApp,
    func: RawHandler,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
) {
    if app.is_null() {
        return;
    }
    let cb = HandlerCb { func, ctx, free };
    (*app).app.before(move |req| cb.call(req, None));
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_add_after(
    app: *mut CapiApp,
    func: RawAfter,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
) {
    if app.is_null() {
        return;
    }
    let cb = AfterCb { func, ctx, free };
    (*app).app.after_response(move |req, resp| cb.call(req, resp));
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_set_not_found(
    app: *mut CapiApp,
    func: RawHandler,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
) {
    if app.is_null() {
        return;
    }
    let cb = HandlerCb { func, ctx, free };
    (*app)
        .app
        .not_found(move |req| cb.call(req, None).unwrap_or_else(WebResponse::not_found));
}

#[no_mangle]
pub unsafe extern "C" fn conduit_app_set_error_handler(
    app: *mut CapiApp,
    func: RawHandler,
    ctx: *mut c_void,
    free: Option<RawCtxFree>,
) {
    if app.is_null() {
        return;
    }
    let cb = HandlerCb { func, ctx, free };
    *(*app).error_handler.lock().unwrap() = Some(cb);
}

// ── Server ──────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn conduit_server_bind(
    host: *const c_char,
    port: u16,
    app: *mut CapiApp,
) -> *mut CapiServer {
    if app.is_null() {
        set_last_error("conduit_server_bind: null app");
        return ptr::null_mut();
    }
    let capi = *Box::from_raw(app);
    let CapiApp { app, error_handler } = capi;
    let host = cstr_to_string(host).unwrap_or_else(|| "127.0.0.1".to_string());

    match ConduitServer::bind_with_options(&host, port, HttpServerOptions::default(), app) {
        Ok(server) => {
            let port = server.local_addr().port();
            let stop = server.stop_handle();
            Box::into_raw(Box::new(CapiServer {
                server: Some(server),
                stop,
                port,
                running: Arc::new(AtomicBool::new(false)),
                bg: None,
                _error_handler: error_handler,
            }))
        }
        Err(e) => {
            set_last_error(&format!("conduit_server_bind: {e:?}"));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_serve(srv: *mut CapiServer) -> c_int {
    if srv.is_null() {
        return -1;
    }
    let s = &mut *srv;
    let Some(server) = s.server.as_mut() else {
        set_last_error("conduit_server_serve: server already consumed (background?)");
        return -1;
    };
    s.running.store(true, Ordering::SeqCst);
    // Isolate panics at the ABI edge: a before/after hook running in a host
    // language could panic (web-core only wraps the route handler in
    // catch_unwind), and unwinding across `extern "C"` is UB. Catch it here.
    let r = catch_unwind(AssertUnwindSafe(|| server.serve()));
    s.running.store(false, Ordering::SeqCst);
    match r {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => {
            set_last_error(&format!("conduit_server_serve: {e:?}"));
            -1
        }
        Err(_) => {
            set_last_error("conduit_server_serve: handler panicked");
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_serve_background(srv: *mut CapiServer) -> c_int {
    if srv.is_null() {
        return -1;
    }
    let s = &mut *srv;
    let Some(server) = s.server.take() else {
        set_last_error("conduit_server_serve_background: server already consumed");
        return -1;
    };
    let running = Arc::clone(&s.running);
    running.store(true, Ordering::SeqCst);
    let moved = AssertSend(server);
    let handle = std::thread::spawn(move || {
        let AssertSend(mut server) = moved;
        // Same ABI-edge panic isolation as the foreground path: a host hook must
        // not unwind out of the thread that re-enters host code.
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _ = server.serve();
        }));
        running.store(false, Ordering::SeqCst);
    });
    s.bg = Some(handle);
    0
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_stop(srv: *mut CapiServer) {
    if srv.is_null() {
        return;
    }
    let s = &mut *srv;
    s.stop.stop();
    if let Some(h) = s.bg.take() {
        let _ = h.join();
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_local_port(srv: *mut CapiServer) -> u16 {
    if srv.is_null() {
        0
    } else {
        (*srv).port
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_running(srv: *mut CapiServer) -> c_int {
    if srv.is_null() {
        0
    } else if (*srv).running.load(Ordering::SeqCst) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_server_free(srv: *mut CapiServer) {
    if srv.is_null() {
        return;
    }
    let mut s = Box::from_raw(srv);
    s.stop.stop();
    if let Some(h) = s.bg.take() {
        let _ = h.join();
    }
    // Dropping `s` frees the server (and all handler ctx via HandlerCb::drop).
}

// ── Request accessors ───────────────────────────────────────────────────────

macro_rules! req_str_accessor {
    ($name:ident, $field:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(req: *const ConduitRequest) -> *const c_char {
            if req.is_null() {
                EMPTY_CSTR.as_ptr()
            } else {
                (*req).$field.as_ptr()
            }
        }
    };
}

// A stable empty C string for null-request safety.
static EMPTY_CSTR_STORAGE: &[u8] = b"\0";
#[allow(non_upper_case_globals)]
static EMPTY_CSTR: EmptyC = EmptyC;
struct EmptyC;
impl EmptyC {
    fn as_ptr(&self) -> *const c_char {
        EMPTY_CSTR_STORAGE.as_ptr() as *const c_char
    }
}

req_str_accessor!(conduit_request_method, method);
req_str_accessor!(conduit_request_path, path);
req_str_accessor!(conduit_request_query_string, query_string);
req_str_accessor!(conduit_request_content_type, content_type);
req_str_accessor!(conduit_request_remote_addr, remote_addr);
req_str_accessor!(conduit_request_error, error);

#[no_mangle]
pub unsafe extern "C" fn conduit_request_body(
    req: *const ConduitRequest,
    out_len: *mut usize,
) -> *const u8 {
    if req.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return ptr::null();
    }
    let b = &(*req).body;
    if !out_len.is_null() {
        *out_len = b.len();
    }
    b.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn conduit_request_param(
    req: *const ConduitRequest,
    name: *const c_char,
) -> *const c_char {
    if req.is_null() {
        return ptr::null();
    }
    let Some(key) = cstr_to_string(name) else {
        return ptr::null();
    };
    let r = &*req;
    r.lookup(&r.route_params, &key)
}

#[no_mangle]
pub unsafe extern "C" fn conduit_request_query(
    req: *const ConduitRequest,
    name: *const c_char,
) -> *const c_char {
    if req.is_null() {
        return ptr::null();
    }
    let Some(key) = cstr_to_string(name) else {
        return ptr::null();
    };
    let r = &*req;
    r.lookup(&r.query_params, &key)
}

#[no_mangle]
pub unsafe extern "C" fn conduit_request_header(
    req: *const ConduitRequest,
    name: *const c_char,
) -> *const c_char {
    if req.is_null() {
        return ptr::null();
    }
    let Some(key) = cstr_to_string(name) else {
        return ptr::null();
    };
    let r = &*req;
    r.lookup(&r.headers, &key.to_ascii_lowercase())
}

// ── Response builder / reader ───────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn conduit_response_new(
    status: u16,
    body: *const u8,
    body_len: usize,
) -> *mut ConduitResponse {
    let clamped = status.clamp(100, 599);
    let bytes = if body.is_null() || body_len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(body, body_len).to_vec()
    };
    Box::into_raw(Box::new(ConduitResponse::wrap(WebResponse::new(clamped, bytes))))
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_set_header(
    resp: *mut ConduitResponse,
    name: *const c_char,
    value: *const c_char,
) {
    if resp.is_null() {
        return;
    }
    if let (Some(n), Some(v)) = (cstr_to_string(name), cstr_to_string(value)) {
        if header_safe(&n, &v) {
            (*resp).inner.headers.push((n, v));
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_status(resp: *const ConduitResponse) -> u16 {
    if resp.is_null() {
        0
    } else {
        (*resp).inner.status
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_body(
    resp: *const ConduitResponse,
    out_len: *mut usize,
) -> *const u8 {
    if resp.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return ptr::null();
    }
    let b = &(*resp).inner.body;
    if !out_len.is_null() {
        *out_len = b.len();
    }
    b.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_header_count(resp: *const ConduitResponse) -> usize {
    if resp.is_null() {
        0
    } else {
        (*resp).inner.headers.len()
    }
}

/// Name of the `i`-th response header. The returned pointer is valid until the
/// response is mutated or freed. NULL if `i` is out of range. The header's owned
/// `CString` is cached on the response so the pointer stays valid for the call.
#[no_mangle]
pub unsafe extern "C" fn conduit_response_header_name(
    resp: *const ConduitResponse,
    i: usize,
) -> *const c_char {
    header_field(resp, i, true)
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_header_value(
    resp: *const ConduitResponse,
    i: usize,
) -> *const c_char {
    header_field(resp, i, false)
}

unsafe fn header_field(resp: *const ConduitResponse, i: usize, name: bool) -> *const c_char {
    if resp.is_null() {
        return ptr::null();
    }
    let r = &*resp;
    match r.inner.headers.get(i) {
        Some((n, v)) => {
            let c = cstr(if name { n } else { v });
            let p = c.as_ptr();
            r.hdr_arena.borrow_mut().push(c);
            p
        }
        None => ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_response_free(resp: *mut ConduitResponse) {
    if !resp.is_null() {
        drop(Box::from_raw(resp));
    }
}

#[no_mangle]
pub unsafe extern "C" fn conduit_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ── Tests (pure-Rust helpers, no C artifacts needed) ────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_safe_blocks_crlf_and_colon() {
        assert!(header_safe("content-type", "text/html"));
        assert!(!header_safe("x", "a\r\nb")); // CR/LF
        assert!(!header_safe("x", "a\nb"));
        assert!(!header_safe("bad:name", "v")); // colon in name
        assert!(!header_safe("", "v")); // empty name
        assert!(!header_safe("x", "a\x01b")); // control byte
        assert!(!header_safe("x\x7f", "v")); // DEL
    }

    #[test]
    fn cstr_strips_interior_nul() {
        // A string with an interior NUL must still produce a valid CString.
        let c = cstr("a\0b");
        assert_eq!(c.to_str().unwrap(), "ab");
    }

    #[test]
    fn response_status_is_clamped() {
        unsafe {
            let r = conduit_response_new(700, ptr::null(), 0);
            assert_eq!(conduit_response_status(r), 599);
            conduit_response_free(r);
            let r = conduit_response_new(0, ptr::null(), 0);
            assert_eq!(conduit_response_status(r), 100);
            conduit_response_free(r);
        }
    }

    #[test]
    fn response_set_header_drops_unsafe() {
        unsafe {
            let r = conduit_response_new(200, ptr::null(), 0);
            let ok_n = CString::new("x-ok").unwrap();
            let ok_v = CString::new("fine").unwrap();
            conduit_response_set_header(r, ok_n.as_ptr(), ok_v.as_ptr());
            let bad_n = CString::new("x-bad").unwrap();
            let bad_v = CString::new("a\r\nb").unwrap();
            conduit_response_set_header(r, bad_n.as_ptr(), bad_v.as_ptr());
            assert_eq!((*r).inner.headers.len(), 1); // only the safe one
            conduit_response_free(r);
        }
    }

    #[test]
    fn app_setting_round_trips() {
        unsafe {
            let app = conduit_app_new();
            let k = CString::new("views").unwrap();
            let v = CString::new("tmpl").unwrap();
            conduit_app_set_setting(app, k.as_ptr(), v.as_ptr());
            let got = conduit_app_get_setting(app, k.as_ptr());
            assert!(!got.is_null());
            assert_eq!(CStr::from_ptr(got).to_str().unwrap(), "tmpl");
            conduit_string_free(got);
            let missing = CString::new("nope").unwrap();
            assert!(conduit_app_get_setting(app, missing.as_ptr()).is_null());
            conduit_app_free(app);
        }
    }
}
