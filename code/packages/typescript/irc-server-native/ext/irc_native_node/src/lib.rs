//! `irc_native_node` — a Node.js N-API addon that embeds the all-Rust IRC engine
//! (`irc-net-reactor`) and exposes a small lifecycle control surface.
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  Node's only job is to *launch and control* the
//! server, so the module exports a single `newServer(...)` factory that returns
//! an object with methods:
//!
//! | method        | meaning                                                   |
//! |---------------|-----------------------------------------------------------|
//! | `serve()`     | start the event loop on a **background thread** (non-blocking) |
//! | `stop()`      | signal the loop to stop and join the thread               |
//! | `localHost()` / `localPort()` | the bound address                         |
//! | `running()`   | is the loop currently running?                            |
//! | `dispose()`   | drop the engine (must be stopped first)                   |
//!
//! Because there is **no per-request callback into JavaScript** (unlike
//! `conduit`, which dispatches HTTP routes to JS handlers), this binding needs
//! none of N-API's threadsafe-function machinery — just `std::thread::spawn`.
//!
//! ## Threading & the V8 event loop
//!
//! Node's `serve()` must not block the single V8 thread, so the blocking
//! `IrcReactorServer::serve()` runs on a spawned background OS thread; `serve()`
//! returns to JS immediately.  All of `serve`/`stop`/`dispose` are called only
//! on the V8 thread (JavaScript is single-threaded), so they never race each
//! other — the only other thread is the one running the cloned engine.
//!
//! ## Use-after-free safety
//!
//! The background thread runs an **owned clone** of the engine (`IrcReactorServer`
//! is `Clone` over `Arc`s).  So even though `dispose()` drops the struct's copy,
//! the runtime the background thread is using stays alive — there is no dangling
//! engine.  `dispose()` additionally refuses to run while the server is running.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use irc_net_reactor::{IrcConfig, IrcReactorServer};
use node_bridge::{
    array_get, array_len, bool_to_js, create_function, get_cb_info, i32_from_js, is_array,
    napi_callback_info, napi_env, napi_value, object_new, set_named_property, str_from_js,
    str_to_js, throw_error, undefined, unwrap_data_mut, usize_to_js, wrap_data,
};

/// The Rust state wrapped inside the JS server object.
struct NativeServer {
    /// The bound engine.  `None` after `dispose()`.  Kept as a control handle
    /// (for `stop`); the background thread runs its own clone.
    server: Option<IrcReactorServer>,
    local_host: String,
    local_port: u16,
    running: Arc<AtomicBool>,
    bg_thread: Option<JoinHandle<()>>,
}

impl Drop for NativeServer {
    fn drop(&mut self) {
        // When the JS object is garbage-collected, make sure the background
        // event loop is stopped and joined rather than leaked.
        if let Some(server) = self.server.as_ref() {
            server.stop();
        }
        if let Some(handle) = self.bg_thread.take() {
            let _ = handle.join();
        }
    }
}

