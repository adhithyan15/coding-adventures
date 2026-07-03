//! `irc-server-capi` — a reusable **C ABI** for the all-Rust IRC engine
//! (`irc-net-reactor`).
//!
//! ## Why a C ABI?
//!
//! The Python/Ruby/Node/JNI/Erlang/Perl bindings each speak their host VM's own
//! native protocol (CPython C-API, N-API, JNI, …) through a dedicated bridge
//! crate. Swift has no such bridge in this repo — but Swift (like C, C++, Go-cgo,
//! Dart-FFI, C#-P/Invoke, Zig, …) speaks the **plain C ABI** fluently. So instead
//! of a Swift-specific bridge, we expose the engine through a flat `extern "C"`
//! surface that *any* C-FFI language can import. The Swift package
//! (`code/packages/swift/IrcServerNative`) is the first consumer.
//!
//! ## The control surface (no callbacks)
//!
//! Because **all** IRC and TCP logic lives in Rust, the binding is a pure
//! lifecycle controller — create, serve, stop — with no per-message callback back
//! into the host language. The ABI is therefore tiny:
//!
//! | function                       | meaning                                        |
//! |--------------------------------|------------------------------------------------|
//! | `irc_server_new`               | bind a server, return an opaque handle (or NULL)|
//! | `irc_server_serve`             | run the loop on the **calling** thread (blocks) |
//! | `irc_server_serve_background`  | run the loop on a background Rust thread         |
//! | `irc_server_stop`              | signal stop + join the background thread         |
//! | `irc_server_running`           | is the loop running?                            |
//! | `irc_server_local_host`        | bound IP as a heap C string (caller frees)      |
//! | `irc_server_local_port`        | bound TCP port (the OS port when bound to 0)    |
//! | `irc_server_string_free`       | free a string returned by this library          |
//! | `irc_server_free`              | stop, join, and free the handle                 |
//!
//! ## Trust boundary & safety
//!
//! Every pointer that crosses this boundary is untrusted:
//!
//! * **Strings** are validated as UTF-8 (`CStr::to_str`); invalid or NULL inputs
//!   fall back to safe defaults rather than feeding raw bytes downstream.
//! * **Numbers** are clamped (`max_connections >= 1`); `port` is a `u16` so it is
//!   already in range by construction.
//! * **Every function** wraps its body in `catch_unwind` — a Rust panic must never
//!   unwind across the C ABI (that is undefined behaviour).
//! * **`serve`/`serve_background`** run an **owned clone** of the engine (`Clone`
//!   over `Arc`s), so the background thread never dereferences the handle and the
//!   handle can't dangle out from under it.
//! * **Cross-thread shutdown:** `irc_server_stop` (and `irc_server_running` /
//!   `irc_server_local_*`) may be called from a thread *other* than the one
//!   blocked in `irc_server_serve` — every entry point takes only a shared
//!   `&*srv` reference, and all shared state is atomic or `Mutex`-guarded, so no
//!   two calls ever form aliasing `&mut`s.
//! * **Ownership contract:** strings from `irc_server_local_host` must be returned
//!   to `irc_server_string_free`; the handle from `irc_server_new` must be returned
//!   to `irc_server_free` exactly once, and that free must **happen-after** every
//!   other call on the handle has returned (in particular, a foreground
//!   `irc_server_serve` must have returned first — call `irc_server_stop`, then
//!   free). This is the standard C ownership contract: don't free an object while
//!   another thread is still inside a call on it.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use irc_net_reactor::{IrcConfig, IrcReactorServer};

/// The Rust state behind the opaque handle.
///
/// `local_host`/`port` are captured at bind time so they remain readable after
/// the engine has been moved onto a background serve thread.
pub struct CapiServer {
    /// The bound engine, kept as a control handle; the serve loop runs a clone.
    /// Set once at construction and only ever *read* afterwards (`.clone()` /
    /// `.as_ref()`), so concurrent shared access from serve/stop threads is safe.
    server: Option<IrcReactorServer>,
    local_host: String,
    port: u16,
    /// Whether a serve loop is active. Atomic, so it is safe to read/write from
    /// the serving thread and a stopping thread simultaneously.
    running: Arc<AtomicBool>,
    /// The background serve thread's join handle. This is the only field mutated
    /// after construction (set by `serve_background`, taken by `stop`/`free`), so
    /// it is guarded by a `Mutex` to keep those calls race-free across threads.
    bg: Mutex<Option<JoinHandle<()>>>,
}

