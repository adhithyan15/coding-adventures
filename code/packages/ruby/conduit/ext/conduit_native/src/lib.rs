use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use embeddable_http_server::{HttpRequest, HttpResponse, HttpServer, HttpServerOptions};
use http_core::Header;
use ruby_bridge::{ID, VALUE};

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

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type PlatformHttpServer = HttpServer<transport_platform::bsd::KqueueTransportPlatform>;

#[cfg(target_os = "linux")]
type PlatformHttpServer = HttpServer<transport_platform::linux::EpollTransportPlatform>;

#[cfg(target_os = "windows")]
type PlatformHttpServer = HttpServer<transport_platform::windows::WindowsTransportPlatform>;

struct RubyConduitServer {
    server: Option<PlatformHttpServer>,
    owner: VALUE,
    running: Arc<AtomicBool>,
}

struct ServeCall {
    server: *mut RubyConduitServer,
    ok: bool,
    error: Option<String>,
}

struct RubyDispatch {
    owner: VALUE,
    request: HttpRequest,
    response: Option<Result<HttpResponse, String>>,
}

struct ProtectedDispatch {
    owner: VALUE,
    env: VALUE,
    result: VALUE,
}

static mut NATIVE_SERVER_CLASS: VALUE = 0;
static mut SERVER_ERROR: VALUE = 0;

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

extern "C" fn server_initialize(
    self_val: VALUE,
    host_val: VALUE,
    port_val: VALUE,
    max_connections_val: VALUE,
) -> VALUE {
    let host = string_from_rb(host_val, "host must be a String");
    let port = u16_from_rb(port_val, "port must be between 0 and 65535");
    let max_connections =
        usize_from_rb(max_connections_val, "max_connections must be non-negative");

    let mut options = HttpServerOptions::default();
    options.tcp.max_connections = max_connections;

    let owner = self_val;
    let running = {
        let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
        slot.owner = owner;
        Arc::clone(&slot.running)
    };

    let server = match bind_server(&host, port, options, move |request| {
        dispatch_http_request(owner, request)
    }) {
        Ok(server) => server,
        Err(error) => raise_server_error(&format!("failed to start Conduit HTTP runtime: {error}")),
    };

    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
    slot.server = Some(server);
    slot.running = running;
    self_val
}

extern "C" fn server_serve(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data_mut::<RubyConduitServer>(self_val) };
    if slot.server.is_none() {
        raise_server_error("server is closed");
    }
    let mut call = ServeCall {
        server: slot as *mut RubyConduitServer,
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
        let message = call
            .error
            .unwrap_or_else(|| "Conduit HTTP runtime failed".to_string());
        raise_server_error(&message)
    }
}

extern "C" fn server_stop(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    match slot.server.as_ref() {
        Some(server) => server.stop_handle().stop(),
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
    let server = slot
        .server
        .as_ref()
        .unwrap_or_else(|| raise_server_error("server is closed"));
    ruby_bridge::str_to_rb(&server.local_addr().ip().to_string())
}

extern "C" fn server_local_port(self_val: VALUE) -> VALUE {
    let slot = unsafe { ruby_bridge::unwrap_data::<RubyConduitServer>(self_val) };
    let server = slot
        .server
        .as_ref()
        .unwrap_or_else(|| raise_server_error("server is closed"));
    ruby_bridge::usize_to_rb(server.local_addr().port() as usize)
}

unsafe extern "C" fn serve_without_gvl(data: *mut c_void) -> *mut c_void {
    let call = &mut *(data as *mut ServeCall);
    let slot = &mut *call.server;
    let Some(server) = slot.server.as_mut() else {
        call.ok = false;
        call.error = Some("server is closed".to_string());
        return ptr::null_mut();
    };

    slot.running.store(true, Ordering::SeqCst);
    let result = server.serve();
    slot.running.store(false, Ordering::SeqCst);
    match result {
        Ok(()) => {
            call.ok = true;
        }
        Err(error) => {
            call.ok = false;
            call.error = Some(format!("Conduit HTTP runtime failed: {error}"));
        }
    }
    ptr::null_mut()
}

fn dispatch_http_request(owner: VALUE, request: HttpRequest) -> HttpResponse {
    let mut dispatch = RubyDispatch {
        owner,
        request,
        response: None,
    };

    unsafe {
        rb_thread_call_with_gvl(
            dispatch_with_gvl,
            &mut dispatch as *mut RubyDispatch as *mut c_void,
        );
    }

    match dispatch
        .response
        .take()
        .unwrap_or_else(|| Err("Conduit dispatch did not return a response".to_string()))
    {
        Ok(response) => response,
        Err(message) => HttpResponse::new(500, message).with_header("Content-Type", "text/plain"),
    }
}

unsafe extern "C" fn dispatch_with_gvl(data: *mut c_void) -> *mut c_void {
    let dispatch = &mut *(data as *mut RubyDispatch);
    let env = build_env(&dispatch.request);
    let mut protected = ProtectedDispatch {
        owner: dispatch.owner,
        env,
        result: ruby_bridge::QNIL,
    };
    let mut state = 0;
    let result = rb_protect(
        protected_dispatch,
        &mut protected as *mut ProtectedDispatch as VALUE,
        &mut state,
    );

    if state != 0 {
        let error = rb_errinfo();
        rb_set_errinfo(ruby_bridge::QNIL);
        let _ = error;
        dispatch.response = Some(Err("Conduit app raised an exception".to_string()));
        return ptr::null_mut();
    }

    protected.result = result;
    dispatch.response = Some(parse_response(result));
    ptr::null_mut()
}

unsafe extern "C" fn protected_dispatch(data: VALUE) -> VALUE {
    let protected = &mut *(data as *mut ProtectedDispatch);
    let mid = intern("dispatch_request");
    let args = [protected.env];
    ruby_bridge::rb_funcallv(protected.owner, mid, 1, args.as_ptr())
}

fn build_env(request: &HttpRequest) -> VALUE {
    let env = ruby_bridge::hash_new();
    set_hash_str(&env, "REQUEST_METHOD", request.method());
    let (path, query) = split_target(request.target());
    set_hash_str(&env, "PATH_INFO", path);
    set_hash_str(&env, "QUERY_STRING", query);
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("conduit.query_params"),
        build_query_params_hash(query),
    );
    let headers_hash = build_headers_hash(&request.head.headers);
    ruby_bridge::hash_aset(env, ruby_bridge::str_to_rb("conduit.headers"), headers_hash);
    set_hash_str(
        &env,
        "SERVER_PROTOCOL",
        &format!(
            "HTTP/{}.{}",
            request.head.version.major, request.head.version.minor
        ),
    );
    set_hash_str(&env, "rack.url_scheme", "http");
    set_hash_str(&env, "rack.input", &String::from_utf8_lossy(&request.body));
    set_hash_str(
        &env,
        "REMOTE_ADDR",
        &request.connection.peer_addr.ip().to_string(),
    );
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("REMOTE_PORT"),
        ruby_bridge::usize_to_rb(request.connection.peer_addr.port() as usize),
    );
    set_hash_str(
        &env,
        "SERVER_NAME",
        &request.connection.local_addr.ip().to_string(),
    );
    ruby_bridge::hash_aset(
        env,
        ruby_bridge::str_to_rb("SERVER_PORT"),
        ruby_bridge::usize_to_rb(request.connection.local_addr.port() as usize),
    );
    if let Some(content_length) = request.head.content_length() {
        ruby_bridge::hash_aset(
            env,
            ruby_bridge::str_to_rb("conduit.content_length"),
            ruby_bridge::usize_to_rb(content_length),
        );
    }
    if let Some((content_type, _charset)) = request.head.content_type() {
        ruby_bridge::hash_aset(
            env,
            ruby_bridge::str_to_rb("conduit.content_type"),
            ruby_bridge::str_to_rb(&content_type),
        );
    }

    for header in &request.head.headers {
        let key = header_env_key(&header.name);
        set_hash_str(&env, &key, &header.value);
    }

    env
}

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

