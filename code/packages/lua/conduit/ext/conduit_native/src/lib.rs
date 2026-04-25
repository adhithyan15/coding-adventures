//! conduit_native — Lua 5.4 Conduit extension
//!
//! This cdylib is loaded by Lua via `require("conduit_native")`. The entry
//! point `luaopen_conduit_native` registers a module table that the Lua DSL
//! (`conduit/init.lua`) wraps into the friendly Application/Server API.
//!
//! ## Threading model
//!
//! Lua 5.4's `lua_State` is NOT thread-safe. `web-core` dispatches incoming
//! requests on background Rust I/O threads. Every dispatch closure captures an
//! `Arc<Mutex<()>>` (the "Lua lock") and acquires it before calling `lua_pcall`,
//! serialising all Lua re-entries to a single OS thread at a time.
//!
//! `server_serve` blocks the calling Lua thread inside `web_server.serve()`.
//! `server_serve_background` spawns a Rust thread (used by tests to start the
//! server without blocking the Lua process).
//!
//! ## Response protocol (mirrors Ruby WEB02 and Python WEB03)
//!
//!   - Handler returns `nil`             → no response (before filters only)
//!   - Handler returns `{s, hdrs, body}` → use this response
//!   - Handler raises a HaltError table  → `lua_pcall` returns non-zero;
//!       Rust checks the top-of-stack for `__conduit_halt = true`, extracts
//!       `{status, body, headers}` → WebResponse.
//!   - Handler raises any other error    → calls error handler if registered;
//!       falls back to 500.

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::{c_char, c_int, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use http_core::Header;
use lua_bridge::{
    get_str, lua_Integer, lua_State, lua_gettop, lua_getfield, lua_isnumber,
    lua_newtable, lua_pop, lua_pushboolean, lua_pushcclosure, lua_pushinteger, lua_pushnil,
    lua_pushvalue, lua_rawgeti, lua_rawlen, lua_setfield, lua_setmetatable,
    lua_settop, lua_toboolean, lua_tointeger, lua_type, lua_newuserdatauv, luaL_checkudata,
    luaL_error, luaL_Reg, push_str, register_lib,
    LUA_REGISTRYINDEX, LUA_TFUNCTION, LUA_TNIL, LUA_TNONE, LUA_TSTRING, LUA_TTABLE,
};
use web_core::{WebApp, WebRequest, WebResponse, WebServer};

// ---------------------------------------------------------------------------
// Symbols missing from lua-bridge — declared here directly.
//
// lua-bridge has `lua_newmetatable` but that symbol does NOT exist in liblua.
// The real auxiliary-library function is `luaL_newmetatable`. We declare it
// below.  Similarly, lua_pcall (a C macro wrapping lua_pcallk), luaL_ref, and
// luaL_unref are not in lua-bridge.
// ---------------------------------------------------------------------------

/// Equivalent of `LUA_NOREF` — the value returned by luaL_ref when asked to
/// store nil, and the sentinel meaning "no reference stored".
pub const LUA_NOREF: c_int = -2;

extern "C" {
    /// Create (or look up) a metatable with name `tname` in the Lua registry.
    /// Returns 1 if created new, 0 if already existed. Pushes the table.
    fn luaL_newmetatable(L: *mut lua_State, tname: *const c_char) -> c_int;

    /// Store the value at the top of the stack in table `t` and return an
    /// integer key (reference). The value is popped. Use LUA_REGISTRYINDEX
    /// for `t` to store in the global registry.
    fn luaL_ref(L: *mut lua_State, t: c_int) -> c_int;

    /// Release reference `r` from table `t`. After this call `r` is invalid.
    fn luaL_unref(L: *mut lua_State, t: c_int, r: c_int);

    /// Underlying function that `lua_pcall` expands to in Lua 5.4.
    /// Call a Lua function already on the stack with `nargs` arguments.
    /// Returns 0 on success; non-zero on error (error value left on stack).
    fn lua_pcallk(
        L: *mut lua_State,
        nargs: c_int,
        nresults: c_int,
        msgh: c_int,
        ctx: isize,
        k: Option<unsafe extern "C" fn(*mut lua_State, c_int, isize) -> c_int>,
    ) -> c_int;
}

/// Safe call: push function + args, call, collect result.
/// Returns 0 on success; sets error value on stack on failure.
#[inline]
unsafe fn lua_pcall(L: *mut lua_State, nargs: c_int, nresults: c_int, msgh: c_int) -> c_int {
    lua_pcallk(L, nargs, nresults, msgh, 0, None)
}

// ---------------------------------------------------------------------------
// Thread-safe pointer wrapper
//
// Raw pointers are !Send + !Sync. Closures that capture *mut lua_State or
// *mut PlatformWebServer won't satisfy the Send + Sync bounds that web-core's
// handler closures require. We assert safety here: all accesses to lua_State
// are serialised by lua_lock; WebServer pointer is only used in the single
// background serve thread.
// ---------------------------------------------------------------------------

struct ThreadSafePtr<T>(*mut T);
unsafe impl<T> Send for ThreadSafePtr<T> {}
unsafe impl<T> Sync for ThreadSafePtr<T> {}

impl<T> ThreadSafePtr<T> {
    /// Return the wrapped raw pointer.
    ///
    /// Using a method here prevents Rust 2021's "capture disjoint fields"
    /// from capturing the inner `*mut T` directly (which is `!Send + !Sync`).
    /// Calling `.ptr()` causes Rust to capture `self` (the `ThreadSafePtr`
    /// wrapper, which IS `Send + Sync`) rather than just the field.
    fn ptr(&self) -> *mut T { self.0 }
}

// ---------------------------------------------------------------------------
// Platform alias
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Metatable name constants
// ---------------------------------------------------------------------------

const APP_MT: *const c_char = b"ConduitApp\0".as_ptr() as *const c_char;
const SERVER_MT: *const c_char = b"ConduitServer\0".as_ptr() as *const c_char;

// ---------------------------------------------------------------------------
// Rust-side state stored in Lua full userdata
// ---------------------------------------------------------------------------

/// One registered route: HTTP method, URL pattern, and a registry reference
/// to the Lua handler function.
struct RouteEntry {
    method: String,
    pattern: String,
    handler_ref: i32,
}

/// Application state stored in a Lua userdata.
///
/// The `lua_State` pointer is NOT kept here — it is captured at `new_server`
/// time by the closure created for each route/hook. During `app_add_route`
/// and friends, the caller passes `L` from the C function argument.
struct LuaConduitApp {
    routes: Vec<RouteEntry>,
    before_refs: Vec<i32>,
    after_refs: Vec<i32>,
    not_found_ref: i32,
    error_handler_ref: i32,
    settings: Vec<(String, String)>,
}

impl LuaConduitApp {
    fn new() -> Self {
        Self {
            routes: Vec::new(),
            before_refs: Vec::new(),
            after_refs: Vec::new(),
            not_found_ref: LUA_NOREF,
            error_handler_ref: LUA_NOREF,
            settings: Vec::new(),
        }
    }
}

/// Server state stored in a Lua userdata.
struct LuaConduitServer {
    /// Raw pointer to the Lua interpreter. NEVER accessed without holding
    /// `lua_lock`.
    lua: *mut lua_State,
    /// Serialises all callbacks from web-core's background I/O threads back
    /// into the single-threaded Lua interpreter.
    lua_lock: Arc<Mutex<()>>,
    server: Option<PlatformWebServer>,
    running: Arc<AtomicBool>,
}

// SAFETY: All access to `lua` is gated by `lua_lock`.
unsafe impl Send for LuaConduitServer {}
unsafe impl Sync for LuaConduitServer {}

// ---------------------------------------------------------------------------
// Userdata accessors
// ---------------------------------------------------------------------------

unsafe fn check_app(L: *mut lua_State, idx: c_int) -> *mut LuaConduitApp {
    luaL_checkudata(L, idx, APP_MT) as *mut LuaConduitApp
}

unsafe fn check_server(L: *mut lua_State, idx: c_int) -> *mut LuaConduitServer {
    luaL_checkudata(L, idx, SERVER_MT) as *mut LuaConduitServer
}

/// Read a string argument at stack index `idx`, raising a Lua error if absent.
unsafe fn str_arg(L: *mut lua_State, idx: c_int, name: &str) -> String {
    if lua_type(L, idx) != LUA_TSTRING && lua_isnumber(L, idx) == 0 {
        let msg = CString::new(format!("{name} must be a string")).unwrap();
        luaL_error(L, msg.as_ptr());
    }
    get_str(L, idx).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// new_app() → app userdata
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_new_app(L: *mut lua_State) -> c_int {
    // Allocate exactly sizeof(LuaConduitApp) bytes as a Lua full userdata.
    // Lua's GC owns this memory; we placement-construct our Rust struct into it.
    let ptr = lua_newuserdatauv(L, std::mem::size_of::<LuaConduitApp>(), 0);
    ptr::write(ptr as *mut LuaConduitApp, LuaConduitApp::new());

    // Attach the APP_MT metatable (created lazily) so __gc fires on collection.
    // Stack after luaL_newmetatable: [ud | mt]
    // Stack after lua_pushcclosure:  [ud | mt | fn]
    // lua_rawset_str_top pops fn and sets mt["__gc"] = fn → [ud | mt]
    // lua_setmetatable(-2) pops mt and sets ud's metatable → [ud]
    luaL_newmetatable(L, APP_MT);
    lua_pushcclosure(L, Some(app_gc), 0);
    lua_rawset_str_top(L, -2, "__gc\0");
    lua_setmetatable(L, -2);

    1
}

unsafe extern "C" fn app_gc(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    // Release all stored Lua function references before the memory is freed.
    for r in &(*app).routes {
        luaL_unref(L, LUA_REGISTRYINDEX, r.handler_ref);
    }
    for &r in &(*app).before_refs {
        luaL_unref(L, LUA_REGISTRYINDEX, r);
    }
    for &r in &(*app).after_refs {
        luaL_unref(L, LUA_REGISTRYINDEX, r);
    }
    if (*app).not_found_ref != LUA_NOREF {
        luaL_unref(L, LUA_REGISTRYINDEX, (*app).not_found_ref);
    }
    if (*app).error_handler_ref != LUA_NOREF {
        luaL_unref(L, LUA_REGISTRYINDEX, (*app).error_handler_ref);
    }
    ptr::drop_in_place(app);
    0
}

// ---------------------------------------------------------------------------
// app_add_route(app, method, pattern, fn) → nil
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_app_add_route(L: *mut lua_State) -> c_int {
    let app     = check_app(L, 1);
    let method  = str_arg(L, 2, "method");
    let pattern = str_arg(L, 3, "pattern");
    if lua_type(L, 4) != LUA_TFUNCTION {
        luaL_error(L, b"handler must be a function\0".as_ptr() as *const c_char);
    }
    lua_pushvalue(L, 4);
    let handler_ref = luaL_ref(L, LUA_REGISTRYINDEX);
    (*app).routes.push(RouteEntry { method, pattern, handler_ref });
    0
}

// ---------------------------------------------------------------------------
// app_add_before(app, fn) / app_add_after(app, fn) → nil
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_app_add_before(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    if lua_type(L, 2) != LUA_TFUNCTION {
        luaL_error(L, b"before filter must be a function\0".as_ptr() as *const c_char);
    }
    lua_pushvalue(L, 2);
    (*app).before_refs.push(luaL_ref(L, LUA_REGISTRYINDEX));
    0
}

unsafe extern "C" fn lua_app_add_after(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    if lua_type(L, 2) != LUA_TFUNCTION {
        luaL_error(L, b"after filter must be a function\0".as_ptr() as *const c_char);
    }
    lua_pushvalue(L, 2);
    (*app).after_refs.push(luaL_ref(L, LUA_REGISTRYINDEX));
    0
}

// ---------------------------------------------------------------------------
// app_set_not_found(app, fn) / app_set_error_handler(app, fn) → nil
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_app_set_not_found(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    if lua_type(L, 2) != LUA_TFUNCTION {
        luaL_error(L, b"not_found handler must be a function\0".as_ptr() as *const c_char);
    }
    if (*app).not_found_ref != LUA_NOREF {
        luaL_unref(L, LUA_REGISTRYINDEX, (*app).not_found_ref);
    }
    lua_pushvalue(L, 2);
    (*app).not_found_ref = luaL_ref(L, LUA_REGISTRYINDEX);
    0
}

unsafe extern "C" fn lua_app_set_error_handler(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    if lua_type(L, 2) != LUA_TFUNCTION {
        luaL_error(L, b"error_handler must be a function\0".as_ptr() as *const c_char);
    }
    if (*app).error_handler_ref != LUA_NOREF {
        luaL_unref(L, LUA_REGISTRYINDEX, (*app).error_handler_ref);
    }
    lua_pushvalue(L, 2);
    (*app).error_handler_ref = luaL_ref(L, LUA_REGISTRYINDEX);
    0
}

// ---------------------------------------------------------------------------
// app_set_setting / app_get_setting
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_app_set_setting(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    let key = str_arg(L, 2, "key");
    let val = str_arg(L, 3, "value");
    if let Some(entry) = (*app).settings.iter_mut().find(|(k, _)| k == &key) {
        entry.1 = val;
    } else {
        (*app).settings.push((key, val));
    }
    0
}

unsafe extern "C" fn lua_app_get_setting(L: *mut lua_State) -> c_int {
    let app = check_app(L, 1);
    let key = str_arg(L, 2, "key");
    match (*app).settings.iter().find(|(k, _)| k == &key) {
        Some((_, v)) => { let owned = v.clone(); push_str(L, &owned); }
        None         => lua_pushnil(L),
    }
    1
}

// ---------------------------------------------------------------------------
// new_server(app, host, port, max_conn) → server userdata
//
// Reads all routes/hooks from the app, builds a WebApp, and binds the TCP
// socket. Returns a server userdata ready to serve().
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_new_server(L: *mut lua_State) -> c_int {
    let app_ptr  = check_app(L, 1);
    let host     = str_arg(L, 2, "host");
    let port     = lua_tointeger(L, 3) as u16;
    let max_conn = lua_tointeger(L, 4) as usize;

    let lua_lock = Arc::new(Mutex::new(()));
    let running  = Arc::new(AtomicBool::new(false));

    // Wrap the raw lua_State pointer so closures sent to background threads
    // satisfy the Send + Sync bounds required by web_core's handler API.
    // SAFETY: every access to the pointer is serialised by lua_lock.
    let lua_safe = ThreadSafePtr(L);

    let mut web_app = WebApp::new();

    // Routes — each closure captures the lua_State pointer, lua_lock, and
    // the error handler ref so Lua errors are routed to the error handler.
    let eh_ref = (*app_ptr).error_handler_ref;
    for route in &(*app_ptr).routes {
        let lua_cap  = ThreadSafePtr(lua_safe.ptr());
        let lock_cap = Arc::clone(&lua_lock);
        let href     = route.handler_ref;

        web_app.add(&route.method, &route.pattern, move |req: &WebRequest| {
            dispatch_route(lua_cap.ptr(), &lock_cap, href, eh_ref, req)
        });
    }

    // Before filters.
    if !(*app_ptr).before_refs.is_empty() {
        let refs     = (*app_ptr).before_refs.clone();
        let lua_cap  = ThreadSafePtr(lua_safe.ptr());
        let lock_cap = Arc::clone(&lua_lock);
        web_app.before_routing(move |req| dispatch_before(lua_cap.ptr(), &lock_cap, &refs, req));
    }

    // After filters.
    if !(*app_ptr).after_refs.is_empty() {
        let refs     = (*app_ptr).after_refs.clone();
        let lua_cap  = ThreadSafePtr(lua_safe.ptr());
        let lock_cap = Arc::clone(&lua_lock);
        web_app.after_handler(move |req, resp| dispatch_after(lua_cap.ptr(), &lock_cap, &refs, req, resp));
    }

    // Not-found handler.
    if (*app_ptr).not_found_ref != LUA_NOREF {
        let nf_ref   = (*app_ptr).not_found_ref;
        let lua_cap  = ThreadSafePtr(lua_safe.ptr());
        let lock_cap = Arc::clone(&lua_lock);
        web_app.on_not_found(move |req| {
            dispatch(lua_cap.ptr(), &lock_cap, nf_ref, req, None)
                .unwrap_or_else(WebResponse::not_found)
        });
    }

    // Error handler (covers Rust panics; Lua errors are handled inline).
    if (*app_ptr).error_handler_ref != LUA_NOREF {
        let eh_ref   = (*app_ptr).error_handler_ref;
        let lua_cap  = ThreadSafePtr(lua_safe.ptr());
        let lock_cap = Arc::clone(&lua_lock);
        web_app.on_handler_error(move |req, _| {
            dispatch(lua_cap.ptr(), &lock_cap, eh_ref, req, None)
                .unwrap_or_else(|| WebResponse::internal_error("handler error"))
        });
    }

    // Bind the TCP socket.
    let mut opts = embeddable_http_server::HttpServerOptions::default();
    opts.tcp.max_connections = max_conn;

    let server = match bind_server(&host, port, opts, Arc::new(web_app)) {
        Ok(s)  => s,
        Err(e) => {
            let msg = CString::new(format!("conduit: bind failed: {e}")).unwrap();
            luaL_error(L, msg.as_ptr());
            unreachable!()  // luaL_error performs a longjmp and never returns
        }
    };

    // Allocate server userdata.
    let ptr = lua_newuserdatauv(L, std::mem::size_of::<LuaConduitServer>(), 0);
    ptr::write(
        ptr as *mut LuaConduitServer,
        LuaConduitServer { lua: L, lua_lock, server: Some(server), running },
    );

    luaL_newmetatable(L, SERVER_MT);
    lua_pushcclosure(L, Some(server_gc), 0);
    lua_rawset_str_top(L, -2, "__gc\0");
    lua_setmetatable(L, -2);

    1
}

unsafe extern "C" fn server_gc(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    if (*srv).running.load(Ordering::SeqCst) {
        if let Some(ref s) = (*srv).server {
            s.stop_handle().stop();
        }
    }
    ptr::drop_in_place(srv);
    0
}

// ---------------------------------------------------------------------------
// server_serve(server) → nil  [blocks until stopped]
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_server_serve(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    let sp  = match (*srv).server.as_mut() {
        Some(s) => s as *mut PlatformWebServer,
        None    => { luaL_error(L, b"server is disposed\0".as_ptr() as *const c_char); unreachable!() },
    };
    (*srv).running.store(true, Ordering::SeqCst);
    let result = (*sp).serve();
    (*srv).running.store(false, Ordering::SeqCst);
    if let Err(e) = result {
        let msg = CString::new(format!("conduit server error: {e}")).unwrap();
        luaL_error(L, msg.as_ptr());
    }
    0
}

// ---------------------------------------------------------------------------
// server_serve_background(server) → nil  [non-blocking; for tests]
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_server_serve_background(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    let sp_raw = match (*srv).server.as_mut() {
        Some(s) => s as *mut PlatformWebServer,
        None    => { luaL_error(L, b"server is disposed\0".as_ptr() as *const c_char); unreachable!() },
    };
    // Wrap the raw pointer so the closure is Send.
    // SAFETY: sp is owned by the LuaConduitServer userdata which outlives this
    // thread (test teardown calls stop() and waits before releasing the userdata).
    let sp = ThreadSafePtr(sp_raw);
    let running = Arc::clone(&(*srv).running);
    running.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        let _ = (*sp.ptr()).serve();
        running.store(false, Ordering::SeqCst);
    });
    0
}