// SAFETY: `IrcReactorServer` is `Send + Sync` (every field is an `Arc<Mutex<…>>`
// or `Arc<Atomic…>`), so a clone may be moved onto the background thread AND the
// handle may be shared (via `&*srv`) across the serving and stopping threads.
// All post-construction mutation goes through the `Mutex<bg>` or the atomic
// `running`; `server`/`local_host`/`port` are read-only after `new`. The ABI
// therefore takes only shared `&*srv` references (never `&mut`), so two threads
// in `serve`/`stop` never form aliasing `&mut`s. The one exception is
// `irc_server_free`, which takes ownership and must happen-after every other
// call on the handle has returned (the standard C ownership contract).
unsafe impl Send for CapiServer {}
unsafe impl Sync for CapiServer {}

/// Lock the background-thread slot, recovering a poisoned mutex.
///
/// The guarded value is just an `Option<JoinHandle>`; a panic while the guard was
/// held cannot leave it in a torn or half-updated state, so recovering the inner
/// value (rather than propagating the poison) is sound and keeps the ABI total.
fn lock_bg(m: &Mutex<Option<JoinHandle<()>>>) -> MutexGuard<'_, Option<JoinHandle<()>>> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Validate an untrusted C string as UTF-8.
///
/// Returns `None` for a NULL pointer *or* invalid UTF-8 — callers substitute a
/// safe default rather than ever forwarding unvalidated bytes into the engine.
unsafe fn cstr_to_string(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok().map(|s| s.to_owned())
}

/// Build a new IRC server bound to `host:port`.
///
/// `motd` is a single newline-joined string (the host language joins its lines);
/// it is split back into lines here, dropping empties. Any string argument that
/// is NULL or non-UTF-8 falls back to a safe default. `max_connections` is
/// clamped to at least 1.
///
/// Returns an opaque handle, or NULL if the socket could not be bound.
#[no_mangle]
pub unsafe extern "C" fn irc_server_new(
    host: *const c_char,
    port: u16,
    server_name: *const c_char,
    motd: *const c_char,
    oper_password: *const c_char,
    max_connections: c_uint,
) -> *mut CapiServer {
    catch_unwind(AssertUnwindSafe(|| {
        let host = cstr_to_string(host).unwrap_or_else(|| "127.0.0.1".to_string());
        let server_name = cstr_to_string(server_name).unwrap_or_else(|| "irc.local".to_string());
        let motd: Vec<String> = cstr_to_string(motd)
            .map(|joined| {
                joined
                    .split('\n')
                    .filter(|line| !line.is_empty())
                    .map(|line| line.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let oper_password = cstr_to_string(oper_password).unwrap_or_default();
        let max_connections = if max_connections >= 1 {
            max_connections as usize
        } else {
            1
        };

        let config = IrcConfig {
            host,
            port,
            server_name,
            motd,
            oper_password,
            max_connections,
        };
        match IrcReactorServer::bind(config) {
            Ok(server) => {
                let addr = server.local_addr();
                Box::into_raw(Box::new(CapiServer {
                    local_host: addr.ip().to_string(),
                    port: addr.port(),
                    server: Some(server),
                    running: Arc::new(AtomicBool::new(false)),
                    bg: Mutex::new(None),
                }))
            }
            Err(_) => ptr::null_mut(),
        }
    }))
    .unwrap_or(ptr::null_mut())
}

/// Run the event loop on the **calling** thread; blocks until `irc_server_stop`.
/// Returns 0 on a clean shutdown, -1 on a NULL handle or serve error.
#[no_mangle]
pub unsafe extern "C" fn irc_server_serve(srv: *mut CapiServer) -> c_int {
    if srv.is_null() {
        return -1;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // Shared ref only: a concurrent `irc_server_stop` from another thread
        // also takes `&*srv`, and all shared state is atomic/Mutex-guarded.
        let s = &*srv;
        if s.running.load(Ordering::SeqCst) {
            return 0;
        }
        let Some(engine) = s.server.clone() else {
            return -1;
        };
        s.running.store(true, Ordering::SeqCst);
        let outcome = engine.serve();
        s.running.store(false, Ordering::SeqCst);
        match outcome {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }));
    result.unwrap_or(-1)
}

/// Run the event loop on a background Rust thread; returns immediately.
/// Returns 0 on success, -1 on a NULL handle or if the engine is unavailable.
#[no_mangle]
pub unsafe extern "C" fn irc_server_serve_background(srv: *mut CapiServer) -> c_int {
    if srv.is_null() {
        return -1;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let s = &*srv;
        if s.running.load(Ordering::SeqCst) {
            return 0;
        }
        let Some(engine) = s.server.clone() else {
            return -1;
        };
        let running = Arc::clone(&s.running);
        // Set running *before* spawning so a stop() that races the spawn is not
        // lost (the engine's StopHandle is honoured by the reactor regardless).
        running.store(true, Ordering::SeqCst);
        let handle = std::thread::spawn(move || {
            // The spawned thread runs pure Rust and never re-enters the host VM.
            let _ = catch_unwind(AssertUnwindSafe(|| {
                let _ = engine.serve();
            }));
            running.store(false, Ordering::SeqCst);
        });
        *lock_bg(&s.bg) = Some(handle);
        0
    }));
    result.unwrap_or(-1)
}

