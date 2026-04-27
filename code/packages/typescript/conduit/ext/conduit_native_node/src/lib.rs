// lib.rs — conduit_native_node
//
// N-API native addon bridging TypeScript/Node.js to the Rust web-core engine.
// Loaded by Node.js via `require('./conduit_native_node.node')`.
//
// # Threading model
//
// Node.js runs JS on a single V8 event loop thread. N-API calls may ONLY be
// made from this thread. web-core dispatches HTTP requests on background Rust
// I/O threads. We bridge using `napi_threadsafe_function` (TSFN):
//
//   Background thread → tsfn_call(tsfn, slot_ptr, BLOCKING)  (blocks)
//   V8 main thread    → call_js_cb(env, js_fn, ctx, slot_ptr)
//                         build ctx, call js_fn(ctx), parse result
//                         write to slot.response, signal condvar
//   Background thread → wakes up, reads response
//
// # HaltError protocol
//
// `conduit.halt(status, body)` throws a HaltError with `__conduit_halt=true`.
// call_js_cb checks `exception_pending`, extracts the exception, and checks
// `__conduit_halt`. The TypeScript HaltError constructor also sets
// `haltHeaderPairs: [string,string][]` for header extraction without needing
// `napi_get_all_property_names` (N-API v6+).
//
// # Response sentinel
//
// parse_response() returns status=0 for `undefined` (no override). The
// is_no_override() helper checks for this sentinel. Before/after hooks and
// the not_found handler use this to distinguish "return nothing" from
// "return a real response".

// clippy::not_unsafe_ptr_arg_deref — fires on every N-API extern "C" callback
// because they all receive raw pointer args by ABI contract.  Suppressed at
// module scope since this entire module is an N-API bridge; each unsafe block
// is individually reviewed.  For non-callback helpers the safety is documented
// at each call site.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use node_bridge::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use web_core::{WebApp, WebRequest, WebResponse, WebServer};

// ---------------------------------------------------------------------------
// Platform-specific type aliases
// ---------------------------------------------------------------------------

#[cfg(any(
    target_os = "macos",   target_os = "freebsd", target_os = "openbsd",
    target_os = "netbsd",  target_os = "dragonfly"
))]
type PlatformWebServer = WebServer<transport_platform::bsd::KqueueTransportPlatform>;

#[cfg(target_os = "linux")]
type PlatformWebServer = WebServer<transport_platform::linux::EpollTransportPlatform>;

#[cfg(target_os = "windows")]
type PlatformWebServer = WebServer<transport_platform::windows::WindowsTransportPlatform>;

// ---------------------------------------------------------------------------
// bind_server — platform-conditional socket bind
// ---------------------------------------------------------------------------

#[cfg(any(
    target_os = "macos",   target_os = "freebsd", target_os = "openbsd",
    target_os = "netbsd",  target_os = "dragonfly"
))]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app:  Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_kqueue((host, port), opts, app)
}

#[cfg(target_os = "linux")]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app:  Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_epoll((host, port), opts, app)
}

#[cfg(target_os = "windows")]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app:  Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_windows((host, port), opts, app)
}

// ---------------------------------------------------------------------------
// ThreadSafePtr — newtype to make napi_threadsafe_function Send+Sync
// ---------------------------------------------------------------------------
//
// web-core closures must be Send+Sync. napi_threadsafe_function is *mut c_void
// which is !Send. This wrapper asserts the invariant: the TSFN handle is only
// ever touched via tsfn_call() from background threads (the N-API-approved
// cross-thread operation). All other TSFN operations happen on the V8 thread.

struct ThreadSafePtr(napi_threadsafe_function);
unsafe impl Send for ThreadSafePtr {}
unsafe impl Sync for ThreadSafePtr {}

impl ThreadSafePtr {
    /// Return the raw TSFN handle.
    ///
    /// Using a method rather than `.0` field access is important for Rust 2021
    /// closure capture rules: `move || self.get()` captures the whole
    /// `ThreadSafePtr` (which is Send+Sync), whereas `move || self.0` would
    /// capture only the inner `*mut c_void` field (which is !Send), causing a
    /// compile error even though `ThreadSafePtr` has `unsafe impl Send`.
    #[inline(always)]
    fn get(&self) -> napi_threadsafe_function { self.0 }
}