// ---------------------------------------------------------------------------
// server_stop / server_local_port / server_running / server_dispose
// ---------------------------------------------------------------------------

unsafe extern "C" fn lua_server_stop(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    if let Some(ref s) = (*srv).server { s.stop_handle().stop(); }
    0
}

unsafe extern "C" fn lua_server_local_port(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    match &(*srv).server {
        Some(s) => { lua_pushinteger(L, s.local_addr().port() as lua_Integer); 1 }
        None    => { luaL_error(L, b"server is disposed\0".as_ptr() as *const c_char); unreachable!() },
    }
}

unsafe extern "C" fn lua_server_running(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    lua_pushboolean(L, if (*srv).running.load(Ordering::SeqCst) { 1 } else { 0 });
    1
}

unsafe extern "C" fn lua_server_dispose(L: *mut lua_State) -> c_int {
    let srv = check_server(L, 1);
    if (*srv).running.load(Ordering::SeqCst) {
        luaL_error(L, b"stop the server before disposing\0".as_ptr() as *const c_char);
    }
    (*srv).server.take();
    0
}

// =============================================================================
// Dispatch — called from web-core I/O threads
// =============================================================================

/// Call Lua function `handler_ref` with the request env table (and an optional
/// string error argument for error handlers). Returns `None` for nil returns,
/// or `Some(WebResponse)` for table returns. Catches HaltError and Lua errors.
///
/// Caller must hold `lua_lock` (or this is the top-level caller that acquires it).
fn dispatch(
    L: *mut lua_State,
    lua_lock: &Arc<Mutex<()>>,
    handler_ref: i32,
    req: &WebRequest,
    error_msg: Option<&str>,
) -> Option<WebResponse> {
    let _guard = lua_lock.lock().unwrap();
    unsafe {
        let top_before = lua_gettop(L);

        lua_rawgeti(L, LUA_REGISTRYINDEX, handler_ref as lua_Integer);
        build_env(L, req);

        let nargs = if let Some(msg) = error_msg {
            push_str(L, msg);
            2
        } else {
            1
        };

        let status = lua_pcall(L, nargs, 1, 0);

        let resp = if status != 0 {
            // pcall failed — check for HaltError first.
            if is_halt_error(L, -1) {
                Some(parse_halt_error(L, -1))
            } else {
                Some(WebResponse::internal_error("unhandled Lua error in handler"))
            }
        } else {
            parse_lua_response(L, -1)
        };

        lua_settop(L, top_before);
        resp
    }
}

