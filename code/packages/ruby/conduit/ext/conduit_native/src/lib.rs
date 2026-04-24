/// Conduit native extension — Phase 2
///
/// Routing now lives entirely in Rust inside a `WebApp`. Ruby only supplies
/// handler blocks. When a route matches, Rust calls back to
/// `NativeServer#native_dispatch_route(route_index, env)` with a pre-built
/// Rack env hash; Ruby executes the block and returns `[status, headers, body]`.
///
/// Connection management (TCP sockets, HTTP framing, event-loop) remains in
/// `web-core` → `embeddable-http-server` → `tcp-runtime`.

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use http_core::Header;
use ruby_bridge::{ID, VALUE};
use web_core::{WebApp, WebRequest, WebResponse, WebServer};

extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_thread_call_without_gvl(
        func: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        data1: *mut c_void,
        ubf: Option<unsafe extern "C" fn(*mut c_void)>,
        data2: *mut c_void,
    ) -> *mut c_void;
    fn rb_thread_call_with_gvl(
        func: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        data1: *mut c_void,
    ) -> *mut c_void;
    fn rb_protect(
        func: unsafe extern "C" fn(VALUE) -> VALUE,
        data: VALUE,
        state: *mut c_int,
    ) -> VALUE;
    fn rb_errinfo() -> VALUE;
    fn rb_set_errinfo(error: VALUE);
}

// Platform alias — kqueue on BSD/macOS, epoll on Linux, IOCP on Windows.

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

// --- Rust-side server state ---

struct RubyConduitServer {
    server: Option<PlatformWebServer>,
    owner: VALUE,
    running: Arc<AtomicBool>,
}

struct ServeCall {
    server: *mut PlatformWebServer,
    running: Arc<AtomicBool>,
    ok: bool,
    error: Option<String>,
}

// Data packet threaded through rb_protect for route dispatch.
struct ProtectedRouteDispatch {
    owner: VALUE,
    route_index: VALUE, // Ruby integer — index into app.routes
    env: VALUE,         // Ruby hash — the Rack env
}

static mut NATIVE_SERVER_CLASS: VALUE = 0;
static mut SERVER_ERROR: VALUE = 0;

// --- Ruby-object lifecycle ---

unsafe extern "C" fn server_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(
        klass,
        RubyConduitServer {
            server: None,
            owner: ruby_bridge::QNIL,
            running: Arc::new(AtomicBool::new(false)),
        },
    )
}

// NativeServer.new(app, host, port, max_connections)
//
// Iterates `app.routes` and registers every route in a fresh `WebApp`.
// Each route handler calls back to Ruby's `native_dispatch_route`.
extern "C" fn server_initialize(
    self_val: VALUE,
    app_val: VALUE,
    host_val: VALUE,
    port_val: VALUE,
    max_connections_val: VALUE,
) -> VALUE {
    let host = string_from_rb(host_val, "host must be a String");
    let port = u16_from_rb(port_val, "port must be between 0 and 65535");
    let max_connections =
        usize_from_rb(max_connections_val, "max_connections must be non-negative");
    let owner = self_val;

    // Iterate app.routes to register each route in the WebApp.
    let routes_val = unsafe {
        let mid = intern("routes");
        ruby_bridge::rb_funcallv(app_val, mid, 0, ptr::null())
    };
    let route_count = ruby_bridge::array_len(routes_val);

    let mut web_app = WebApp::new();

    for i in 0..route_count {
        let route_val = ruby_bridge::array_entry(routes_val, i);

        let method = {
            let mid = intern("method");
            let v = unsafe { ruby_bridge::rb_funcallv(route_val, mid, 0, ptr::null()) };
            string_from_rb(v, "route method must be a String")
        };
        let pattern = {
            let mid = intern("pattern");
            let v = unsafe { ruby_bridge::rb_funcallv(route_val, mid, 0, ptr::null()) };
            string_from_rb(v, "route pattern must be a String")
        };

        // The closure captures owner (VALUE = usize, which is Send+Sync) and i.
        let owner_cap = owner;
        let index_cap = i;
        web_app.add(&method, &pattern, move |req: &WebRequest| {
            dispatch_route_to_ruby(owner_cap, index_cap, req)
        });
    }

    // Store the running flag before moving web_app into the server.
    let running = {
        let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
        slot.owner = owner;
        Arc::clone(&slot.running)
    };

    let mut options = embeddable_http_server::HttpServerOptions::default();
    options.tcp.max_connections = max_connections;

    let web_app = Arc::new(web_app);
    let server = match bind_server(&host, port, options, web_app) {
        Ok(s) => s,
        Err(e) => raise_server_error(&format!("failed to start Conduit server: {e}")),
    };

    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
    slot.server = Some(server);
    slot.running = running;
    self_val
}

