// lib.rs — conduit_native (Elixir/Erlang NIF backend for Conduit)
//
// # What this is
//
// A Rust cdylib loaded by the BEAM (Erlang's VM) via `:erlang.load_nif/2`
// from the Elixir side (`Conduit.Native.load_nif/0`). It bridges the
// Elixir DSL to the same `web-core` engine that powers the Ruby, Python,
// Lua, and TypeScript Conduit ports. The HTTP I/O loop runs on a Rust
// thread; route handlers run as Elixir functions called via BEAM message
// passing.
//
// # Threading model — the BEAM-flavored TSFN dance
//
// BEAM differs from CPython/Lua/Ruby in two ways that matter here:
//
//   1. There is no global lock. Multiple schedulers run on multiple OS
//      threads simultaneously, each pinned to a different BEAM process.
//      A NIF runs on whichever scheduler thread happened to call it.
//
//   2. You cannot `call` an Elixir function from C. You can only `send`
//      a message to a process and let the BEAM scheduler dispatch it.
//
// So our cross-thread dispatch must look like:
//
//     ┌─ Rust I/O thread (on a dirty I/O scheduler) ─────────────┐
//     │  request arrives                                          │
//     │  allocate Slot { Mutex<Option<Response>>, Condvar }       │
//     │  insert into SLOTS table at slot_id                       │
//     │  enif_alloc_env, build env map term                       │
//     │  enif_send(NULL, dispatcher_pid, msg_env, message)        │
//     │  msg = {:conduit_request, slot_id, handler_id, env_map}   │
//     │  block on slot.condvar (with timeout)                     │
//     │  read slot.response, send HTTP reply                      │
//     └───────────────────────────────────────────────────────────┘
//                                  │
//                                  ▼  (BEAM scheduler dispatches)
//     ┌─ Elixir Conduit.Dispatcher gen_server ───────────────────┐
//     │  handle_info({:conduit_request, slot_id, hid, env}, ...) │
//     │  handler = state.handlers[hid]                            │
//     │  response = run_handler(handler, env)                     │
//     │  Conduit.Native.respond(slot_id, response)                │
//     └───────────────────────────────────────────────────────────┘
//                                  │
//                                  ▼  (regular fast NIF)
//     ┌─ respond/2 NIF (any scheduler thread) ───────────────────┐
//     │  slot = SLOTS.remove(slot_id)                             │
//     │  *slot.response.lock() = Some(parsed_response)            │
//     │  slot.condvar.notify_one()                                │
//     └───────────────────────────────────────────────────────────┘
//
// This is structurally identical to the napi_threadsafe_function pattern
// in WEB05's TypeScript port, just using BEAM's enif_send + a custom slot
// table instead of N-API's threadsafe function queue. The Rust I/O thread
// sleeps; the V8/BEAM main thread runs the handler; the Rust thread
// wakes up.
//
// # Why a slot ID and not a pid?
//
// The Rust thread needs a way to address "the response to this specific
// request". The dispatcher process handles many concurrent requests, so
// we can't use the dispatcher's pid alone. A monotonically increasing
// 64-bit slot ID identifies which condvar to wake up. The slot table
// (`SLOTS` static) maps slot_id → Arc<Slot>.
//
// # Halt protocol
//
// Elixir handlers signal "halt" by calling `throw {:conduit_halt, status,
// body, headers}`. The Conduit.Dispatcher catches this and converts to a
// regular response tuple before calling `respond/2`. So Rust never sees
// the halt — it just sees a normal `{status, headers, body}` come back.

#![allow(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]

use erl_nif_bridge::*;
use std::collections::HashMap;
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::Duration;
use web_core::{WebApp, WebRequest, WebResponse, WebServer};

// ---------------------------------------------------------------------------
// Platform-specific WebServer alias
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
// Slot table — in-flight requests waiting for an Elixir response
// ---------------------------------------------------------------------------
//
// Each in-flight request has a Slot. The Rust I/O thread inserts an
// Arc<Slot> at a unique slot_id, sends the message, then blocks on the
// slot's condvar. The Elixir dispatcher calls respond/2 with the slot_id;
// the respond NIF removes the entry, writes the response, and signals.

struct Slot {
    response: Mutex<Option<WebResponse>>,
    cond:     Condvar,
    /// Set by the I/O thread on timeout, or by `respond/2` on completion.
    /// Lets a late `respond/2` short-circuit cleanly even if the slot was
    /// already removed from the table — and lets a late wake-up notice
    /// after timeout return without trying to fetch a never-arrived
    /// response.
    done: AtomicBool,
}