/// Dispatch a route handler. On Lua error (not HaltError), calls `error_handler_ref`
/// if registered. Holds the Lua lock across both calls to ensure atomicity.
fn dispatch_route(
    L: *mut lua_State,
    lua_lock: &Arc<Mutex<()>>,
    handler_ref: i32,
    error_handler_ref: i32,
    req: &WebRequest,
) -> WebResponse {
    let _guard = lua_lock.lock().unwrap();
    unsafe {
        let top_before = lua_gettop(L);

        // Call the route handler.
        lua_rawgeti(L, LUA_REGISTRYINDEX, handler_ref as lua_Integer);
        build_env(L, req);
        let status = lua_pcall(L, 1, 1, 0);

        if status != 0 {
            // Handler raised an error.
            if is_halt_error(L, -1) {
                // HaltError — send it directly.
                let resp = parse_halt_error(L, -1);
                lua_settop(L, top_before);
                return resp;
            }
            // Plain Lua error — extract the message for the error handler.
            let err_msg = get_str(L, -1).unwrap_or_else(|| "internal server error".to_string());
            lua_settop(L, top_before);

            if error_handler_ref != LUA_NOREF {
                // Call the error handler: fn(env, err_message).
                lua_rawgeti(L, LUA_REGISTRYINDEX, error_handler_ref as lua_Integer);
                build_env(L, req);
                push_str(L, &err_msg);
                let eh_status = lua_pcall(L, 2, 1, 0);
                let resp = if eh_status == 0 {
                    parse_lua_response(L, -1)
                        .unwrap_or_else(|| WebResponse::internal_error("error handler returned nil"))
                } else {
                    WebResponse::internal_error("error handler itself raised an error")
                };
                lua_settop(L, top_before);
                return resp;
            }
            return WebResponse::internal_error("unhandled Lua error in route handler");
        }

        // Handler returned normally.
        let resp = parse_lua_response(L, -1)
            .unwrap_or_else(|| WebResponse::internal_error("route handler returned nil"));
        lua_settop(L, top_before);
        resp
    }
}