// ---------------------------------------------------------------------------
// RequestSlot — per-request heap context transferred through the TSFN
// ---------------------------------------------------------------------------

struct RequestSlot {
    env_map:       HashMap<String, String>,
    route_params:  HashMap<String, String>,
    query_params:  HashMap<String, String>,
    headers:       HashMap<String, String>,
    /// Set for error handler calls; empty for all other handler types.
    error_message: Option<String>,
    response:      Arc<(Mutex<Option<WebResponse>>, Condvar)>,
}

unsafe impl Send for RequestSlot {}

// ---------------------------------------------------------------------------
// NativeApp — holds JS function stable-refs for all registered handlers
// ---------------------------------------------------------------------------

struct RouteEntry {
    method:      String,
    pattern:     String,
    handler_ref: napi_ref,
}

struct NativeApp {
    routes:            Vec<RouteEntry>,
    before_refs:       Vec<napi_ref>,
    after_refs:        Vec<napi_ref>,
    not_found_ref:     Option<napi_ref>,
    error_handler_ref: Option<napi_ref>,
    settings:          HashMap<String, String>,
}

impl NativeApp {
    fn new() -> Self {
        NativeApp {
            routes: Vec::new(),
            before_refs: Vec::new(),
            after_refs: Vec::new(),
            not_found_ref: None,
            error_handler_ref: None,
            settings: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// NativeServer — owns the TsFns, stop handle, and server background thread
// ---------------------------------------------------------------------------

struct RouteTsfn {
    method:  String,
    pattern: String,
    tsfn:    napi_threadsafe_function,
}

struct NativeServer {
    /// Live napi_env — only dereferenced on the V8 thread.
    #[allow(dead_code)]
    env: napi_env,
    /// Stable port number captured at bind time (before serve() moves the server).
    local_port: u16,
    /// StopHandle captured at bind time so stop() can signal the serve thread.
    stop_handle: tcp_runtime::StopHandle,
    route_tsfns:        Vec<RouteTsfn>,
    before_tsfns:       Vec<napi_threadsafe_function>,
    after_tsfns:        Vec<napi_threadsafe_function>,
    not_found_tsfn:     Option<napi_threadsafe_function>,
    error_handler_tsfn: Option<napi_threadsafe_function>,
    /// The server is moved into the background thread at serve() time.
    server:     Option<PlatformWebServer>,
    running:    Arc<AtomicBool>,
    bg_thread:  Option<std::thread::JoinHandle<()>>,
    /// Track whether TsFns have already been released (avoid double-release).
    tsfns_released: bool,
}

// SAFETY:
// • env is only used on the V8 main thread (stop, finalize_server).
// • TSFN handles are passed to background threads ONLY via tsfn_call(),
//   which is the N-API-approved cross-thread path.
// • tcp_runtime::StopHandle is explicitly Send (it wraps an Arc).
// • PlatformWebServer is Send (used only on the bg_thread after move).
// • Arc<AtomicBool> is Send+Sync by definition.
unsafe impl Send for NativeServer {}
unsafe impl Sync for NativeServer {}

// ---------------------------------------------------------------------------
// build_env_map — copy WebRequest into thread-safe Rust structures
// ---------------------------------------------------------------------------

fn build_env_map(req: &WebRequest) -> (
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, String>,
) {
    let mut env = HashMap::new();
    env.insert("REQUEST_METHOD".into(), req.method().to_string());
    env.insert("PATH_INFO".into(),      req.path().to_string());

    // Extract QUERY_STRING from the raw request target (e.g. "/foo?q=bar" → "q=bar").
    let target = req.http.head.target.as_str();
    let query_string = target.find('?')
        .map(|i| &target[i+1..])
        .unwrap_or("");
    env.insert("QUERY_STRING".into(), query_string.to_string());

    if let Some(ct) = req.content_type() {
        env.insert("conduit.content_type".into(), ct.to_string());
    }
    if let Some(cl) = req.content_length() {
        env.insert("conduit.content_length".into(), cl.to_string());
    }

    // Body: convert bytes to UTF-8 string (lossy — binary bodies are unusual in REST APIs).
    let body_str = String::from_utf8_lossy(req.body()).into_owned();
    env.insert("conduit.body".into(), body_str);

    let peer = req.peer_addr();
    env.insert("REMOTE_ADDR".into(), peer.ip().to_string());
    env.insert("REMOTE_PORT".into(), peer.port().to_string());

    // Build headers HashMap (lowercase keys, first value wins for duplicates).
    let mut headers: HashMap<String, String> = HashMap::new();
    for h in &req.http.head.headers {
        headers.entry(h.name.to_lowercase()).or_insert_with(|| h.value.clone());
    }

    (env, req.route_params.clone(), req.query_params.clone(), headers)
}

// ---------------------------------------------------------------------------
// call_js_cb — runs on the V8 main thread for every TSFN call
// ---------------------------------------------------------------------------
//
// DESIGN: `func` is passed as NULL to `napi_create_threadsafe_function`
// (avoiding Node.js's IsFunction check which can spuriously return
// napi_invalid_arg on some platforms).  Instead, the `context` parameter
// carries the `napi_ref` to the JS handler function.  We dereference it here.

unsafe extern "C" fn call_js_cb(
    env:  napi_env,
    _js_cb: napi_value,   // always null — we get the function from context
    ctx:  *mut c_void,    // napi_ref to the JS handler
    data: *mut c_void,
) {
    let slot = Box::from_raw(data as *mut RequestSlot);

    // Retrieve the JS handler function from the stable napi_ref in context.
    let handler_ref = ctx as napi_ref;
    let js_fn = deref(env, handler_ref);

    // Build the CGI-style env map JS object to pass as the single argument.
    let env_obj = build_env_obj(env, &slot);

    let call_result = call_function(env, undefined(env), js_fn, &[env_obj]);

    let response = if exception_pending(env) {
        let ex = clear_exception(env);
        extract_halt_or_error(env, ex)
    } else {
        match call_result {
            Some(val) => parse_response(env, val),
            None      => no_override_response(),
        }
    };

    let (lock, cvar) = &*slot.response;
    let mut guard = lock.lock().unwrap();
    *guard = Some(response);
    cvar.notify_one();
}

// ---------------------------------------------------------------------------
// build_env_obj — construct the flat CGI-style env map for JS handlers
// ---------------------------------------------------------------------------
//
// Produces a Record<string, string> JS object with the Rack/CGI-style keys:
//
//   REQUEST_METHOD, PATH_INFO, QUERY_STRING, REMOTE_ADDR, REMOTE_PORT
//   conduit.content_type, conduit.content_length, conduit.body
//   conduit.route_params  ← JSON-encoded HashMap<String,String>
//   conduit.query_params  ← JSON-encoded HashMap<String,String>
//   conduit.headers       ← JSON-encoded HashMap<String,String>
//   conduit.error         ← error message (only for error handler calls)
//
// The `Request` TypeScript class reads these exact keys.

unsafe fn build_env_obj(env: napi_env, slot: &RequestSlot) -> napi_value {
    let obj = object_new(env);

    // Flat string fields from env_map.
    let flat_keys = [
        "REQUEST_METHOD", "PATH_INFO", "QUERY_STRING",
        "REMOTE_ADDR", "REMOTE_PORT",
        "conduit.content_type", "conduit.content_length", "conduit.body",
    ];
    for key in flat_keys {
        if let Some(val) = slot.env_map.get(key) {
            set_named_property(env, obj, key, str_to_js(env, val.as_str()));
        }
    }

    // JSON-encoded maps — the TypeScript Request class parses these.
    set_named_property(env, obj, "conduit.route_params",
        str_to_js(env, &map_to_json(&slot.route_params)));
    set_named_property(env, obj, "conduit.query_params",
        str_to_js(env, &map_to_json(&slot.query_params)));
    set_named_property(env, obj, "conduit.headers",
        str_to_js(env, &map_to_json(&slot.headers)));

    // Error message (only present for error handler calls).
    if let Some(ref msg) = slot.error_message {
        set_named_property(env, obj, "conduit.error", str_to_js(env, msg.as_str()));
    }

    obj
}

// ---------------------------------------------------------------------------
// map_to_json — serialize a string→string HashMap as a compact JSON object
// ---------------------------------------------------------------------------
//
// We roll our own instead of pulling in serde_json to keep Cargo.toml minimal.
// The values in these maps come from HTTP headers and URL components — they
// can contain double quotes, backslashes, and control characters, so we
// must escape properly.

fn map_to_json(map: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(map.len() * 32 + 2);
    out.push('{');
    let mut first = true;
    for (k, v) in map {
        if !first { out.push(','); }
        first = false;
        out.push('"');
        push_json_string(&mut out, k);
        out.push_str("\":\"");
        push_json_string(&mut out, v);
        out.push('"');
    }
    out.push('}');
    out
}

fn push_json_string(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"'  => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => buf.push(c),
        }
    }
}

// ---------------------------------------------------------------------------
// parse_response — JS return value → WebResponse
// ---------------------------------------------------------------------------
//
// status=0 sentinel = "undefined/null" (no override from before/after hook).
// TypeScript handlers return: [status, Record<string,string> headers, body].

unsafe fn parse_response(env: napi_env, val: napi_value) -> WebResponse {
    let vtype = value_type(env, val);
    if vtype == NAPI_UNDEFINED || vtype == NAPI_NULL {
        return no_override_response();
    }
    if !is_array(env, val) {
        return WebResponse::internal_error(
            "handler must return [status, headers, body] or undefined",
        );
    }

    // Validate and clamp status to the standard HTTP range 100–599.
    // Security: casting an unchecked i32 to u16 can wrap negative values
    // (e.g. -1 → 65535) and produce invalid status lines.  Status 0 would
    // collide with our is_no_override() sentinel.
    let raw_status = i32_from_js(env, array_get(env, val, 0)).unwrap_or(500);
    let status = if raw_status < 100 || raw_status > 599 { 500u16 } else { raw_status as u16 };

    // headers: the TypeScript helpers return Record<string,string> (a plain
    // JS object).  We enumerate its keys with napi_get_property_names.
    let headers_val = array_get(env, val, 1);
    let mut headers: Vec<(String, String)> = Vec::new();
    let htype = value_type(env, headers_val);
    if htype == NAPI_OBJECT && !is_array(env, headers_val) {
        // Record<string,string> — enumerate with Object.keys()
        let keys = get_property_names(env, headers_val);
        let key_count = array_len(env, keys);
        for i in 0..key_count {
            let key_val = array_get(env, keys, i);
            let val_for_key = get_property_by_key(env, headers_val, key_val);
            if let (Some(k), Some(v)) = (str_from_js(env, key_val), str_from_js(env, val_for_key)) {
                headers.push((k, v));
            }
        }
    } else if is_array(env, headers_val) {
        // Pair-array fallback [[name, value], ...] — kept for forward-compat.
        let hlen = array_len(env, headers_val);
        for i in 0..hlen {
            let pair = array_get(env, headers_val, i);
            if is_array(env, pair) && array_len(env, pair) >= 2 {
                let name  = str_from_js(env, array_get(env, pair, 0)).unwrap_or_default();
                let value = str_from_js(env, array_get(env, pair, 1)).unwrap_or_default();
                headers.push((name, value));
            }
        }
    }

    let body = str_from_js(env, array_get(env, val, 2)).unwrap_or_default();
    WebResponse { status, headers, body: body.into_bytes() }
}

/// Sentinel response: status=0 means "no override" from a before/after hook.
fn no_override_response() -> WebResponse {
    WebResponse { status: 0, headers: vec![], body: vec![] }
}

fn is_no_override(r: &WebResponse) -> bool {
    r.status == 0 && r.headers.is_empty() && r.body.is_empty()
}

// ---------------------------------------------------------------------------
// extract_halt_or_error — inspect a pending JS exception
// ---------------------------------------------------------------------------

unsafe fn extract_halt_or_error(env: napi_env, ex: napi_value) -> WebResponse {
    let is_halt = bool_from_js(env, get_property(env, ex, "__conduit_halt"))
        .unwrap_or(false);

    if is_halt {
        // Clamp to 100–599 for the same reason as parse_response: unchecked
        // cast of negative or out-of-range values wraps to invalid u16 codes.
        let raw_status = i32_from_js(env, get_property(env, ex, "status")).unwrap_or(500);
        let status = if raw_status < 100 || raw_status > 599 { 500u16 } else { raw_status as u16 };
        let body = str_from_js(env, get_property(env, ex, "body"))
            .unwrap_or_default();

        // haltHeaderPairs is [[name,value],...] set by HaltError constructor.
        let pairs_val = get_property(env, ex, "haltHeaderPairs");
        let mut headers: Vec<(String, String)> = Vec::new();
        if is_array(env, pairs_val) {
            let len = array_len(env, pairs_val);
            for i in 0..len {
                let pair = array_get(env, pairs_val, i);
                if is_array(env, pair) && array_len(env, pair) >= 2 {
                    let name  = str_from_js(env, array_get(env, pair, 0)).unwrap_or_default();
                    let value = str_from_js(env, array_get(env, pair, 1)).unwrap_or_default();
                    headers.push((name, value));
                }
            }
        }
        WebResponse { status, headers, body: body.into_bytes() }
    } else {
        let msg = str_from_js(env, get_property(env, ex, "message"))
            .unwrap_or_else(|| "unhandled error in handler".into());
        // Return a bare 500 with NO headers so the route handler loop can
        // distinguish "an exception was thrown" (headers empty, body = message)
        // from "the handler explicitly returned a 500 with headers".
        // WebResponse::internal_error() adds Content-Type, which would defeat
        // the `resp.headers.is_empty()` check used to decide whether to invoke
        // the error handler TSFN.
        WebResponse { status: 500, headers: vec![], body: msg.into_bytes() }
    }
}

// ---------------------------------------------------------------------------
// dispatch_via_tsfn — call a JS handler from a background Rust thread
// ---------------------------------------------------------------------------

/// Maximum time a background Rust thread waits for the V8 event loop to
/// execute a handler and return a response.  If the Node.js main thread is
/// stalled, this prevents request threads from parking forever and exhausting
/// the connection pool.
const HANDLER_TIMEOUT: Duration = Duration::from_secs(30);

fn dispatch_via_tsfn(
    tsfn:          napi_threadsafe_function,
    env_map:       HashMap<String, String>,
    route_params:  HashMap<String, String>,
    query_params:  HashMap<String, String>,
    headers:       HashMap<String, String>,
    error_message: Option<String>,
) -> WebResponse {
    let channel = Arc::new((Mutex::new(None::<WebResponse>), Condvar::new()));
    let slot = Box::new(RequestSlot {
        env_map, route_params, query_params, headers,
        error_message,
        response: Arc::clone(&channel),
    });

    tsfn_call(tsfn, Box::into_raw(slot) as *mut c_void, NAPI_TSFN_BLOCKING);

    // Block until the V8 main thread writes the response or the timeout elapses.
    //
    // SAFETY: even on timeout, the Arc is still held by the RequestSlot, so
    // when call_js_cb eventually fires it can safely write to the Mutex without
    // dangling.  The write is simply discarded.
    let (lock, cvar) = &*channel;
    let mut guard = lock.lock().unwrap();
    let deadline = std::time::Instant::now() + HANDLER_TIMEOUT;
    while guard.is_none() {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return WebResponse::internal_error("handler timeout: V8 event loop did not respond");
        }
        let (g, _timed_out) = cvar.wait_timeout(guard, remaining).unwrap();
        guard = g;
    }
    guard.take().unwrap()
}