// --- Serve / stop / dispose ---

extern "C" fn server_serve(self_val: VALUE) -> VALUE {
    let (server, running) = {
        let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
        let server = match slot.server.as_mut() {
            Some(s) => s as *mut PlatformWebServer,
            None => raise_server_error("server is closed"),
        };
        (server, Arc::clone(&slot.running))
    };
    let mut call = ServeCall {
        server,
        running,
        ok: false,
        error: None,
    };

    unsafe {
        rb_thread_call_without_gvl(
            serve_without_gvl,
            &mut call as *mut ServeCall as *mut c_void,
            None,
            ptr::null_mut(),
        );
    }

    if call.ok {
        ruby_bridge::QNIL
    } else {
        let message = call.error.unwrap_or_else(|| "Conduit server failed".to_string());
        raise_server_error(&message)
    }
}

extern "C" fn server_stop(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    match slot.server.as_ref() {
        Some(s) => s.stop_handle().stop(),
        None => raise_server_error("server is closed"),
    }
    ruby_bridge::QNIL
}

extern "C" fn server_dispose(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
    if slot.running.load(Ordering::SeqCst) {
        raise_server_error("cannot dispose a running server; stop and wait first");
    }
    slot.server.take();
    slot.owner = ruby_bridge::QNIL;
    ruby_bridge::QNIL
}

extern "C" fn server_running(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    ruby_bridge::bool_to_rb(slot.running.load(Ordering::SeqCst))
}

extern "C" fn server_local_host(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    let s = slot.server.as_ref().unwrap_or_else(|| raise_server_error("server is closed"));
    ruby_bridge::str_to_rb(&s.local_addr().ip().to_string())
}

extern "C" fn server_local_port(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    let s = slot.server.as_ref().unwrap_or_else(|| raise_server_error("server is closed"));
    ruby_bridge::usize_to_rb(s.local_addr().port() as usize)
}

unsafe extern "C" fn serve_without_gvl(data: *mut c_void) -> *mut c_void {
    let call = &mut *(data as *mut ServeCall);
    call.running.store(true, Ordering::SeqCst);
    let result = (*call.server).serve();
    call.running.store(false, Ordering::SeqCst);
    match result {
        Ok(()) => call.ok = true,
        Err(e) => {
            call.ok = false;
            call.error = Some(format!("Conduit server failed: {e}"));
        }
    }
    ptr::null_mut()
}

// --- Route dispatch back to Ruby ---

struct RouteDispatchCall {
    owner: VALUE,
    route_index: usize,
    request: WebRequest,
    response: Option<Result<WebResponse, String>>,
}

fn dispatch_route_to_ruby(owner: VALUE, route_index: usize, req: &WebRequest) -> WebResponse {
    let mut call = RouteDispatchCall {
        owner,
        route_index,
        request: req.clone(),
        response: None,
    };

    unsafe {
        rb_thread_call_with_gvl(
            dispatch_route_with_gvl,
            &mut call as *mut RouteDispatchCall as *mut c_void,
        );
    }

    match call.response.take().unwrap_or_else(|| Err("no response from route".to_string())) {
        Ok(r) => r,
        Err(msg) => WebResponse::internal_error(msg),
    }
}

unsafe extern "C" fn dispatch_route_with_gvl(data: *mut c_void) -> *mut c_void {
    let call = &mut *(data as *mut RouteDispatchCall);
    let env = build_env(&call.request);
    let route_index_rb = ruby_bridge::usize_to_rb(call.route_index);

    let mut protected = ProtectedRouteDispatch {
        owner: call.owner,
        route_index: route_index_rb,
        env,
    };
    let mut state = 0;
    let result = rb_protect(
        protected_route_dispatch,
        &mut protected as *mut ProtectedRouteDispatch as VALUE,
        &mut state,
    );

    if state != 0 {
        let error = rb_errinfo();
        rb_set_errinfo(ruby_bridge::QNIL);
        let _ = error;
        call.response = Some(Err("route handler raised an exception".to_string()));
        return ptr::null_mut();
    }

    call.response = Some(parse_web_response(result));
    ptr::null_mut()
}