impl Slot {
    fn new() -> Self {
        Self {
            response: Mutex::new(None),
            cond:     Condvar::new(),
            done:     AtomicBool::new(false),
        }
    }
}

// ---------------------------------------------------------------------------
// EnvGuard — RAII wrapper around enif_alloc_env / enif_free_env
// ---------------------------------------------------------------------------
//
// Without this, a panic anywhere between alloc and free leaks the env.
// Sustained panics → memory growth. Per security review finding H5.
//
// The guard owns the env exclusively while alive; calling `.into_raw()`
// transfers ownership out (used when the env's terms are consumed by
// `enif_send` — BEAM still owns the contents but the caller must NOT
// free the env in that case... actually `enif_send` only copies the
// terms; the env still needs to be freed afterwards. So we always
// drop the guard, but `into_raw` is provided for completeness).

struct EnvGuard {
    env: ErlNifEnv,
}

impl EnvGuard {
    /// Allocate a fresh long-lived env.
    fn new() -> Self {
        let env = unsafe { enif_alloc_env() };
        Self { env }
    }

    fn raw(&self) -> ErlNifEnv {
        self.env
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if !self.env.is_null() {
            unsafe { enif_free_env(self.env) };
        }
    }
}

/// Global slot table. Keyed by a monotonically increasing u64.
///
/// We use `RwLock` rather than `Mutex` because:
///  - Many threads may concurrently insert (writer-light).
///  - The respond NIF removes the slot under a write lock briefly.
///  - Reads (`get`) are rare; they only happen for diagnostic logging.
///
/// `OnceLock` defers the allocation until the NIF is loaded, so the static
/// has no static-initializer cost in BEAM startup.
static SLOTS: OnceLock<RwLock<HashMap<u64, Arc<Slot>>>> = OnceLock::new();
static NEXT_SLOT_ID: AtomicU64 = AtomicU64::new(1);

