//! `irc_server_native` — a Ruby C extension that embeds the all-Rust IRC engine
//! (`irc-net-reactor`) and exposes a small lifecycle control surface.
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  Ruby's only job is to *launch and control* the
//! server, so `CodingAdventures::IrcServerNative::Server` exposes:
//!
//! | method         | meaning                                                 |
//! |----------------|---------------------------------------------------------|
//! | `new(host, port, server_name, motd, oper_password, max_connections)` | build + bind |
//! | `serve`        | run the event loop, **releasing the GVL**               |
//! | `stop`         | signal the loop to stop (callable from another thread)  |
//! | `local_host` / `local_port` | the bound address                          |
//! | `running?`     | is the loop currently running?                          |
//! | `dispose`      | drop the engine (must be stopped first)                 |
//!
//! Unlike `conduit`, there is no per-request dispatch back into Ruby.
//!
//! ## GVL discipline and resilience
//!
//! `serve` releases the GVL via `rb_thread_call_without_gvl` around the blocking
//! `IrcReactorServer::serve()`, so a *different* Ruby thread can call `stop`
//! (which only touches the thread-safe stop handle) while the loop runs.
//!
//! Two safety measures mirror the Python binding:
//!
//! * `serve` runs on an **owned clone** of the engine (cheap — `IrcReactorServer`
//!   is `Clone` over `Arc`s), so even if another thread disposed the stored copy
//!   the runtime stays alive: no use-after-free.
//! * the `running` flag is set to true **before** the GVL is released, and
//!   `dispose` (which runs only with the GVL held) refuses to free a live server.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use irc_net_reactor::{IrcConfig, IrcReactorServer};
use ruby_bridge::{
    bool_to_rb, define_alloc_func, define_class_under, define_method_raw, define_module,
    define_module_under, nil_value, object_class, raise_arg_error, raise_error, rb_num2long,
    standard_error_class, str_from_rb, str_to_rb, unwrap_data_mut, usize_to_rb, vec_str_from_rb,
    wrap_data, VALUE,
};

// `rb_thread_call_without_gvl` is part of the stable Ruby C API but is not
// wrapped by ruby-bridge (its function-pointer signature is use-site specific).
extern "C" {
    fn rb_thread_call_without_gvl(
        func: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        data1: *mut c_void,
        unblock: Option<unsafe extern "C" fn(*mut c_void)>,
        data2: *mut c_void,
    ) -> *mut c_void;
}

/// The custom exception class raised on server errors, set up in `Init_`.
static mut SERVER_ERROR: VALUE = 0;

fn raise_server_error(message: &str) -> ! {
    // SAFETY: SERVER_ERROR is assigned once in Init_irc_server_native before any
    // method can run, and only read thereafter.
    let class = unsafe { SERVER_ERROR };
    raise_error(class, message)
}

/// The Rust state wrapped inside each Ruby `Server` object.  `running` mirrors
/// the engine's serve state so `dispose` can refuse to free a live server.
struct RubyIrcServer {
    server: Option<IrcReactorServer>,
    running: Arc<AtomicBool>,
}

/// Ruby allocates the object (empty) before calling `initialize`.
unsafe extern "C" fn server_alloc(klass: VALUE) -> VALUE {
    wrap_data(
        klass,
        RubyIrcServer {
            server: None,
            running: Arc::new(AtomicBool::new(false)),
        },
    )
}

// ── argument helpers ───────────────────────────────────────────────────────

fn string_from_rb(value: VALUE, message: &str) -> String {
    str_from_rb(value).unwrap_or_else(|| raise_arg_error(message))
}

fn usize_from_rb(value: VALUE, message: &str) -> usize {
    // rb_num2long raises a Ruby TypeError itself if `value` is not numeric.
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

// ── initialize ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
extern "C" fn server_initialize(
    self_val: VALUE,
    host_val: VALUE,
    port_val: VALUE,
    name_val: VALUE,
    motd_val: VALUE,
    oper_val: VALUE,
    max_val: VALUE,
) -> VALUE {
    let host = string_from_rb(host_val, "host must be a String");
    let port = u16_from_rb(port_val, "port must be between 0 and 65535");
    let server_name = string_from_rb(name_val, "server_name must be a String");
    let motd = vec_str_from_rb(motd_val);
    let oper_password = string_from_rb(oper_val, "oper_password must be a String");
    let max_connections = usize_from_rb(max_val, "max_connections must be >= 1");
    if max_connections < 1 {
        raise_arg_error("max_connections must be >= 1");
    }

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
        Err(err) => raise_server_error(&format!("failed to bind IRC server: {err}")),
    };

    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    slot.server = Some(server);
    self_val
}