unsafe extern "C" fn protected_route_dispatch(data: VALUE) -> VALUE {
    let protected = &mut *(data as *mut ProtectedRouteDispatch);
    let mid = intern("native_dispatch_route");
    let args = [protected.route_index, protected.env];
    ruby_bridge::rb_funcallv(protected.owner, mid, 2, args.as_ptr())
}

// --- Env hash builder ---

fn build_env(request: &WebRequest) -> VALUE {
    let env = ruby_bridge::hash_new();

    set_hash_str(&env, "REQUEST_METHOD", request.method());
    set_hash_str(&env, "PATH_INFO", request.path());

    // Reconstruct the original query string from the HTTP target.
    let (_, query) = split_target(request.http.target());
    set_hash_str(&env, "QUERY_STRING", query);

    // Pre-parsed query params from web-core (avoids re-parsing in Ruby).
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("conduit.query_params"),
        map_to_rb_hash(&request.query_params),
    );

    // Route params injected by web-core's router.
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("conduit.route_params"),
        map_to_rb_hash(&request.route_params),
    );

    let headers_hash = build_headers_hash(&request.http.head.headers);
    ruby_bridge::hash_aset(env, ruby_bridge::str_to_rb("conduit.headers"), headers_hash);

    set_hash_str(
        &env,
        "SERVER_PROTOCOL",
        &format!(
            "HTTP/{}.{}",
            request.http.head.version.major, request.http.head.version.minor
        ),
    );
    set_hash_str(&env, "rack.url_scheme", "http");
    set_hash_str(&env, "rack.input", &String::from_utf8_lossy(request.body()));
    set_hash_str(&env, "REMOTE_ADDR", &request.peer_addr().ip().to_string());
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("REMOTE_PORT"),
        ruby_bridge::usize_to_rb(request.peer_addr().port() as usize),
    );
    set_hash_str(
        &env,
        "SERVER_NAME",
        &request.http.connection.local_addr.ip().to_string(),
    );
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("SERVER_PORT"),
        ruby_bridge::usize_to_rb(request.http.connection.local_addr.port() as usize),
    );

    if let Some(content_length) = request.content_length() {
        ruby_bridge::hash_aset(
            env,
            ruby_bridge::str_to_rb("conduit.content_length"),
            ruby_bridge::usize_to_rb(content_length),
        );
    }
    if let Some(ct) = request.content_type() {
        ruby_bridge::hash_aset(
            env,
            ruby_bridge::str_to_rb("conduit.content_type"),
            ruby_bridge::str_to_rb(ct),
        );
    }

    for header in &request.http.head.headers {
        let key = header_env_key(&header.name);
        set_hash_str(&env, &key, &header.value);
    }

    env
}

// --- Response parsers ---

fn parse_web_response(value: VALUE) -> Result<WebResponse, String> {
    if ruby_bridge::array_len(value) != 3 {
        return Err("Conduit app must return [status, headers, body]".to_string());
    }

    let status = u16_from_rb_result(
        ruby_bridge::array_entry(value, 0),
        "response status must be between 0 and 65535",
    )?;
    let headers = parse_header_pairs(ruby_bridge::array_entry(value, 1))?;
    let body = parse_body_chunks(ruby_bridge::array_entry(value, 2))?;

    Ok(WebResponse {
        status,
        headers: headers.into_iter().map(|h| (h.name, h.value)).collect(),
        body,
    })
}

fn parse_header_pairs(value: VALUE) -> Result<Vec<Header>, String> {
    let mut headers = Vec::new();
    let len = ruby_bridge::array_len(value);
    for index in 0..len {
        let pair = ruby_bridge::array_entry(value, index);
        if ruby_bridge::array_len(pair) != 2 {
            return Err("response headers must be [name, value] pairs".to_string());
        }
        let name = string_from_rb_result(
            ruby_bridge::array_entry(pair, 0),
            "response header name must be a String",
        )?;
        let val = string_from_rb_result(
            ruby_bridge::array_entry(pair, 1),
            "response header value must be a String",
        )?;
        headers.push(Header { name, value: val });
    }
    Ok(headers)
}