// ---------------------------------------------------------------------------
// N-API module entry point
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env:     napi_env,
    exports: napi_value,
) -> napi_value {
    set_named_property(env, exports, "newApp",
        create_function(env, "newApp",    Some(js_new_app)));
    set_named_property(env, exports, "newServer",
        create_function(env, "newServer", Some(js_new_server)));
    exports
}

// ---------------------------------------------------------------------------
// newApp()
// ---------------------------------------------------------------------------

unsafe extern "C" fn js_new_app(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, _) = get_cb_info(env, info, 0);
    let app_obj = object_new(env);
    wrap_data(env, app_obj, NativeApp::new());

    macro_rules! m { ($n:literal, $f:expr) => {
        set_named_property(env, app_obj, $n, create_function(env, $n, Some($f)));
    }}
    m!("addRoute",        js_app_add_route);
    m!("addBefore",       js_app_add_before);
    m!("addAfter",        js_app_add_after);
    m!("setNotFound",     js_app_set_not_found);
    m!("setErrorHandler", js_app_set_error_handler);
    m!("setSetting",      js_app_set_setting);
    m!("getSetting",      js_app_get_setting);

    app_obj
}

unsafe extern "C" fn js_app_add_route(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 3);
    if args.len() < 3 { throw_error(env, "addRoute(method, pattern, fn)"); return undefined(env); }
    let method  = str_from_js(env, args[0]).unwrap_or_default();
    let pattern = str_from_js(env, args[1]).unwrap_or_default();
    let r       = create_ref(env, args[2]);
    (*unwrap_data_mut::<NativeApp>(env, this)).routes.push(RouteEntry { method, pattern, handler_ref: r });
    undefined(env)
}