fn dispatch_before(
    L: *mut lua_State,
    lua_lock: &Arc<Mutex<()>>,
    refs: &[i32],
    req: &WebRequest,
) -> Option<WebResponse> {
    for &r in refs {
        if let Some(resp) = dispatch(L, lua_lock, r, req, None) {
            return Some(resp);
        }
    }
    None
}

fn dispatch_after(
    L: *mut lua_State,
    lua_lock: &Arc<Mutex<()>>,
    refs: &[i32],
    req: &WebRequest,
    mut resp: WebResponse,
) -> WebResponse {
    for &r in refs {
        if let Some(new_resp) = dispatch(L, lua_lock, r, req, None) {
            resp = new_resp;
        }
    }
    resp
}

// ---------------------------------------------------------------------------
// HaltError detection
// ---------------------------------------------------------------------------

/// Returns true if the Lua value at `idx` is a HaltError table.
/// HaltError tables have `__conduit_halt = true`.
unsafe fn is_halt_error(L: *mut lua_State, idx: c_int) -> bool {
    if lua_type(L, idx) != LUA_TTABLE { return false; }
    let key = CString::new("__conduit_halt").unwrap();
    lua_getfield(L, idx, key.as_ptr());
    let result = lua_toboolean(L, -1) != 0;
    lua_pop(L, 1);
    result
}