// ── serve (GVL released) ─────────────────────────────────────────────────────

/// Carries the owned engine clone and the result across the GVL boundary.
struct ServeCall {
    server: IrcReactorServer,
    ok: bool,
    error: Option<String>,
}

/// Runs with the GVL released.  Touches only the owned clone — never the Ruby
/// object — so it cannot race `stop`/`dispose` (which run with the GVL held).
unsafe extern "C" fn serve_without_gvl(data: *mut c_void) -> *mut c_void {
    let call = &mut *(data as *mut ServeCall);
    match call.server.serve() {
        Ok(()) => call.ok = true,
        Err(err) => {
            call.ok = false;
            call.error = Some(format!("IRC server error: {err}"));
        }
    }
    ptr::null_mut()
}

extern "C" fn server_serve(self_val: VALUE) -> VALUE {
    // Clone the engine and grab the running flag while we (implicitly) hold the
    // GVL, then release it for the blocking run.  The owned clone keeps the
    // runtime alive even if another thread disposes the stored copy.
    let (server, running) = {
        let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
        let server = match slot.server.as_ref() {
            Some(server) => server.clone(),
            None => raise_server_error("server has been disposed"),
        };
        (server, Arc::clone(&slot.running))
    };

    // Set running BEFORE releasing the GVL so `dispose` always observes it.
    running.store(true, Ordering::SeqCst);
    let mut call = ServeCall {
        server,
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
    running.store(false, Ordering::SeqCst);

    if call.ok {
        nil_value()
    } else {
        let message = call
            .error
            .take()
            .unwrap_or_else(|| "IRC server error".to_string());
        // Drop the owned clone before the longjmp in raise_* skips destructors.
        drop(call);
        raise_server_error(&message);
    }
}

// ── stop / dispose / running? / local_host / local_port ──────────────────────

extern "C" fn server_stop(self_val: VALUE) -> VALUE {
    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    match slot.server.as_ref() {
        Some(server) => {
            server.stop();
            nil_value()
        }
        None => raise_server_error("server has been disposed"),
    }
}

extern "C" fn server_dispose(self_val: VALUE) -> VALUE {
    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    if slot.running.load(Ordering::SeqCst) {
        raise_server_error("cannot dispose a running server; call stop first");
    }
    slot.server.take();
    nil_value()
}

extern "C" fn server_running(self_val: VALUE) -> VALUE {
    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    bool_to_rb(slot.running.load(Ordering::SeqCst))
}

extern "C" fn server_local_host(self_val: VALUE) -> VALUE {
    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    match slot.server.as_ref() {
        Some(server) => str_to_rb(&server.local_addr().ip().to_string()),
        None => raise_server_error("server has been disposed"),
    }
}

extern "C" fn server_local_port(self_val: VALUE) -> VALUE {
    let slot = unsafe { unwrap_data_mut::<RubyIrcServer>(self_val) };
    match slot.server.as_ref() {
        Some(server) => usize_to_rb(server.local_addr().port() as usize),
        None => raise_server_error("server has been disposed"),
    }
}

// ── module init ──────────────────────────────────────────────────────────────

/// Entry point — Ruby calls this when the extension is `require`d.
///
/// # Safety
/// Called once by the Ruby loader on the main thread.
#[no_mangle]
pub extern "C" fn Init_irc_server_native() {
    let coding_adventures = define_module("CodingAdventures");
    let module = define_module_under(coding_adventures, "IrcServerNative");

    let error_class = define_class_under(module, "Error", standard_error_class());
    // SAFETY: assigned once here before any method can run.
    unsafe { SERVER_ERROR = error_class };

    let server_class = define_class_under(module, "NativeServer", object_class());
    define_alloc_func(server_class, server_alloc);
    define_method_raw(
        server_class,
        "initialize",
        server_initialize as *const c_void,
        6,
    );
    define_method_raw(server_class, "serve", server_serve as *const c_void, 0);
    define_method_raw(server_class, "stop", server_stop as *const c_void, 0);
    define_method_raw(server_class, "dispose", server_dispose as *const c_void, 0);
    define_method_raw(server_class, "running?", server_running as *const c_void, 0);
    define_method_raw(
        server_class,
        "local_host",
        server_local_host as *const c_void,
        0,
    );
    define_method_raw(
        server_class,
        "local_port",
        server_local_port as *const c_void,
        0,
    );
}