unsafe extern "C" fn js_app_add_before(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() { throw_error(env, "addBefore(fn)"); return undefined(env); }
    (*unwrap_data_mut::<NativeApp>(env, this)).before_refs.push(create_ref(env, args[0]));
    undefined(env)
}

unsafe extern "C" fn js_app_add_after(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() { throw_error(env, "addAfter(fn)"); return undefined(env); }
    (*unwrap_data_mut::<NativeApp>(env, this)).after_refs.push(create_ref(env, args[0]));
    undefined(env)
}

unsafe extern "C" fn js_app_set_not_found(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() { throw_error(env, "setNotFound(fn)"); return undefined(env); }
    let app = &mut *unwrap_data_mut::<NativeApp>(env, this);
    if let Some(old) = app.not_found_ref.replace(create_ref(env, args[0])) { delete_ref(env, old); }
    undefined(env)
}

unsafe extern "C" fn js_app_set_error_handler(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() { throw_error(env, "setErrorHandler(fn)"); return undefined(env); }
    let app = &mut *unwrap_data_mut::<NativeApp>(env, this);
    if let Some(old) = app.error_handler_ref.replace(create_ref(env, args[0])) { delete_ref(env, old); }
    undefined(env)
}

unsafe extern "C" fn js_app_set_setting(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 { throw_error(env, "setSetting(key, value)"); return undefined(env); }
    let key   = str_from_js(env, args[0]).unwrap_or_default();
    let value = str_from_js(env, args[1]).unwrap_or_default();
    (*unwrap_data_mut::<NativeApp>(env, this)).settings.insert(key, value);
    undefined(env)
}

