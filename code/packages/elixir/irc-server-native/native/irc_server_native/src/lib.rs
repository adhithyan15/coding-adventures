//! `irc_server_native` — an Erlang NIF embedding the all-Rust IRC engine
//! (`irc-net-reactor`) into the BEAM (Elixir/Erlang).
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  Elixir only *launches and controls* the server, so
//! this NIF exposes a small set of functions over a single BEAM resource:
//! `new_server`, `server_serve` (dirty I/O — blocks), `server_serve_background`,
//! `server_stop`, `server_running`, `server_local_host`, `server_local_port`.
//!
//! Unlike `conduit`'s NIF, there is **no per-request dispatch back into Elixir**
//! (no routes, no dispatcher pid), so the binding is pure lifecycle control.
//!
//! ## Use-after-free safety
//!
//! The blocking loop runs on an **owned clone** of the engine (`IrcReactorServer`
//! is `Clone` over `Arc`s).  The resource keeps its own copy as a control handle
//! (for `stop` and `local_*`), so the background thread never dangles even when
//! the resource is GC'd — the destructor stops and joins before dropping.

// NIF tables use nul-terminated byte-string literals (`b"...\0"`) for C `char*`
// fields, matching the `conduit_native` NIF; clippy's c-string-literal lint is noise here.
#![allow(clippy::manual_c_str_literals)]

use erl_nif_bridge::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use irc_net_reactor::{IrcConfig, IrcReactorServer};

/// The mutable interior of a server resource.  A single BEAM resource can be
/// driven concurrently from multiple Elixir processes, so all access to these
/// fields — and the `running` check-and-set — happens under [`NativeServer::inner`]'s
/// `Mutex`; an `AtomicBool` alone would not serialize the field mutations.
struct ServerInner {
    /// The bound engine, kept as a control handle; the serve loop runs a clone.
    server: Option<IrcReactorServer>,
    bg_thread: Option<std::thread::JoinHandle<()>>,
}

/// State held inside the BEAM `NativeServer` resource.
struct NativeServer {
    inner: Mutex<ServerInner>,
    /// Shared with the background serve thread, which clears it on exit, so it
    /// stays an atomic outside the mutex.
    running: Arc<AtomicBool>,
    // Immutable after construction — read without locking.
    local_host: String,
    local_port: u16,
}

/// Lock the resource interior, recovering from a poisoned mutex rather than
/// re-panicking across the NIF boundary (a panic into the BEAM is UB).
fn lock_inner(srv: &NativeServer) -> std::sync::MutexGuard<'_, ServerInner> {
    srv.inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// Resource type handle, registered on NIF load.
static SERVER_RTYPE: OnceLock<usize> = OnceLock::new();

fn server_rtype() -> *mut ErlNifResourceType {
    *SERVER_RTYPE
        .get()
        .expect("irc_server_native NIF not loaded") as *mut ErlNifResourceType
}

/// GC destructor: stop the loop, join the background thread, then drop the value.
unsafe extern "C" fn server_dtor(_env: ErlNifEnv, obj: *mut c_void) {
    let srv_ptr = obj as *mut NativeServer;
    // The destructor runs only once the last reference is gone, so no NIF call is
    // touching the resource concurrently. Stop + join (releasing the lock before
    // the join) then drop the value in place — the BEAM owns the memory.
    let handle = {
        let srv = &*srv_ptr;
        let mut inner = lock_inner(srv);
        if let Some(server) = inner.server.as_ref() {
            server.stop();
        }
        inner.bg_thread.take()
    };
    if let Some(handle) = handle {
        let _ = handle.join();
    }
    std::ptr::drop_in_place(srv_ptr);
}

unsafe extern "C" fn nif_load(
    env: ErlNifEnv,
    _priv: *mut *mut c_void,
    _info: ERL_NIF_TERM,
) -> c_int {
    let module = b"Elixir.CodingAdventures.IrcServerNative.Native\0";
    let srv_name = b"NativeServer\0";
    let mut tried = 0;
    let srv_rt = enif_open_resource_type(
        env,
        module.as_ptr() as *const _,
        srv_name.as_ptr() as *const _,
        Some(server_dtor),
        ERL_NIF_RT_CREATE,
        &mut tried,
    );
    if srv_rt.is_null() {
        return 1;
    }
    let _ = SERVER_RTYPE.set(srv_rt as usize);
    0
}

unsafe fn argv_slice<'a>(argv: *const ERL_NIF_TERM, argc: c_int) -> &'a [ERL_NIF_TERM] {
    if argv.is_null() || argc <= 0 {
        return &[];
    }
    std::slice::from_raw_parts(argv, argc as usize)
}

// ── new_server(host, port, server_name, motd, oper_password, max_connections) ──

unsafe extern "C" fn nif_new_server(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 6 {
        return badarg(env);
    }

    let host = match binary_to_string(env, args[0]) {
        Some(s) => s,
        None => return badarg(env),
    };
    let port = match get_i64(env, args[1]) {
        Some(v) if (0..=65535).contains(&v) => v as u16,
        _ => return badarg(env),
    };
    let server_name = match binary_to_string(env, args[2]) {
        Some(s) => s,
        None => return badarg(env),
    };
    // MOTD arrives as a single newline-joined binary; split it back into lines.
    let motd: Vec<String> = match binary_to_string(env, args[3]) {
        Some(joined) => joined
            .split('\n')
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect(),
        None => return badarg(env),
    };
    let oper_password = match binary_to_string(env, args[4]) {
        Some(s) => s,
        None => return badarg(env),
    };
    let max_connections = match get_i64(env, args[5]) {
        Some(v) if v >= 1 => v as usize,
        _ => return badarg(env),
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
        Err(_) => return badarg(env),
    };
    let addr = server.local_addr();
    let native = NativeServer {
        local_host: addr.ip().to_string(),
        local_port: addr.port(),
        running: Arc::new(AtomicBool::new(false)),
        inner: Mutex::new(ServerInner {
            server: Some(server),
            bg_thread: None,
        }),
    };
    wrap_resource::<NativeServer>(env, server_rtype(), native)
}