fn parse_body_chunks(value: VALUE) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let len = ruby_bridge::array_len(value);
    for index in 0..len {
        let chunk = string_from_rb_result(
            ruby_bridge::array_entry(value, index),
            "response body chunks must be Strings",
        )?;
        body.extend_from_slice(chunk.as_bytes());
    }
    Ok(body)
}

// --- Utility helpers ---

fn set_hash_str(hash: &VALUE, key: &str, value: &str) {
    ruby_bridge::hash_aset(
        *hash,
        ruby_bridge::str_to_rb(key),
        ruby_bridge::str_to_rb(value),
    );
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    }
}

fn map_to_rb_hash(map: &std::collections::HashMap<String, String>) -> VALUE {
    let hash = ruby_bridge::hash_new();
    for (key, value) in map {
        ruby_bridge::hash_aset(
            hash,
            ruby_bridge::str_to_rb(key),
            ruby_bridge::str_to_rb(value),
        );
    }
    hash
}

fn build_headers_hash(headers: &[Header]) -> VALUE {
    let hash = ruby_bridge::hash_new();
    for header in headers {
        ruby_bridge::hash_aset(
            hash,
            ruby_bridge::str_to_rb(&header.name.to_ascii_lowercase()),
            ruby_bridge::str_to_rb(&header.value),
        );
    }
    hash
}

fn string_from_rb(value: VALUE, message: &str) -> String {
    match ruby_bridge::str_from_rb(value) {
        Some(v) => v,
        None => raise_arg_error(message),
    }
}

fn string_from_rb_result(value: VALUE, message: &str) -> Result<String, String> {
    ruby_bridge::str_from_rb(value).ok_or_else(|| message.to_string())
}

fn usize_from_rb(value: VALUE, message: &str) -> usize {
    let number = unsafe { rb_num2long(value) };
    if number < 0 {
        raise_arg_error(message);
    }
    number as usize
}

fn u16_from_rb(value: VALUE, message: &str) -> u16 {
    let number = usize_from_rb(value, message);
    if number > u16::MAX as usize {
        raise_arg_error(message);
    }
    number as u16
}

fn u16_from_rb_result(value: VALUE, message: &str) -> Result<u16, String> {
    let number = unsafe { rb_num2long(value) };
    if number < 0 || number > u16::MAX as c_long {
        return Err(message.to_string());
    }
    Ok(number as u16)
}

fn header_env_key(name: &str) -> String {
    let normalized = name.replace('-', "_").to_ascii_uppercase();
    match normalized.as_str() {
        "CONTENT_TYPE" | "CONTENT_LENGTH" => normalized,
        _ => format!("HTTP_{normalized}"),
    }
}

fn raise_arg_error(message: &str) -> ! {
    ruby_bridge::raise_error(ruby_bridge::path2class("ArgumentError"), message)
}

fn raise_server_error(message: &str) -> ! {
    ruby_bridge::raise_error(unsafe { SERVER_ERROR }, message)
}

fn intern(name: &str) -> ID {
    let c_name = CString::new(name).expect("method name must not contain NUL");
    unsafe { ruby_bridge::rb_intern(c_name.as_ptr() as *const c_char) }
}

// --- Platform-specific server binding ---

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

// --- Extension entry point ---

#[no_mangle]
pub extern "C" fn Init_conduit_native() {
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let conduit = ruby_bridge::define_module_under(coding_adventures, "Conduit");

    let error_class = ruby_bridge::define_class_under(
        conduit,
        "ServerError",
        ruby_bridge::standard_error_class(),
    );
    unsafe { SERVER_ERROR = error_class };

    let server_class =
        ruby_bridge::define_class_under(conduit, "NativeServer", ruby_bridge::object_class());
    unsafe { NATIVE_SERVER_CLASS = server_class };

    ruby_bridge::define_alloc_func(server_class, server_alloc);
    ruby_bridge::define_method_raw(
        server_class,
        "initialize",
        server_initialize as *const c_void,
        4, // app, host, port, max_connections
    );
    ruby_bridge::define_method_raw(server_class, "serve", server_serve as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "stop", server_stop as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "dispose", server_dispose as *const c_void, 0);
    ruby_bridge::define_method_raw(server_class, "running?", server_running as *const c_void, 0);
    ruby_bridge::define_method_raw(
        server_class,
        "local_host",
        server_local_host as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        server_class,
        "local_port",
        server_local_port as *const c_void,
        0,
    );
}