unsafe extern "C" fn js_app_get_setting(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() { return undefined(env); }
    let key = str_from_js(env, args[0]).unwrap_or_default();
    match (*unwrap_data::<NativeApp>(env, this)).settings.get(&key) {
        Some(v) => str_to_js(env, v),
        None    => undefined(env),
    }
}

// ---------------------------------------------------------------------------
// newServer(appObj, host, port, maxConn)
// ---------------------------------------------------------------------------

unsafe extern "C" fn js_new_server(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 4);
    if args.len() < 4 {
        throw_error(env, "newServer(app, host, port, maxConn)");
        return undefined(env);
    }

    let app_obj  = args[0];
    let host     = str_from_js(env, args[1]).unwrap_or_else(|| "127.0.0.1".into());
    let port     = i32_from_js(env, args[2]).unwrap_or(0) as u16;
    let max_conn = i32_from_js(env, args[3]).unwrap_or(128) as usize;

    let app = &*unwrap_data::<NativeApp>(env, app_obj);

    // Create a TSFN from an napi_ref.
    //
    // We pass null for `func` (the JS function value) because Node.js v25
    // returns napi_invalid_arg when func is provided but call_js_cb is also
    // provided on some platforms.  Instead, we pass the napi_ref itself as
    // the `context` pointer — call_js_cb retrieves the function via deref().
    let make_tsfn = |r: napi_ref, name: &str| -> napi_threadsafe_function {
        let tsfn = tsfn_create(
            env,
            ptr::null_mut(),   // func = null; function is in context instead
            name,
            0,                 // max_queue = unlimited
            1,                 // initial_thread_count
            r as *mut c_void,  // context = napi_ref to the JS handler
            Some(call_js_cb),
        );
        tsfn_ref(env, tsfn);   // keep the event loop alive while TSFN exists
        tsfn
    };

    let before_tsfns: Vec<napi_threadsafe_function> = app.before_refs.iter()
        .enumerate().map(|(i, &r)| make_tsfn(r, &format!("before_{i}"))).collect();
    let after_tsfns: Vec<napi_threadsafe_function> = app.after_refs.iter()
        .enumerate().map(|(i, &r)| make_tsfn(r, &format!("after_{i}"))).collect();
    let not_found_tsfn     = app.not_found_ref.map(|r| make_tsfn(r, "not_found"));
    let error_handler_tsfn = app.error_handler_ref.map(|r| make_tsfn(r, "error_handler"));
    let route_tsfns: Vec<RouteTsfn> = app.routes.iter().map(|rt| {
        RouteTsfn {
            method:  rt.method.clone(),
            pattern: rt.pattern.clone(),
            tsfn:    make_tsfn(rt.handler_ref, &format!("route_{}_{}", rt.method, rt.pattern)),
        }
    }).collect();

    // Build WebApp — register all routes and hooks.
    let mut web_app = WebApp::new();

    for &tsfn in &before_tsfns {
        let t = ThreadSafePtr(tsfn);
        web_app.before_routing(move |req| {
            let (em, rp, qp, hd) = build_env_map(req);
            let resp = dispatch_via_tsfn(t.get(), em, rp, qp, hd, None);
            if is_no_override(&resp) { None } else { Some(resp) }
        });
    }

    for rt in &route_tsfns {
        let t  = ThreadSafePtr(rt.tsfn);
        let eh = error_handler_tsfn.map(ThreadSafePtr);
        web_app.add(&rt.method, &rt.pattern, move |req| {
            let (em, rp, qp, hd) = build_env_map(req);
            let resp = dispatch_via_tsfn(t.get(), em.clone(), rp.clone(), qp.clone(), hd.clone(), None);
            // A 500 with no headers is our "unhandled error" sentinel from
            // extract_halt_or_error.  Re-dispatch to the error handler with
            // the error message encoded in conduit.error.
            if resp.status == 500 && resp.headers.is_empty() {
                if let Some(ref e) = eh {
                    let err_msg = String::from_utf8_lossy(&resp.body).into_owned();
                    return dispatch_via_tsfn(e.get(), em, rp, qp, hd, Some(err_msg));
                }
            }
            resp
        });
    }

    for &tsfn in &after_tsfns {
        let t = ThreadSafePtr(tsfn);
        web_app.after_handler(move |req, resp| {
            let (em, rp, qp, hd) = build_env_map(req);
            let new_resp = dispatch_via_tsfn(t.get(), em, rp, qp, hd, None);
            if is_no_override(&new_resp) { resp } else { new_resp }
        });
    }

    if let Some(nf) = not_found_tsfn {
        let t = ThreadSafePtr(nf);
        web_app.on_not_found(move |req| {
            let (em, rp, qp, hd) = build_env_map(req);
            let resp = dispatch_via_tsfn(t.get(), em, rp, qp, hd, None);
            if is_no_override(&resp) { WebResponse::not_found() } else { resp }
        });
    }

    if let Some(eh) = error_handler_tsfn {
        let t = ThreadSafePtr(eh);
        web_app.on_handler_error(move |req, msg| {
            let (em, rp, qp, hd) = build_env_map(req);
            let resp = dispatch_via_tsfn(t.get(), em, rp, qp, hd, Some(msg.to_string()));
            if is_no_override(&resp) {
                WebResponse::internal_error("internal server error")
            } else { resp }
        });
    }

    // Bind the TCP socket. Capture local_port and stop_handle before serve().
    let mut opts = embeddable_http_server::HttpServerOptions::default();
    opts.tcp.max_connections = max_conn;
    let server = match bind_server(&host, port, opts, Arc::new(web_app)) {
        Ok(s)  => s,
        Err(e) => {
            throw_error(env, &format!("conduit: bind failed: {e}"));
            return undefined(env);
        }
    };

    let local_port  = server.local_addr().port();
    let stop_handle = server.stop_handle();
    let running     = Arc::new(AtomicBool::new(false));

    let native_server = NativeServer {
        env,
        local_port,
        stop_handle,
        route_tsfns,
        before_tsfns,
        after_tsfns,
        not_found_tsfn,
        error_handler_tsfn,
        server:         Some(server),
        running,
        bg_thread:      None,
        tsfns_released: false,
    };

    let srv_obj = object_new(env);
    wrap_data(env, srv_obj, native_server);

    macro_rules! m { ($n:literal, $f:expr) => {
        set_named_property(env, srv_obj, $n, create_function(env, $n, Some($f)));
    }}
    m!("serve",           js_server_serve);
    m!("serveBackground", js_server_serve_background);
    m!("stop",            js_server_stop);
    m!("localPort",       js_server_local_port);
    m!("running",         js_server_running);

    srv_obj
}