fn slots() -> &'static RwLock<HashMap<u64, Arc<Slot>>> {
    SLOTS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn next_slot_id() -> u64 {
    NEXT_SLOT_ID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// NativeApp — held inside a BEAM resource on the Elixir side
// ---------------------------------------------------------------------------
//
// Stores route definitions, before/after filter IDs, and the not_found /
// error_handler IDs. All "handlers" are integer IDs that the Elixir
// dispatcher uses to look up the actual function. We never store Elixir
// function refs in Rust — they are tied to a specific BEAM process and
// cannot safely outlive the construction site.

struct RouteEntry {
    method:     String,
    pattern:    String,
    handler_id: u64,
}

struct NativeApp {
    routes:            Vec<RouteEntry>,
    before_ids:        Vec<u64>,
    after_ids:         Vec<u64>,
    not_found_id:      Option<u64>,
    error_handler_id:  Option<u64>,
    settings:          HashMap<String, String>,
}

impl NativeApp {
    fn new() -> Self {
        NativeApp {
            routes:           Vec::new(),
            before_ids:       Vec::new(),
            after_ids:        Vec::new(),
            not_found_id:     None,
            error_handler_id: None,
            settings:         HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// NativeServer — held inside a BEAM resource
// ---------------------------------------------------------------------------

struct NativeServer {
    /// The dispatcher process to send `{:conduit_request, …}` messages to.
    /// Stored on the resource for inspection/diagnostics; the closures
    /// registered with WebApp captured a `Copy` of this pid at creation.
    #[allow(dead_code)]
    dispatcher_pid: ErlNifPid,
    /// Captured at bind time so server_local_port works after serve has moved
    /// the server into a thread.
    local_port: u16,
    /// Signals the background thread to stop (or stops the in-thread server).
    stop_handle: tcp_runtime::StopHandle,
    /// Server is moved into the dirty thread when serve() runs.
    server: Option<PlatformWebServer>,
    /// Background thread (only set by serve_background — None for serve()).
    bg_thread: Option<std::thread::JoinHandle<()>>,
    /// True while the server thread is active.
    running: Arc<AtomicBool>,
}

// We do NOT manually `unsafe impl Send/Sync` for NativeServer — the
// auto-derived impls already cover this struct (ErlNifPid is Send+Sync
// since its single `usize` field is, as are PlatformWebServer, JoinHandle,
// StopHandle, and Arc<AtomicBool>). Per security review finding H4, we
// removed the manual `unsafe impl` so we can't accidentally claim
// thread-safety beyond what the compiler verifies.

// ---------------------------------------------------------------------------
// Resource type registration
// ---------------------------------------------------------------------------
//
// On NIF library load, we register two resource types — one per Rust
// struct that crosses the BEAM boundary. The destructors run on the BEAM
// when the last reference is GC'd: they reconstruct the Box and drop it.

static APP_RTYPE:    OnceLock<usize> = OnceLock::new();   // *mut ErlNifResourceType
static SERVER_RTYPE: OnceLock<usize> = OnceLock::new();

fn app_rtype() -> *mut ErlNifResourceType {
    *APP_RTYPE.get().expect("conduit_native NIF not loaded") as *mut ErlNifResourceType
}

fn server_rtype() -> *mut ErlNifResourceType {
    *SERVER_RTYPE.get().expect("conduit_native NIF not loaded") as *mut ErlNifResourceType
}

unsafe extern "C" fn app_dtor(_env: ErlNifEnv, obj: *mut c_void) {
    // The resource memory itself is freed by BEAM. We just drop the inner
    // Box<NativeApp> that we placed there with `wrap_resource`.
    let app_ptr = obj as *mut NativeApp;
    ptr::drop_in_place(app_ptr);
}

unsafe extern "C" fn server_dtor(_env: ErlNifEnv, obj: *mut c_void) {
    let srv_ptr = obj as *mut NativeServer;
    let srv = &mut *srv_ptr;
    // Stop the server first so the background thread (if any) exits.
    srv.stop_handle.stop();
    if let Some(handle) = srv.bg_thread.take() {
        let _ = handle.join();
    }
    ptr::drop_in_place(srv_ptr);
}

unsafe extern "C" fn nif_load(env: ErlNifEnv, _priv: *mut *mut c_void, _info: ERL_NIF_TERM) -> c_int {
    let module = b"Elixir.CodingAdventures.Conduit.Native\0";
    let app_name = b"NativeApp\0";
    let srv_name = b"NativeServer\0";
    let mut tried = 0;
    let app_rt = enif_open_resource_type(
        env,
        module.as_ptr() as *const _,
        app_name.as_ptr() as *const _,
        Some(app_dtor),
        ERL_NIF_RT_CREATE,
        &mut tried,
    );
    if app_rt.is_null() { return 1; }
    let srv_rt = enif_open_resource_type(
        env,
        module.as_ptr() as *const _,
        srv_name.as_ptr() as *const _,
        Some(server_dtor),
        ERL_NIF_RT_CREATE,
        &mut tried,
    );
    if srv_rt.is_null() { return 1; }
    let _ = APP_RTYPE.set(app_rt as usize);
    let _ = SERVER_RTYPE.set(srv_rt as usize);
    0 // load success
}

// ---------------------------------------------------------------------------
// Helpers — safe term I/O
// ---------------------------------------------------------------------------
//
// argv slicing and consistent badarg returns.

unsafe fn argv_slice<'a>(argv: *const ERL_NIF_TERM, argc: c_int) -> &'a [ERL_NIF_TERM] {
    if argv.is_null() || argc <= 0 {
        return &[];
    }
    std::slice::from_raw_parts(argv, argc as usize)
}

/// Try to extract a Rust `String` from a binary term, falling back to None.
unsafe fn term_to_string(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<String> {
    binary_to_string(env, term)
}

/// Extract a `u64` from a non-negative integer term.
unsafe fn term_to_u64(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<u64> {
    let v = get_i64(env, term)?;
    if v < 0 { None } else { Some(v as u64) }
}

// ---------------------------------------------------------------------------
// new_app/0 — create a fresh NativeApp resource
// ---------------------------------------------------------------------------

unsafe extern "C" fn nif_new_app(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    wrap_resource::<NativeApp>(env, app_rtype(), NativeApp::new())
}

// ---------------------------------------------------------------------------
// app_add_route/4 — (app, "GET", "/path", handler_id)
// ---------------------------------------------------------------------------

unsafe extern "C" fn nif_app_add_route(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 4 { return badarg(env); }
    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p,
        None    => return badarg(env),
    };
    let method  = match term_to_string(env, args[1]) { Some(s) => s, None => return badarg(env) };
    let pattern = match term_to_string(env, args[2]) { Some(s) => s, None => return badarg(env) };
    let id      = match term_to_u64(env, args[3])    { Some(v) => v, None => return badarg(env) };
    (*app_ptr).routes.push(RouteEntry { method, pattern, handler_id: id });
    atom(env, "ok")
}

unsafe fn add_handler_id_into(
    env: ErlNifEnv,
    args: &[ERL_NIF_TERM],
    field: fn(&mut NativeApp) -> &mut Vec<u64>,
) -> ERL_NIF_TERM {
    if args.len() != 2 { return badarg(env); }
    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p,
        None    => return badarg(env),
    };
    let id = match term_to_u64(env, args[1]) { Some(v) => v, None => return badarg(env) };
    field(&mut *app_ptr).push(id);
    atom(env, "ok")
}

unsafe extern "C" fn nif_app_add_before(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    add_handler_id_into(env, argv_slice(argv, argc), |app| &mut app.before_ids)
}

unsafe extern "C" fn nif_app_add_after(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    add_handler_id_into(env, argv_slice(argv, argc), |app| &mut app.after_ids)
}

unsafe fn set_optional_handler(
    env: ErlNifEnv,
    args: &[ERL_NIF_TERM],
    field: fn(&mut NativeApp) -> &mut Option<u64>,
) -> ERL_NIF_TERM {
    if args.len() != 2 { return badarg(env); }
    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p,
        None    => return badarg(env),
    };
    let id = match term_to_u64(env, args[1]) { Some(v) => v, None => return badarg(env) };
    *field(&mut *app_ptr) = Some(id);
    atom(env, "ok")
}

unsafe extern "C" fn nif_app_set_not_found(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    set_optional_handler(env, argv_slice(argv, argc), |app| &mut app.not_found_id)
}

unsafe extern "C" fn nif_app_set_error_handler(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    set_optional_handler(env, argv_slice(argv, argc), |app| &mut app.error_handler_id)
}

// ---------------------------------------------------------------------------
// app_set_setting/3 and app_get_setting/2
// ---------------------------------------------------------------------------

unsafe extern "C" fn nif_app_set_setting(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 3 { return badarg(env); }
    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let key   = match term_to_string(env, args[1]) { Some(s) => s, None => return badarg(env) };
    let value = match term_to_string(env, args[2]) { Some(s) => s, None => return badarg(env) };
    (*app_ptr).settings.insert(key, value);
    atom(env, "ok")
}

unsafe extern "C" fn nif_app_get_setting(
    env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 2 { return badarg(env); }
    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let key = match term_to_string(env, args[1]) { Some(s) => s, None => return badarg(env) };
    match (*app_ptr).settings.get(&key) {
        Some(v) => str_to_binary(env, v),
        None    => atom(env, "nil"),
    }
}

// ---------------------------------------------------------------------------
// new_server/5 — (app, host, port, max_conn, dispatcher_pid)
// ---------------------------------------------------------------------------

unsafe extern "C" fn nif_new_server(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 5 { return badarg(env); }

    let app_ptr = match unwrap_resource::<NativeApp>(env, args[0], app_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let host = match term_to_string(env, args[1])  { Some(s) => s, None => return badarg(env) };
    let port = match get_i64(env, args[2])         { Some(v) if (0..=65535).contains(&v) => v as u16, _ => return badarg(env) };
    let max_conn = match get_i64(env, args[3])     { Some(v) if v > 0 => v as usize, _ => return badarg(env) };
    let dispatcher_pid = match get_pid(env, args[4]) { Some(p) => p, None => return badarg(env) };

    let app_ref = &*app_ptr;
    let mut web_app = WebApp::new();

    // Register routes — every closure captures the dispatcher_pid (Copy)
    // and the handler_id for that route.
    for rt in &app_ref.routes {
        let pid = dispatcher_pid;
        let id  = rt.handler_id;
        let err_id = app_ref.error_handler_id;
        web_app.add(&rt.method, &rt.pattern, move |req| {
            let resp = dispatch_to_elixir(pid, id, req, None);
            // If a route handler raised an exception (sentinel: status 500
            // with no headers and a non-empty body) and an error_handler is
            // registered, re-dispatch.
            if resp.status == 500 && resp.headers.is_empty() {
                if let Some(eid) = err_id {
                    let err_msg = String::from_utf8_lossy(&resp.body).into_owned();
                    return dispatch_to_elixir(pid, eid, req, Some(err_msg));
                }
            }
            resp
        });
    }

    // before filters: each can short-circuit by returning a real response.
    for &bid in &app_ref.before_ids {
        let pid = dispatcher_pid;
        web_app.before_routing(move |req| {
            let resp = dispatch_to_elixir(pid, bid, req, None);
            if is_no_override(&resp) { None } else { Some(resp) }
        });
    }

    // after filters: receive the existing response and may rewrite it.
    for &aid in &app_ref.after_ids {
        let pid = dispatcher_pid;
        web_app.after_handler(move |req, resp| {
            let new_resp = dispatch_to_elixir(pid, aid, req, None);
            if is_no_override(&new_resp) { resp } else { new_resp }
        });
    }

    // not_found
    if let Some(nf_id) = app_ref.not_found_id {
        let pid = dispatcher_pid;
        web_app.on_not_found(move |req| {
            let resp = dispatch_to_elixir(pid, nf_id, req, None);
            if is_no_override(&resp) { WebResponse::not_found() } else { resp }
        });
    }

    // error_handler (web-core's hook; in addition to the per-route fallback)
    if let Some(err_id) = app_ref.error_handler_id {
        let pid = dispatcher_pid;
        web_app.on_handler_error(move |req, msg| {
            let resp = dispatch_to_elixir(pid, err_id, req, Some(msg.to_string()));
            if is_no_override(&resp) {
                WebResponse::internal_error("internal server error")
            } else { resp }
        });
    }

    let mut opts = embeddable_http_server::HttpServerOptions::default();
    opts.tcp.max_connections = max_conn;

    let server = match bind_server(&host, port, opts, Arc::new(web_app)) {
        Ok(s) => s,
        Err(_e) => return badarg(env),
    };
    let local_port = server.local_addr().port();
    let stop_handle = server.stop_handle();

    let native_server = NativeServer {
        dispatcher_pid,
        local_port,
        stop_handle,
        server: Some(server),
        bg_thread: None,
        running: Arc::new(AtomicBool::new(false)),
    };

    wrap_resource::<NativeServer>(env, server_rtype(), native_server)
}

// ---------------------------------------------------------------------------
// dispatch_to_elixir — the heart of cross-thread request handling
// ---------------------------------------------------------------------------
//
// Called on a Rust I/O thread for each request. Posts a message to the
// dispatcher process via enif_send and blocks on a Condvar until the
// Elixir handler finishes (or 30s elapses).

const HANDLER_TIMEOUT: Duration = Duration::from_secs(30);

fn dispatch_to_elixir(
    dispatcher_pid: ErlNifPid,
    handler_id: u64,
    req: &WebRequest,
    error_message: Option<String>,
) -> WebResponse {
    // 1. Allocate a slot and put it in the global table.
    let slot = Arc::new(Slot::new());
    let slot_id = next_slot_id();
    {
        // Recover-from-poison rather than panic — a panic in another thread
        // shouldn't take down every subsequent request (review finding L1).
        let mut table = slots().write().unwrap_or_else(|e| e.into_inner());
        table.insert(slot_id, Arc::clone(&slot));
    }

    // 2. Build the env map and send it to the dispatcher.
    //
    // EnvGuard is RAII: if anything between alloc and send panics, drop
    // frees the env. enif_send copies the terms into the recipient mailbox;
    // the env is freed when the guard goes out of scope (review finding H5).
    {
        let guard = EnvGuard::new();
        let msg_env = guard.raw();
        let sent = unsafe {
            let env_map_term = build_env_map_term(msg_env, req, &error_message);
            let tag       = atom(msg_env, "conduit_request");
            let slot_term = make_i64(msg_env, slot_id as i64);
            let id_term   = make_i64(msg_env, handler_id as i64);
            let arr       = [tag, slot_term, id_term, env_map_term];
            let msg       = enif_make_tuple_from_array(msg_env, arr.as_ptr(), 4);
            send_from_thread(&dispatcher_pid, msg_env, msg)
        };
        // guard drops here, freeing msg_env unconditionally.

        if !sent {
            // Dispatcher died — clean up and return 500.
            let mut table = slots().write().unwrap_or_else(|e| e.into_inner());
            table.remove(&slot_id);
            return WebResponse::internal_error("dispatcher process is dead");
        }
    }

    // 3. Block on the slot's condvar with a timeout.
    let (lock, cvar) = (&slot.response, &slot.cond);
    let mut response_guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    let deadline = std::time::Instant::now() + HANDLER_TIMEOUT;
    while response_guard.is_none() {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            // Timeout: mark slot as done so a late respond/2 short-circuits,
            // remove from table, return error response.  We MUST drop the
            // mutex guard before touching the table to avoid lock ordering
            // issues with respond/2 (which holds the mutex while we'd be
            // waiting on the table write lock — review finding H1).
            slot.done.store(true, Ordering::SeqCst);
            drop(response_guard);
            let mut table = slots().write().unwrap_or_else(|e| e.into_inner());
            table.remove(&slot_id);
            return WebResponse::internal_error("handler timeout: BEAM dispatcher did not respond");
        }
        let (g, _timed_out) =
            cvar.wait_timeout(response_guard, remaining).unwrap_or_else(|e| e.into_inner());
        response_guard = g;
    }
    slot.done.store(true, Ordering::SeqCst);
    response_guard.take().unwrap()
}

/// Sentinel: status=0 with no headers/body means "no override".
fn no_override_response() -> WebResponse {
    WebResponse { status: 0, headers: vec![], body: vec![] }
}

fn is_no_override(r: &WebResponse) -> bool {
    r.status == 0 && r.headers.is_empty() && r.body.is_empty()
}

// ---------------------------------------------------------------------------
// build_env_map_term — Rust HashMap → Elixir map term (in msg_env)
// ---------------------------------------------------------------------------

unsafe fn build_env_map_term(
    msg_env: ErlNifEnv,
    req: &WebRequest,
    error_message: &Option<String>,
) -> ERL_NIF_TERM {
    let mut m = enif_make_new_map(msg_env);

    let put = |env: ErlNifEnv, m_in: ERL_NIF_TERM, k: &str, v: ERL_NIF_TERM| -> ERL_NIF_TERM {
        let key = str_to_binary(env, k);
        map_put(env, m_in, key, v)
    };
    let put_s = |env: ErlNifEnv, m_in: ERL_NIF_TERM, k: &str, v: &str| -> ERL_NIF_TERM {
        put(env, m_in, k, str_to_binary(env, v))
    };

    m = put_s(msg_env, m, "REQUEST_METHOD", req.method());
    m = put_s(msg_env, m, "PATH_INFO",      req.path());

    let target = req.http.head.target.as_str();
    let qs = target.find('?').map(|i| &target[i + 1..]).unwrap_or("");
    m = put_s(msg_env, m, "QUERY_STRING", qs);

    let peer = req.peer_addr();
    m = put_s(msg_env, m, "REMOTE_ADDR", &peer.ip().to_string());
    m = put_s(msg_env, m, "REMOTE_PORT", &peer.port().to_string());

    if let Some(ct) = req.content_type() {
        m = put_s(msg_env, m, "conduit.content_type", ct);
    }
    if let Some(cl) = req.content_length() {
        m = put_s(msg_env, m, "conduit.content_length", &cl.to_string());
    }
    let body = String::from_utf8_lossy(req.body()).into_owned();
    m = put_s(msg_env, m, "conduit.body", &body);

    // Route params as a nested map.
    let route_map = build_string_map(msg_env, &req.route_params);
    m = put(msg_env, m, "conduit.route_params", route_map);

    // Query params as a nested map.
    let query_map = build_string_map(msg_env, &req.query_params);
    m = put(msg_env, m, "conduit.query_params", query_map);

    // Headers (lowercase keys) as a nested map.
    let mut headers_hm: HashMap<String, String> = HashMap::new();
    for h in &req.http.head.headers {
        headers_hm.entry(h.name.to_lowercase()).or_insert_with(|| h.value.clone());
    }
    let headers_map = build_string_map(msg_env, &headers_hm);
    m = put(msg_env, m, "conduit.headers", headers_map);

    if let Some(msg) = error_message {
        m = put_s(msg_env, m, "conduit.error", msg);
    }

    m
}

unsafe fn build_string_map(env: ErlNifEnv, hm: &HashMap<String, String>) -> ERL_NIF_TERM {
    let mut m = enif_make_new_map(env);
    for (k, v) in hm {
        let kt = str_to_binary(env, k);
        let vt = str_to_binary(env, v);
        m = map_put(env, m, kt, vt);
    }
    m
}

// ---------------------------------------------------------------------------
// respond/2 — (slot_id, response) — Elixir signals slot completion
// ---------------------------------------------------------------------------
//
// The Elixir dispatcher calls this with the slot_id given to it by the
// {:conduit_request, slot_id, …} message and a response tuple. response is:
//   - nil/atom:nil → "no override" sentinel (status 0)
//   - {status, headers, body} where headers is a map and body is binary

unsafe extern "C" fn nif_respond(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 2 { return badarg(env); }

    let slot_id = match term_to_u64(env, args[0]) { Some(v) => v, None => return badarg(env) };
    let response = parse_response_term(env, args[1]);

    // Remove the slot from the table and signal its condvar. The whole
    // operation is idempotent — a late respond after timeout, or a double
    // respond, is a silent no-op (review finding H1).
    let slot = {
        let mut table = slots().write().unwrap_or_else(|e| e.into_inner());
        match table.remove(&slot_id) {
            Some(s) => s,
            None    => return atom(env, "ok"),  // already responded / timed out
        }
    };
    // Even if the I/O thread already timed out and set `done = true` we
    // still write the response (harmless — nobody is waiting to read it)
    // to keep the slot's invariants tidy. The `done` flag mainly serves
    // future readers / debuggers.
    if !slot.done.load(Ordering::SeqCst) {
        let mut guard = slot.response.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(response);
        slot.cond.notify_one();
    }
    atom(env, "ok")
}

unsafe fn parse_response_term(env: ErlNifEnv, term: ERL_NIF_TERM) -> WebResponse {
    // Atom :nil or anything that isn't a 3-tuple → no_override.
    if enif_is_tuple(env, term) == 0 {
        return no_override_response();
    }
    let mut arity: c_int = 0;
    let mut elems: *const ERL_NIF_TERM = ptr::null();
    if enif_get_tuple(env, term, &mut arity, &mut elems) == 0 || arity != 3 {
        return no_override_response();
    }
    let parts = std::slice::from_raw_parts(elems, 3);

    let raw_status = get_i64(env, parts[0]).unwrap_or(500);
    // Status 0 is our "no override" sentinel — pass through unchanged so
    // before/after filters that returned nil propagate correctly. Any
    // out-of-range non-zero value is clamped to 500 for safety.
    let status: u16 = match raw_status {
        0 => 0,
        n if (100..=599).contains(&n) => n as u16,
        _ => 500,
    };

    let headers = parse_headers_map(env, parts[1]);
    let body_bytes = match binary_to_bytes(env, parts[2]) {
        Some(b) => b.to_vec(),
        None    => Vec::new(),
    };
    WebResponse { status, headers, body: body_bytes }
}

unsafe fn parse_headers_map(env: ErlNifEnv, term: ERL_NIF_TERM) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    if enif_is_map(env, term) == 0 {
        return headers;
    }
    let mut iter = ErlNifMapIterator::zeroed();
    if enif_map_iterator_create(env, term, &mut iter, ERL_NIF_MAP_ITERATOR_FIRST) == 0 {
        return headers;
    }
    loop {
        let mut k: ERL_NIF_TERM = 0;
        let mut v: ERL_NIF_TERM = 0;
        if enif_map_iterator_get_pair(env, &mut iter, &mut k, &mut v) == 0 {
            break;
        }
        if let (Some(ks), Some(vs)) = (binary_to_string(env, k), binary_to_string(env, v)) {
            // Security: reject HTTP response splitting (CR/LF injection).
            // A header name or value containing 0x0A or 0x0D could craft a
            // second response on the wire (poisoning caches, forging cookies).
            // Header names additionally must not contain `:` (which would
            // make `Name: value` parsing ambiguous). Drop any such pair.
            if is_valid_header_name(&ks) && is_valid_header_value(&vs) {
                headers.push((ks, vs));
            }
        }
        if enif_map_iterator_next(env, &mut iter) == 0 {
            break;
        }
    }
    enif_map_iterator_destroy(env, &mut iter);
    headers
}

fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && !name.bytes().any(|b| b == b'\r' || b == b'\n' || b == b':' || b == 0)
}

fn is_valid_header_value(value: &str) -> bool {
    !value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0)
}