fn build_query_params_hash(query: &str) -> VALUE {
    let hash = ruby_bridge::hash_new();
    if query.is_empty() {
        return hash;
    }

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = match pair.split_once('=') {
            Some((key, value)) => (key, value),
            None => (pair, ""),
        };

        let key = percent_decode_component(raw_key);
        if key.is_empty() {
            continue;
        }
        let value = percent_decode_component(raw_value);
        ruby_bridge::hash_aset(
            hash,
            ruby_bridge::str_to_rb(&key),
            ruby_bridge::str_to_rb(&value),
        );
    }

    hash
}

fn percent_decode_component(input: &str) -> String {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_nibble(bytes[index + 1]), hex_nibble(bytes[index + 2]))
                {
                    output.push(high << 4 | low);
                    index += 3;
                } else {
                    output.push(b'%');
                    index += 1;
                }
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn header_env_key(name: &str) -> String {
    let normalized = name.replace('-', "_").to_ascii_uppercase();
    match normalized.as_str() {
        "CONTENT_TYPE" | "CONTENT_LENGTH" => normalized,
        _ => format!("HTTP_{normalized}"),
    }
}

fn parse_response(value: VALUE) -> Result<HttpResponse, String> {
    if ruby_bridge::array_len(value) != 3 {
        return Err("Conduit app must return [status, headers, body]".to_string());
    }

    let status = u16_from_rb_result(
        ruby_bridge::array_entry(value, 0),
        "response status must be between 0 and 65535",
    )?;
    let headers = parse_header_pairs(ruby_bridge::array_entry(value, 1))?;
    let body = parse_body_chunks(ruby_bridge::array_entry(value, 2))?;

    Ok(HttpResponse {
        status,
        reason: String::new(),
        headers,
        body,
        close: false,
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
        let value = string_from_rb_result(
            ruby_bridge::array_entry(pair, 1),
            "response header value must be a String",
        )?;
        headers.push(Header { name, value });
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

fn string_from_rb(value: VALUE, message: &str) -> String {
    match ruby_bridge::str_from_rb(value) {
        Some(value) => value,
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
    options: HttpServerOptions,
    handler: impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
) -> Result<PlatformHttpServer, transport_platform::PlatformError> {
    PlatformHttpServer::bind_kqueue((host, port), options, handler)
}

#[cfg(target_os = "linux")]
fn bind_server(
    host: &str,
    port: u16,
    options: HttpServerOptions,
    handler: impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
) -> Result<PlatformHttpServer, transport_platform::PlatformError> {
    PlatformHttpServer::bind_epoll((host, port), options, handler)
}

#[cfg(target_os = "windows")]
fn bind_server(
    host: &str,
    port: u16,
    options: HttpServerOptions,
    handler: impl Fn(HttpRequest) -> HttpResponse + Send + Sync + 'static,
) -> Result<PlatformHttpServer, transport_platform::PlatformError> {
    PlatformHttpServer::bind_windows((host, port), options, handler)
}

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
        3,
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