/// Parse a HaltError table at `idx` → WebResponse.
/// Fields: `status` (integer), `body` (string), `headers` ({{name,val},...}).
unsafe fn parse_halt_error(L: *mut lua_State, idx: c_int) -> WebResponse {
    let status = {
        let k = CString::new("status").unwrap();
        lua_getfield(L, idx, k.as_ptr());
        let s = lua_tointeger(L, -1) as u16;
        lua_pop(L, 1);
        s
    };
    let body = {
        let k = CString::new("body").unwrap();
        lua_getfield(L, idx, k.as_ptr());
        let b = get_str(L, -1).unwrap_or_default();
        lua_pop(L, 1);
        b
    };
    let headers = {
        let k = CString::new("headers").unwrap();
        lua_getfield(L, idx, k.as_ptr());
        let h = if lua_type(L, -1) == LUA_TTABLE { parse_header_table(L, -1) } else { Vec::new() };
        lua_pop(L, 1);
        h
    };
    WebResponse {
        status,
        headers: headers.into_iter().map(|h| (h.name, h.value)).collect(),
        body: body.into_bytes(),
    }
}

// ---------------------------------------------------------------------------
// Response parsing: nil | {status, headers_array, body_string}
// ---------------------------------------------------------------------------

unsafe fn parse_lua_response(L: *mut lua_State, idx: c_int) -> Option<WebResponse> {
    let t = lua_type(L, idx);
    if t == LUA_TNIL || t == LUA_TNONE { return None; }
    if t != LUA_TTABLE { return None; }

    lua_rawgeti(L, idx, 1);
    let status = lua_tointeger(L, -1) as u16;
    lua_pop(L, 1);

    lua_rawgeti(L, idx, 2);
    let headers = if lua_type(L, -1) == LUA_TTABLE { parse_header_table(L, -1) } else { Vec::new() };
    lua_pop(L, 1);

    lua_rawgeti(L, idx, 3);
    let body = get_str(L, -1).unwrap_or_default().into_bytes();
    lua_pop(L, 1);

    Some(WebResponse {
        status,
        headers: headers.into_iter().map(|h| (h.name, h.value)).collect(),
        body,
    })
}