// ---------------------------------------------------------------------------
// server_serve/1, server_serve_background/1, server_stop/1, server_running/1,
// server_local_port/1
// ---------------------------------------------------------------------------

unsafe extern "C" fn nif_server_serve(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 { return badarg(env); }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let mut server = match (*srv_ptr).server.take() {
        Some(s) => s,
        None    => return badarg(env),  // already running
    };
    let running = Arc::clone(&(*srv_ptr).running);
    running.store(true, Ordering::SeqCst);
    let _ = server.serve();
    running.store(false, Ordering::SeqCst);
    atom(env, "ok")
}

unsafe extern "C" fn nif_server_serve_background(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 { return badarg(env); }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let mut server = match (*srv_ptr).server.take() {
        Some(s) => s, None => return badarg(env),
    };
    let running = Arc::clone(&(*srv_ptr).running);
    running.store(true, Ordering::SeqCst);
    let handle = std::thread::spawn(move || {
        let _ = server.serve();
        running.store(false, Ordering::SeqCst);
    });
    (*srv_ptr).bg_thread = Some(handle);
    atom(env, "ok")
}

unsafe extern "C" fn nif_server_stop(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 { return badarg(env); }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    (*srv_ptr).stop_handle.stop();
    if let Some(h) = (*srv_ptr).bg_thread.take() {
        let _ = h.join();
    }
    atom(env, "ok")
}