/// Signal the loop to stop and join the background thread (if any).
#[no_mangle]
pub unsafe extern "C" fn irc_server_stop(srv: *mut CapiServer) {
    if srv.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let s = &*srv;
        if let Some(server) = s.server.as_ref() {
            server.stop();
        }
        if let Some(handle) = lock_bg(&s.bg).take() {
            let _ = handle.join();
        }
        s.running.store(false, Ordering::SeqCst);
    }));
}

/// Whether the event loop is currently running.
#[no_mangle]
pub unsafe extern "C" fn irc_server_running(srv: *mut CapiServer) -> bool {
    if srv.is_null() {
        return false;
    }
    catch_unwind(AssertUnwindSafe(|| (*srv).running.load(Ordering::SeqCst))).unwrap_or(false)
}

/// The bound IP address as a freshly-allocated C string.
///
/// **Ownership:** the caller owns the returned pointer and must release it with
/// `irc_server_string_free`. Returns NULL on a NULL handle or allocation failure.
#[no_mangle]
pub unsafe extern "C" fn irc_server_local_host(srv: *mut CapiServer) -> *mut c_char {
    if srv.is_null() {
        return ptr::null_mut();
    }
    catch_unwind(AssertUnwindSafe(|| {
        let host = &(*srv).local_host;
        // CString::new fails only if the host contains an interior NUL, which an
        // IP-address string never does; fall back to NULL defensively.
        CString::new(host.as_bytes())
            .map(|c| c.into_raw())
            .unwrap_or(ptr::null_mut())
    }))
    .unwrap_or(ptr::null_mut())
}

/// The bound TCP port (the OS-assigned port when constructed with `port == 0`).
#[no_mangle]
pub unsafe extern "C" fn irc_server_local_port(srv: *mut CapiServer) -> u16 {
    if srv.is_null() {
        return 0;
    }
    catch_unwind(AssertUnwindSafe(|| (*srv).port)).unwrap_or(0)
}

/// Free a string previously returned by this library (e.g. `irc_server_local_host`).
///
/// Passing NULL is a no-op. Passing any pointer not produced by this library, or
/// freeing the same pointer twice, is undefined behaviour.
#[no_mangle]
pub unsafe extern "C" fn irc_server_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(CString::from_raw(s));
    }));
}

/// Stop, join, and free a handle previously returned by `irc_server_new`.
///
/// Passing NULL is a no-op. The handle must be freed exactly once; a double free
/// is undefined behaviour.
#[no_mangle]
pub unsafe extern "C" fn irc_server_free(srv: *mut CapiServer) {
    if srv.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let boxed = Box::from_raw(srv);
        if let Some(server) = boxed.server.as_ref() {
            server.stop();
        }
        // Take the join handle in its own statement so the mutex guard is dropped
        // before `boxed` (releasing the borrow) at the end of scope.
        let bg = lock_bg(&boxed.bg).take();
        if let Some(handle) = bg {
            let _ = handle.join();
        }
        // `boxed` drops here, releasing the engine.
    }));
}