// ---------------------------------------------------------------------------
// Server method implementations
// ---------------------------------------------------------------------------

unsafe extern "C" fn js_server_serve(env: napi_env, info: napi_callback_info) -> napi_value {
    // In Node.js, serve() cannot block the V8 thread — that would prevent the
    // event loop from processing the TSFN callbacks (deadlock). Both serve()
    // and serveBackground() start the server on a background Rust thread.
    // The process stays alive because the TsFns are napi_ref'd.
    js_server_serve_background(env, info)
}

unsafe extern "C" fn js_server_serve_background(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &mut *unwrap_data_mut::<NativeServer>(env, this);

    if srv.running.load(Ordering::SeqCst) { return undefined(env); }

    let mut server = match srv.server.take() {
        Some(s) => s,
        None => {
            throw_error(env, "conduit: server already started");
            return undefined(env);
        }
    };

    let running = Arc::clone(&srv.running);
    running.store(true, Ordering::SeqCst);

    srv.bg_thread = Some(std::thread::spawn(move || {
        let _ = server.serve();
        running.store(false, Ordering::SeqCst);
    }));

    undefined(env)
}

unsafe extern "C" fn js_server_stop(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &mut *unwrap_data_mut::<NativeServer>(env, this);

    // Signal the background serve thread to exit.
    srv.stop_handle.stop();

    // Join the serve thread.
    if let Some(handle) = srv.bg_thread.take() {
        let _ = handle.join();
    }
    srv.running.store(false, Ordering::SeqCst);

    // Release TsFns so the event loop can exit.
    do_release_tsfns(env, srv);

    undefined(env)
}

