//! `irc-server-native-jni` — a JNI bridge embedding the all-Rust IRC engine
//! (`irc-net-reactor`) into the JVM (Java / Kotlin).
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  The JVM only *launches and controls* the server, so
//! this library exposes a tiny set of `native` methods over an opaque `long`
//! peer pointer:
//!
//! * `nativeNewServer(host, port, serverName, motd, operPassword, maxConnections)`
//!   → bind, return the peer pointer.
//! * `nativeServe(ptr)` — run the loop on the calling thread (blocks).
//! * `nativeServeBackground(ptr)` — run the loop on a spawned thread.
//! * `nativeStop(ptr)` — signal the loop to stop and join the background thread.
//! * `nativeRunning` / `nativeLocalHost` / `nativeLocalPort`.
//! * `nativeDisposeServer(ptr)` — stop, join, and free the peer.
//!
//! There is **no per-message callback into the JVM** (unlike `conduit-jni`,
//! which dispatches HTTP routes to Java handlers), so no thread attachment or
//! global-reference machinery is needed.
//!
//! ## Use-after-free safety
//!
//! `serve`/`serveBackground` run the blocking loop on an **owned clone** of the
//! engine (`IrcReactorServer` is `Clone` over `Arc`s).  So the background thread
//! never dereferences the peer struct and never dangles, even though
//! `nativeDisposeServer` frees the peer.  Callers must still not race
//! `dispose()` against other calls on the same handle from another thread (the
//! `long` peer is a raw pointer) — the Java facade documents single-owner
//! lifecycle, exactly as `conduit-jni` does.
//!
//! The peer pointer is validated against 0 on every call; a true data race on a
//! freed peer is the caller's responsibility.

// JNI entry points are `Java_<Pkg>_<Class>_<method>` (not snake_case) and share
// one uniform safety contract (called by the JVM with a valid peer pointer), so
// per-function `# Safety` docs would be noise.
#![allow(non_snake_case, clippy::missing_safety_doc, clippy::unused_unit)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use irc_net_reactor::{IrcConfig, IrcReactorServer};
use jni_bridge::{
    jboolean, jclass, jint, jlong, jni_get_string_utf, jni_new_string_utf, jni_throw_new, jstring,
    JNIEnv,
};

/// Return `$ret` if the peer pointer is null (0).
macro_rules! guard_ptr {
    ($ptr:expr, $ret:expr) => {
        if $ptr == 0 {
            return $ret;
        }
    };
}

/// The Rust state behind the `long` peer pointer.
struct NativeServer {
    /// The bound engine.  Kept as a control handle; the serve loop runs a clone.
    server: Option<IrcReactorServer>,
    local_host: String,
    local_port: u16,
    running: Arc<AtomicBool>,
    bg: Option<std::thread::JoinHandle<()>>,
}

// SAFETY: only an owned clone of `IrcReactorServer` (Arc-based, Send) is moved
// into the background thread; the peer struct itself stays with its owner.
unsafe impl Send for NativeServer {}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeNewServer(
    env: *mut JNIEnv,
    _class: jclass,
    host: jstring,
    port: jint,
    server_name: jstring,
    motd: jstring,
    oper_password: jstring,
    max_connections: jint,
) -> jlong {
    let host = jni_get_string_utf(env, host).unwrap_or_else(|| "127.0.0.1".to_string());
    let port = if (0..=65535).contains(&port) {
        port as u16
    } else {
        jni_throw_new(
            env,
            "java/lang/IllegalArgumentException",
            "port must be between 0 and 65535",
        );
        return 0;
    };
    let server_name =
        jni_get_string_utf(env, server_name).unwrap_or_else(|| "irc.local".to_string());
    // MOTD is passed as a single newline-joined string to avoid marshalling a
    // String[] across JNI; split it back into lines here.
    let motd: Vec<String> = jni_get_string_utf(env, motd)
        .map(|joined| {
            joined
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(|line| line.to_string())
                .collect()
        })
        .unwrap_or_default();
    let oper_password = jni_get_string_utf(env, oper_password).unwrap_or_default();
    let max_connections = if max_connections >= 1 {
        max_connections as usize
    } else {
        jni_throw_new(
            env,
            "java/lang/IllegalArgumentException",
            "maxConnections must be >= 1",
        );
        return 0;
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
        Err(e) => {
            jni_throw_new(
                env,
                "java/lang/RuntimeException",
                &format!("irc_server_native_jni: bind failed: {e}"),
            );
            return 0;
        }
    };

    let addr = server.local_addr();
    let native = NativeServer {
        server: Some(server),
        local_host: addr.ip().to_string(),
        local_port: addr.port(),
        running: Arc::new(AtomicBool::new(false)),
        bg: None,
    };
    Box::into_raw(Box::new(native)) as jlong
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeServe(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    if srv.running.load(Ordering::SeqCst) {
        return; // already serving
    }
    let engine = match srv.server.as_ref() {
        Some(server) => server.clone(),
        None => return,
    };
    srv.running.store(true, Ordering::SeqCst);
    let _ = engine.serve();
    srv.running.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeServeBackground(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    if srv.running.load(Ordering::SeqCst) {
        return;
    }
    let engine = match srv.server.as_ref() {
        Some(server) => server.clone(),
        None => return,
    };
    let running = Arc::clone(&srv.running);
    running.store(true, Ordering::SeqCst);
    srv.bg = Some(std::thread::spawn(move || {
        let _ = engine.serve();
        running.store(false, Ordering::SeqCst);
    }));
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeStop(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    if let Some(server) = srv.server.as_ref() {
        server.stop();
    }
    if let Some(handle) = srv.bg.take() {
        let _ = handle.join();
    }
    srv.running.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeRunning(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) -> jboolean {
    guard_ptr!(server_ptr, 0);
    let srv = &*(server_ptr as *mut NativeServer);
    if srv.running.load(Ordering::SeqCst) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeLocalHost(
    env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) -> jstring {
    guard_ptr!(server_ptr, std::ptr::null_mut());
    let srv = &*(server_ptr as *mut NativeServer);
    jni_new_string_utf(env, &srv.local_host)
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeLocalPort(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) -> jint {
    guard_ptr!(server_ptr, 0);
    let srv = &*(server_ptr as *mut NativeServer);
    srv.local_port as jint
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_codingadventures_ircserver_Native_nativeDisposeServer(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    if server_ptr == 0 {
        return;
    }
    let mut srv = Box::from_raw(server_ptr as *mut NativeServer);
    // Stop and join before the box (and its engine clone) is dropped.
    if let Some(server) = srv.server.as_ref() {
        server.stop();
    }
    if let Some(handle) = srv.bg.take() {
        let _ = handle.join();
    }
    // `srv` dropped here.
}