/// Parse a Lua table of `{{name, value}, ...}` pairs at `idx` → Vec<Header>.
/// Strips `\r` and `\n` from header values to prevent CRLF injection.
unsafe fn parse_header_table(L: *mut lua_State, idx: c_int) -> Vec<Header> {
    let mut out = Vec::new();
    let len = lua_rawlen(L, idx) as lua_Integer;
    for i in 1..=len {
        lua_rawgeti(L, idx, i);
        if lua_type(L, -1) == LUA_TTABLE {
            lua_rawgeti(L, -1, 1);
            let name = get_str(L, -1).unwrap_or_default();
            lua_pop(L, 1);
            lua_rawgeti(L, -1, 2);
            let value = get_str(L, -1).unwrap_or_default().replace('\r', "").replace('\n', "");
            lua_pop(L, 1);
            if !name.is_empty() { out.push(Header { name, value }); }
        }
        lua_pop(L, 1);
    }
    out
}

// ---------------------------------------------------------------------------
// Env table builder — mirrors Ruby/Python env keys
// ---------------------------------------------------------------------------

unsafe fn build_env(L: *mut lua_State, req: &WebRequest) {
    lua_newtable(L);
    let tbl = lua_gettop(L);

    tbl_set_str(L, tbl, "REQUEST_METHOD", req.method());
    tbl_set_str(L, tbl, "PATH_INFO",      req.path());

    let (_, query) = split_target(req.http.target());
    tbl_set_str(L, tbl, "QUERY_STRING", query);

    // route_params and query_params as Lua tables.
    push_str_map(L, &req.route_params);
    let k = CString::new("conduit.route_params").unwrap();
    lua_setfield(L, tbl, k.as_ptr());

    push_str_map(L, &req.query_params);
    let k = CString::new("conduit.query_params").unwrap();
    lua_setfield(L, tbl, k.as_ptr());

    // Headers as a Lua table with lowercase string keys.
    lua_newtable(L);
    let htbl = lua_gettop(L);
    for h in &req.http.head.headers {
        let lk = CString::new(h.name.to_ascii_lowercase()).unwrap_or_default();
        push_str(L, &h.value);
        lua_setfield(L, htbl, lk.as_ptr());
    }
    let k = CString::new("conduit.headers").unwrap();
    lua_setfield(L, tbl, k.as_ptr());

    // Body as a string.
    tbl_set_str(L, tbl, "conduit.body", &String::from_utf8_lossy(req.body()));

    // Optional content_type and content_length.
    if let Some(ct) = req.content_type() {
        tbl_set_str(L, tbl, "conduit.content_type", ct);
    }
    if let Some(cl) = req.content_length() {
        let k = CString::new("conduit.content_length").unwrap();
        lua_pushinteger(L, cl as lua_Integer);
        lua_setfield(L, tbl, k.as_ptr());
    }

    tbl_set_str(L, tbl, "REMOTE_ADDR",  &req.peer_addr().ip().to_string());
    tbl_set_str(L, tbl, "SERVER_NAME",  &req.http.connection.local_addr.ip().to_string());
}