unsafe extern "C" fn nif_server_local_port(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 { return badarg(env); }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    make_i64(env, (*srv_ptr).local_port as i64)
}

unsafe extern "C" fn nif_server_running(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 { return badarg(env); }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p, None => return badarg(env),
    };
    let running = (*srv_ptr).running.load(Ordering::SeqCst);
    atom(env, if running { "true" } else { "false" })
}

// ---------------------------------------------------------------------------
// NIF entry table
// ---------------------------------------------------------------------------

// 15 NIFs total. server_serve has flags=ERL_NIF_DIRTY_JOB_IO_BOUND (=2)
// because it blocks for the entire server lifetime; everything else runs
// on the regular scheduler.
struct FuncTable([ErlNifFunc; 15]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    ErlNifFunc { name: b"new_app\0".as_ptr() as *const _,                arity: 0, fptr: nif_new_app,                 flags: 0 },
    ErlNifFunc { name: b"app_add_route\0".as_ptr() as *const _,          arity: 4, fptr: nif_app_add_route,           flags: 0 },
    ErlNifFunc { name: b"app_add_before\0".as_ptr() as *const _,         arity: 2, fptr: nif_app_add_before,          flags: 0 },
    ErlNifFunc { name: b"app_add_after\0".as_ptr() as *const _,          arity: 2, fptr: nif_app_add_after,           flags: 0 },
    ErlNifFunc { name: b"app_set_not_found\0".as_ptr() as *const _,      arity: 2, fptr: nif_app_set_not_found,       flags: 0 },
    ErlNifFunc { name: b"app_set_error_handler\0".as_ptr() as *const _,  arity: 2, fptr: nif_app_set_error_handler,   flags: 0 },
    ErlNifFunc { name: b"app_set_setting\0".as_ptr() as *const _,        arity: 3, fptr: nif_app_set_setting,         flags: 0 },
    ErlNifFunc { name: b"app_get_setting\0".as_ptr() as *const _,        arity: 2, fptr: nif_app_get_setting,         flags: 0 },
    ErlNifFunc { name: b"new_server\0".as_ptr() as *const _,             arity: 5, fptr: nif_new_server,              flags: 0 },
    ErlNifFunc { name: b"server_serve\0".as_ptr() as *const _,           arity: 1, fptr: nif_server_serve,            flags: 2 },
    ErlNifFunc { name: b"server_serve_background\0".as_ptr() as *const _, arity: 1, fptr: nif_server_serve_background, flags: 0 },
    ErlNifFunc { name: b"server_stop\0".as_ptr() as *const _,            arity: 1, fptr: nif_server_stop,             flags: 0 },
    ErlNifFunc { name: b"server_local_port\0".as_ptr() as *const _,      arity: 1, fptr: nif_server_local_port,       flags: 0 },
    ErlNifFunc { name: b"server_running\0".as_ptr() as *const _,         arity: 1, fptr: nif_server_running,          flags: 0 },
    ErlNifFunc { name: b"respond\0".as_ptr() as *const _,                arity: 2, fptr: nif_respond,                 flags: 0 },
]);

static MODULE_NAME_BYTES: &[u8] = b"Elixir.CodingAdventures.Conduit.Native\0";
static VM_VARIANT_BYTES:  &[u8] = b"beam.vanilla\0";
static MIN_ERTS_BYTES:    &[u8] = b"erts-13.0\0";

struct NifEntry(erl_nif_bridge::ErlNifEntry);
unsafe impl Sync for NifEntry {}

static NIF_ENTRY: NifEntry = NifEntry(erl_nif_bridge::ErlNifEntry {
    major: ERL_NIF_MAJOR_VERSION,
    minor: ERL_NIF_MINOR_VERSION,
    name: MODULE_NAME_BYTES.as_ptr() as *const std::ffi::c_char,
    num_of_funcs: 15,
    funcs: FUNCS.0.as_ptr(),
    load: Some(nif_load),
    reload: None,
    upgrade: None,
    unload: None,
    vm_variant: VM_VARIANT_BYTES.as_ptr() as *const std::ffi::c_char,
    options: 0,
    sizeof_ErlNifResourceTypeInit: 0,
    min_erts: MIN_ERTS_BYTES.as_ptr() as *const std::ffi::c_char,
});

#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const erl_nif_bridge::ErlNifEntry {
    &NIF_ENTRY.0
}