/// `newServer(host, port, serverName, motd, operPassword, maxConnections)` →
/// a server object with the lifecycle methods attached.
unsafe extern "C" fn js_new_server(env: napi_env, info: napi_callback_info) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 6);
    if args.len() < 6 {
        throw_error(
            env,
            "newServer(host, port, serverName, motd, operPassword, maxConnections)",
        );
        return undefined(env);
    }

    let host = str_from_js(env, args[0]).unwrap_or_else(|| "127.0.0.1".to_string());
    let port = match i32_from_js(env, args[1]) {
        Some(p) if (0..=65535).contains(&p) => p as u16,
        _ => {
            throw_error(env, "port must be between 0 and 65535");
            return undefined(env);
        }
    };
    let server_name = str_from_js(env, args[2]).unwrap_or_else(|| "irc.local".to_string());

    // motd: a JS array of strings.
    let motd = if is_array(env, args[3]) {
        let len = array_len(env, args[3]);
        let mut lines = Vec::with_capacity(len as usize);
        for i in 0..len {
            if let Some(line) = str_from_js(env, array_get(env, args[3], i)) {
                lines.push(line);
            }
        }
        lines
    } else {
        Vec::new()
    };

    let oper_password = str_from_js(env, args[4]).unwrap_or_default();
    let max_connections = match i32_from_js(env, args[5]) {
        Some(m) if m >= 1 => m as usize,
        _ => {
            throw_error(env, "maxConnections must be >= 1");
            return undefined(env);
        }
    };

    let config = IrcConfig {
        host,
        port,
        server_name,
        motd,
        oper_password,
        max_connections,
    };
    let server = match IrcReactorServer::bind(config) {
        Ok(server) => server,
        Err(err) => {
            throw_error(env, &format!("failed to bind IRC server: {err}"));
            return undefined(env);
        }
    };

    let addr = server.local_addr();
    let native = NativeServer {
        server: Some(server),
        local_host: addr.ip().to_string(),
        local_port: addr.port(),
        running: Arc::new(AtomicBool::new(false)),
        bg_thread: None,
    };

    let obj = object_new(env);
    wrap_data(env, obj, native);
    set_named_property(
        env,
        obj,
        "serve",
        create_function(env, "serve", Some(js_serve)),
    );
    set_named_property(
        env,
        obj,
        "stop",
        create_function(env, "stop", Some(js_stop)),
    );
    set_named_property(
        env,
        obj,
        "running",
        create_function(env, "running", Some(js_running)),
    );
    set_named_property(
        env,
        obj,
        "localHost",
        create_function(env, "localHost", Some(js_local_host)),
    );
    set_named_property(
        env,
        obj,
        "localPort",
        create_function(env, "localPort", Some(js_local_port)),
    );
    set_named_property(
        env,
        obj,
        "dispose",
        create_function(env, "dispose", Some(js_dispose)),
    );
    obj
}

/// Start the event loop on a background thread; return immediately.
unsafe extern "C" fn js_serve(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &mut *unwrap_data_mut::<NativeServer>(env, this);

    if srv.running.load(Ordering::SeqCst) {
        return undefined(env); // already serving — idempotent
    }

    // Run the blocking loop on an OWNED clone so a later dispose can't free the
    // engine out from under the background thread.
    let engine = match srv.server.as_ref() {
        Some(server) => server.clone(),
        None => {
            throw_error(env, "server has been disposed");
            return undefined(env);
        }
    };

    let running = Arc::clone(&srv.running);
    running.store(true, Ordering::SeqCst);
    srv.bg_thread = Some(std::thread::spawn(move || {
        let _ = engine.serve();
        running.store(false, Ordering::SeqCst);
    }));

    undefined(env)
}

/// Signal the loop to stop and join the background thread.
unsafe extern "C" fn js_stop(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &mut *unwrap_data_mut::<NativeServer>(env, this);

    if let Some(server) = srv.server.as_ref() {
        server.stop();
    }
    if let Some(handle) = srv.bg_thread.take() {
        let _ = handle.join();
    }
    srv.running.store(false, Ordering::SeqCst);
    undefined(env)
}

/// Drop the engine (refused while running).
unsafe extern "C" fn js_dispose(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &mut *unwrap_data_mut::<NativeServer>(env, this);

    if srv.running.load(Ordering::SeqCst) {
        throw_error(env, "cannot dispose a running server; call stop() first");
        return undefined(env);
    }
    srv.server = None;
    undefined(env)
}

unsafe extern "C" fn js_running(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &*unwrap_data_mut::<NativeServer>(env, this);
    bool_to_js(env, srv.running.load(Ordering::SeqCst))
}

unsafe extern "C" fn js_local_host(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &*unwrap_data_mut::<NativeServer>(env, this);
    str_to_js(env, &srv.local_host)
}

unsafe extern "C" fn js_local_port(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _) = get_cb_info(env, info, 0);
    let srv = &*unwrap_data_mut::<NativeServer>(env, this);
    usize_to_js(env, srv.local_port as usize)
}

/// N-API entry point — Node calls this when the addon is `require`d.
///
/// # Safety
/// Invoked once by the Node.js loader on the V8 main thread.
#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(env: napi_env, exports: napi_value) -> napi_value {
    set_named_property(
        env,
        exports,
        "newServer",
        create_function(env, "newServer", Some(js_new_server)),
    );
    exports
}