/// Push a Lua string field: `table[key] = value` (table at `tbl` index).
unsafe fn tbl_set_str(L: *mut lua_State, tbl: c_int, key: &str, value: &str) {
    let ck = CString::new(key).unwrap_or_default();
    push_str(L, value);
    lua_setfield(L, tbl, ck.as_ptr());
}

/// Push a Rust HashMap<String,String> as a new Lua table.
unsafe fn push_str_map(L: *mut lua_State, map: &std::collections::HashMap<String, String>) {
    lua_newtable(L);
    let tbl = lua_gettop(L);
    for (k, v) in map {
        let ck = CString::new(k.as_str()).unwrap_or_default();
        push_str(L, v);
        lua_setfield(L, tbl, ck.as_ptr());
    }
}

fn split_target(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((p, q)) => (p, q),
        None         => (target, ""),
    }
}

// ---------------------------------------------------------------------------
// Small helpers for metatable setup
// ---------------------------------------------------------------------------

/// Push a Rust `&str` onto the stack as a Lua string (same as push_str but
/// as a convenience alias used in metatable construction).
unsafe fn push_cstr(L: *mut lua_State, s: &str) {
    push_str(L, s);
}

/// Set `table[key] = top_of_stack`, consuming the top value.
/// `key_with_nul` must be a `b"...\0"` byte literal.
unsafe fn lua_rawset_str_top(L: *mut lua_State, tbl: c_int, key_with_nul: &str) {
    let ck = CString::new(key_with_nul.trim_end_matches('\0')).unwrap_or_default();
    lua_setfield(L, tbl, ck.as_ptr());
}