unsafe extern "C" fn js_server_local_port(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &*unwrap_data::<NativeServer>(env, this);
    usize_to_js(env, srv.local_port as usize)
}

unsafe extern "C" fn js_server_running(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &*unwrap_data::<NativeServer>(env, this);
    bool_to_js(env, srv.running.load(Ordering::SeqCst))
}

// ---------------------------------------------------------------------------
// finalize_server — called by N-API GC when the JS server object is collected
// ---------------------------------------------------------------------------

// SAFETY — ordering justification (security review Finding 4):
// 1. stop_handle.stop() signals the background thread to unwind.
// 2. bg_thread.join() waits until the thread has fully exited.
//    After join(), the background thread no longer accesses any NativeServer
//    fields.  Critically, background closures only hold ThreadSafePtr values
//    (stack copies of the napi_threadsafe_function handle, an opaque N-API
//    pointer) — they do NOT hold raw pointers back into NativeServer.  So
//    join() guarantees zero concurrent access before step 3.
// 3. do_release_tsfns() unrefs and releases the TsFns — safe because the
//    background thread is guaranteed gone by step 2.
// 4. Box::from_raw(data) drops the NativeServer — safe because all users
//    of its fields have been shut down in the previous steps.
#[allow(dead_code)]
unsafe extern "C" fn finalize_server(env: napi_env, data: *mut c_void, _hint: *mut c_void) {
    if data.is_null() { return; }
    let srv = &mut *(data as *mut NativeServer);

    // Step 1: signal background thread.
    srv.stop_handle.stop();
    // Step 2: wait for background thread — no concurrent access after this.
    if let Some(handle) = srv.bg_thread.take() {
        let _ = handle.join();
    }
    // Step 3: release N-API TsFn handles — safe only after join().
    do_release_tsfns(env, srv);

    // Step 4: free the NativeServer allocation.
    let _ = Box::from_raw(data as *mut NativeServer);
}

unsafe fn do_release_tsfns(env: napi_env, srv: &mut NativeServer) {
    if srv.tsfns_released { return; }
    srv.tsfns_released = true;

    let all: Vec<napi_threadsafe_function> = srv.route_tsfns.iter().map(|r| r.tsfn)
        .chain(srv.before_tsfns.iter().copied())
        .chain(srv.after_tsfns.iter().copied())
        .chain(srv.not_found_tsfn)
        .chain(srv.error_handler_tsfn)
        .collect();

    for tsfn in all {
        tsfn_unref(env, tsfn);
        tsfn_release(tsfn, NAPI_TSFN_RELEASE);
    }

    srv.route_tsfns.clear();
    srv.before_tsfns.clear();
    srv.after_tsfns.clear();
    srv.not_found_tsfn      = None;
    srv.error_handler_tsfn  = None;
}