// ── server_serve(server) — blocks (dirty I/O scheduler) ──────────────────────

unsafe extern "C" fn nif_server_serve(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    let srv = &*srv_ptr;
    // Clone the engine and flip `running` under the lock, then release it BEFORE
    // the blocking serve — otherwise a concurrent server_stop could never take
    // the lock to signal us, deadlocking.
    let engine = {
        let inner = lock_inner(srv);
        if srv.running.load(Ordering::SeqCst) {
            return atom(env, "ok");
        }
        match inner.server.as_ref() {
            Some(server) => {
                srv.running.store(true, Ordering::SeqCst);
                server.clone()
            }
            None => return badarg(env),
        }
    };
    let _ = engine.serve();
    srv.running.store(false, Ordering::SeqCst);
    atom(env, "ok")
}

// ── server_serve_background(server) — spawn a thread, return immediately ──────

unsafe extern "C" fn nif_server_serve_background(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    let srv = &*srv_ptr;
    let mut inner = lock_inner(srv);
    if srv.running.load(Ordering::SeqCst) {
        return atom(env, "ok");
    }
    let engine = match inner.server.as_ref() {
        Some(server) => server.clone(),
        None => return badarg(env),
    };
    let running = Arc::clone(&srv.running);
    running.store(true, Ordering::SeqCst);
    let handle = std::thread::spawn(move || {
        let _ = engine.serve();
        running.store(false, Ordering::SeqCst);
    });
    inner.bg_thread = Some(handle);
    atom(env, "ok")
}

unsafe extern "C" fn nif_server_stop(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    let srv = &*srv_ptr;
    // Signal stop and take the join handle under the lock, then release it before
    // joining (the background thread never touches the lock, so this can't deadlock).
    let handle = {
        let mut inner = lock_inner(srv);
        if let Some(server) = inner.server.as_ref() {
            server.stop();
        }
        inner.bg_thread.take()
    };
    if let Some(handle) = handle {
        let _ = handle.join();
    }
    srv.running.store(false, Ordering::SeqCst);
    atom(env, "ok")
}

unsafe extern "C" fn nif_server_running(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    let running = (*srv_ptr).running.load(Ordering::SeqCst);
    atom(env, if running { "true" } else { "false" })
}

unsafe extern "C" fn nif_server_local_host(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    str_to_binary(env, &(*srv_ptr).local_host)
}

unsafe extern "C" fn nif_server_local_port(
    env: ErlNifEnv,
    argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let args = argv_slice(argv, argc);
    if args.len() != 1 {
        return badarg(env);
    }
    let srv_ptr = match unwrap_resource::<NativeServer>(env, args[0], server_rtype()) {
        Some(p) => p,
        None => return badarg(env),
    };
    make_i64(env, (*srv_ptr).local_port as i64)
}

// ── NIF entry table ──────────────────────────────────────────────────────────

// `server_serve` is dirty I/O (flags = 2 = ERL_NIF_DIRTY_JOB_IO_BOUND) because it
// blocks for the server's lifetime; everything else runs on a normal scheduler.
struct FuncTable([ErlNifFunc; 7]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    ErlNifFunc {
        name: b"new_server\0".as_ptr() as *const _,
        arity: 6,
        fptr: nif_new_server,
        flags: 0,
    },
    ErlNifFunc {
        name: b"server_serve\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_serve,
        flags: 2,
    },
    ErlNifFunc {
        name: b"server_serve_background\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_serve_background,
        flags: 0,
    },
    ErlNifFunc {
        name: b"server_stop\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_stop,
        flags: 0,
    },
    ErlNifFunc {
        name: b"server_running\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_running,
        flags: 0,
    },
    ErlNifFunc {
        name: b"server_local_host\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_local_host,
        flags: 0,
    },
    ErlNifFunc {
        name: b"server_local_port\0".as_ptr() as *const _,
        arity: 1,
        fptr: nif_server_local_port,
        flags: 0,
    },
]);

static MODULE_NAME_BYTES: &[u8] = b"Elixir.CodingAdventures.IrcServerNative.Native\0";
static VM_VARIANT_BYTES: &[u8] = b"beam.vanilla\0";
static MIN_ERTS_BYTES: &[u8] = b"erts-13.0\0";

struct NifEntry(ErlNifEntry);
unsafe impl Sync for NifEntry {}

static NIF_ENTRY: NifEntry = NifEntry(ErlNifEntry {
    major: ERL_NIF_MAJOR_VERSION,
    minor: ERL_NIF_MINOR_VERSION,
    name: MODULE_NAME_BYTES.as_ptr() as *const c_char,
    num_of_funcs: 7,
    funcs: FUNCS.0.as_ptr(),
    load: Some(nif_load),
    reload: None,
    upgrade: None,
    unload: None,
    vm_variant: VM_VARIANT_BYTES.as_ptr() as *const c_char,
    options: 0,
    sizeof_ErlNifResourceTypeInit: 0,
    min_erts: MIN_ERTS_BYTES.as_ptr() as *const c_char,
});

/// The BEAM resolves this symbol when `:erlang.load_nif/2` loads the library.
///
/// # Safety
/// Called once by the BEAM loader.
#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const ErlNifEntry {
    &NIF_ENTRY.0
}