// ---------------------------------------------------------------------------
// Platform-specific server binding
// ---------------------------------------------------------------------------

#[cfg(any(
    target_os = "macos",  target_os = "freebsd", target_os = "openbsd",
    target_os = "netbsd", target_os = "dragonfly"
))]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_kqueue((host, port), opts, app)
}

#[cfg(target_os = "linux")]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_epoll((host, port), opts, app)
}

#[cfg(target_os = "windows")]
fn bind_server(
    host: &str, port: u16,
    opts: embeddable_http_server::HttpServerOptions,
    app: Arc<WebApp>,
) -> Result<PlatformWebServer, transport_platform::PlatformError> {
    WebServer::bind_windows((host, port), opts, app)
}

// =============================================================================
// Module function table and entry point
// =============================================================================

/// Wrapper to allow a static slice of luaL_Reg (which contains raw pointers).
/// SAFETY: the pointers in luaL_Reg are all 'static string literals and
/// 'static function pointers — safe to share across threads.
struct ConduitFuncTable(&'static [luaL_Reg]);
unsafe impl Sync for ConduitFuncTable {}

/// Function table registered by `luaopen_conduit_native`.
/// The final `{null, null}` entry is the required sentinel.
static CONDUIT_FUNCS: ConduitFuncTable = ConduitFuncTable(&[
    luaL_Reg { name: b"new_app\0".as_ptr()                  as *const c_char, func: Some(lua_new_app) },
    luaL_Reg { name: b"app_add_route\0".as_ptr()            as *const c_char, func: Some(lua_app_add_route) },
    luaL_Reg { name: b"app_add_before\0".as_ptr()           as *const c_char, func: Some(lua_app_add_before) },
    luaL_Reg { name: b"app_add_after\0".as_ptr()            as *const c_char, func: Some(lua_app_add_after) },
    luaL_Reg { name: b"app_set_not_found\0".as_ptr()        as *const c_char, func: Some(lua_app_set_not_found) },
    luaL_Reg { name: b"app_set_error_handler\0".as_ptr()    as *const c_char, func: Some(lua_app_set_error_handler) },
    luaL_Reg { name: b"app_set_setting\0".as_ptr()          as *const c_char, func: Some(lua_app_set_setting) },
    luaL_Reg { name: b"app_get_setting\0".as_ptr()          as *const c_char, func: Some(lua_app_get_setting) },
    luaL_Reg { name: b"new_server\0".as_ptr()               as *const c_char, func: Some(lua_new_server) },
    luaL_Reg { name: b"server_serve\0".as_ptr()             as *const c_char, func: Some(lua_server_serve) },
    luaL_Reg { name: b"server_serve_background\0".as_ptr()  as *const c_char, func: Some(lua_server_serve_background) },
    luaL_Reg { name: b"server_stop\0".as_ptr()              as *const c_char, func: Some(lua_server_stop) },
    luaL_Reg { name: b"server_local_port\0".as_ptr()        as *const c_char, func: Some(lua_server_local_port) },
    luaL_Reg { name: b"server_running\0".as_ptr()           as *const c_char, func: Some(lua_server_running) },
    luaL_Reg { name: b"server_dispose\0".as_ptr()           as *const c_char, func: Some(lua_server_dispose) },
    luaL_Reg { name: ptr::null(),                                              func: None }, // sentinel
]);

/// Lua module entry point — called by `require("conduit_native")`.
/// Registers the CONDUIT_FUNCS table and returns it to Lua.
#[no_mangle]
pub unsafe extern "C" fn luaopen_conduit_native(L: *mut lua_State) -> c_int {
    register_lib(L, CONDUIT_FUNCS.0);
    1
}

/// Alias for `require("conduit.conduit_native")` — Lua 5.4 derives the
/// open-function name by replacing dots with underscores in the full
/// module path, yielding `luaopen_conduit_conduit_native`.
#[no_mangle]
pub unsafe extern "C" fn luaopen_conduit_conduit_native(L: *mut lua_State) -> c_int {
    luaopen_conduit_native(L)
}