// ── Tests ────────────────────────────────────────────────────────────────────
//
// The `lib` crate-type lets us drive the C ABI directly from Rust. These mirror
// the broadcast scenario the Swift test exercises, but stay on the Rust side so
// `cargo test --lib` proves the ABI even where Swift is unavailable.
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    unsafe fn new_ephemeral() -> *mut CapiServer {
        let host = CString::new("127.0.0.1").unwrap();
        let name = CString::new("irc.test").unwrap();
        let motd = CString::new("Welcome.").unwrap();
        let pass = CString::new("").unwrap();
        irc_server_new(host.as_ptr(), 0, name.as_ptr(), motd.as_ptr(), pass.as_ptr(), 1024)
    }

    fn recv_until(stream: &mut TcpStream, needle: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        while Instant::now() < deadline {
            if String::from_utf8_lossy(&buf).contains(needle) {
                break;
            }
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(_) => continue, // timeout — poll again
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    }

    fn register(stream: &mut TcpStream, nick: &str) {
        write!(stream, "NICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n").unwrap();
        assert!(recv_until(stream, "001").contains("001"), "001 welcome for {nick}");
    }

    #[test]
    fn null_handle_is_safe() {
        unsafe {
            assert_eq!(irc_server_serve(ptr::null_mut()), -1);
            assert_eq!(irc_server_serve_background(ptr::null_mut()), -1);
            assert!(!irc_server_running(ptr::null_mut()));
            assert_eq!(irc_server_local_port(ptr::null_mut()), 0);
            assert!(irc_server_local_host(ptr::null_mut()).is_null());
            irc_server_stop(ptr::null_mut()); // no-op
            irc_server_free(ptr::null_mut()); // no-op
            irc_server_string_free(ptr::null_mut()); // no-op
        }
    }

    #[test]
    fn invalid_pointers_default_safely() {
        unsafe {
            // NULL strings → defaults; still binds.
            let srv = irc_server_new(
                ptr::null(),
                0,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                0, // clamped to >= 1
            );
            assert!(!srv.is_null());
            let host = irc_server_local_host(srv);
            assert!(!host.is_null());
            irc_server_string_free(host);
            irc_server_free(srv);
        }
    }

    #[test]
    fn broadcast_between_clients() {
        unsafe {
            let srv = new_ephemeral();
            assert!(!srv.is_null());
            assert!(!irc_server_running(srv));

            assert_eq!(irc_server_serve_background(srv), 0);
            // Wait for running to flip.
            let deadline = Instant::now() + Duration::from_secs(2);
            while !irc_server_running(srv) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(irc_server_running(srv));

            let host_ptr = irc_server_local_host(srv);
            let host = CStr::from_ptr(host_ptr).to_str().unwrap().to_owned();
            irc_server_string_free(host_ptr);
            let port = irc_server_local_port(srv);
            assert_eq!(host, "127.0.0.1");
            assert!(port > 0);

            let addr = format!("{host}:{port}");
            let mut alice = TcpStream::connect(&addr).unwrap();
            let mut bob = TcpStream::connect(&addr).unwrap();
            register(&mut alice, "alice");
            register(&mut bob, "bob");

            // PING/PONG liveness.
            write!(alice, "PING :liveness\r\n").unwrap();
            assert!(recv_until(&mut alice, "PONG").contains("PONG"));

            // Join and broadcast: alice's PRIVMSG must reach bob (mailbox fan-out).
            write!(alice, "JOIN #test\r\n").unwrap();
            write!(bob, "JOIN #test\r\n").unwrap();
            recv_until(&mut alice, "JOIN");
            recv_until(&mut bob, "JOIN");
            write!(alice, "PRIVMSG #test :hello bob\r\n").unwrap();
            let received = recv_until(&mut bob, "hello bob");
            assert!(received.contains("PRIVMSG"), "bob got a PRIVMSG");
            assert!(received.contains("hello bob"), "bob got alice's message");

            drop(alice);
            drop(bob);
            irc_server_stop(srv);
            assert!(!irc_server_running(srv));
            irc_server_free(srv);
        }
    }
}
