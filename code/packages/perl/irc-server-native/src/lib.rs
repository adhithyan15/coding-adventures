//! `IrcServerNative` — a Perl XS extension embedding the all-Rust IRC engine
//! (`irc-net-reactor`) into Perl.
//!
//! ## Design: control surface only, no callbacks
//!
//! All IRC and TCP logic lives in Rust (`irc-net-reactor` on the home-grown
//! kqueue/epoll reactor).  Perl only *launches and controls* the server, so this
//! extension registers a handful of XSUBs over an opaque peer pointer (a Perl
//! `IV`): `new_server`, `server_serve` (foreground, blocks), `server_serve_background`,
//! `server_stop`, `server_running`, `server_local_host`, `server_local_port`,
//! `dispose_server`.
//!
//! Unlike `conduit`'s Perl XS, there is **no per-request dispatch back into
//! Perl** (no routes, no coderefs).  Crucially, that means `serve_background`'s
//! spawned thread runs **pure Rust** (`engine.serve()`) and never touches the
//! Perl interpreter — so it is safe even on a single-interpreter (non-ithreads)
//! Perl, where conduit had to refuse to spawn.
//!
//! ## Safety notes
//!
//! * Every XSUB body is wrapped in `catch_unwind` — a Rust panic must never
//!   unwind into the Perl interpreter (UB).  Argument validation uses Perl's
//!   `die` (a longjmp) only at the very start, before any Rust resource is held.
//! * All Rust → Perl strings use `newSVpvn` (explicit length); `newSVpv` would
//!   `strlen` the pointer and read out of bounds on an empty string (segfault).
//! * `serve`/`serve_background` run the blocking loop on an **owned clone** of
//!   the engine (`Clone` over `Arc`s), so the peer is never dereferenced by the
//!   background thread and can't dangle.

// The XS boot table uses nul-terminated byte-string literals for C `char*`,
// matching conduit's Perl XS; clippy's c-string-literal lint is noise here.
#![allow(
    non_snake_case,
    clippy::missing_safety_doc,
    clippy::manual_c_str_literals
)]

use std::ffi::c_char;
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use irc_net_reactor::{IrcConfig, IrcReactorServer};
use perl_bridge::{
    die, newSViv, newSVpvn, newXS, sv_2iv, sv_to_string, xs_boot_finish, xs_bootstrap, xsub_frame,
    xsub_return, CV, IV, SV,
};

/// The Rust state behind the opaque peer `IV`.
struct NativeServer {
    /// The bound engine, kept as a control handle; the serve loop runs a clone.
    server: Option<IrcReactorServer>,
    local_host: String,
    port: u16,
    running: Arc<AtomicBool>,
    bg: Option<std::thread::JoinHandle<()>>,
}

// SAFETY: only an owned clone of `IrcReactorServer` (Arc-based, Send) crosses
// into the background thread; the peer struct stays on the Perl thread.
unsafe impl Send for NativeServer {}

// ── XSUB argument helpers ────────────────────────────────────────────────────

unsafe fn arg_sv(base: *mut *mut SV, ax: i32, n: i32) -> *mut SV {
    *base.add((ax + n) as usize)
}
unsafe fn arg_iv(base: *mut *mut SV, ax: i32, n: i32) -> IV {
    let sv = arg_sv(base, ax, n);
    if sv.is_null() {
        0
    } else {
        sv_2iv(sv)
    }
}
unsafe fn arg_string(base: *mut *mut SV, ax: i32, n: i32) -> String {
    let sv = arg_sv(base, ax, n);
    if sv.is_null() {
        String::new()
    } else {
        sv_to_string(sv).unwrap_or_default()
    }
}
unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

// ── XSUBs ────────────────────────────────────────────────────────────────────

extern "C" fn xs_new_server(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 6 {
            die("new_server(host, port, server_name, motd, oper_password, max_connections)");
        }
        let host = arg_string(f.base, f.ax, 0);
        let port_iv = arg_iv(f.base, f.ax, 1);
        if !(0..=65535).contains(&port_iv) {
            die("port must be between 0 and 65535");
        }
        let port = port_iv as u16;
        let server_name = arg_string(f.base, f.ax, 2);
        // MOTD arrives as a single newline-joined string; split into lines.
        let motd: Vec<String> = arg_string(f.base, f.ax, 3)
            .split('\n')
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();
        let oper_password = arg_string(f.base, f.ax, 4);
        let max_iv = arg_iv(f.base, f.ax, 5);
        if max_iv < 1 {
            die("max_connections must be >= 1");
        }
        let max_connections = max_iv as usize;

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
            Err(e) => die(&format!("irc_server_native: bind failed: {e}")),
        };
        let addr = server.local_addr();
        let native = NativeServer {
            local_host: addr.ip().to_string(),
            port: addr.port(),
            server: Some(server),
            running: Arc::new(AtomicBool::new(false)),
            bg: None,
        };
        set_return(
            f.base,
            f.ax,
            0,
            newSViv(Box::into_raw(Box::new(native)) as IV),
        );
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_server_serve(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        if !srv.running.load(Ordering::SeqCst) {
            if let Some(engine) = srv.server.clone() {
                srv.running.store(true, Ordering::SeqCst);
                let _ = engine.serve();
                srv.running.store(false, Ordering::SeqCst);
            }
        }
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_server_serve_background(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        // The spawned thread runs pure Rust (engine.serve()) and never enters the
        // Perl interpreter, so this is safe even on single-interpreter Perl.
        if !srv.running.load(Ordering::SeqCst) {
            if let Some(engine) = srv.server.clone() {
                let running = Arc::clone(&srv.running);
                running.store(true, Ordering::SeqCst);
                let h = std::thread::spawn(move || {
                    let _ = engine.serve();
                    running.store(false, Ordering::SeqCst);
                });
                srv.bg = Some(h);
            }
        }
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_server_stop(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        if let Some(server) = srv.server.as_ref() {
            server.stop();
        }
        if let Some(h) = srv.bg.take() {
            let _ = h.join();
        }
        srv.running.store(false, Ordering::SeqCst);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_server_running(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &*(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        let r = if srv.running.load(Ordering::SeqCst) {
            1
        } else {
            0
        };
        set_return(f.base, f.ax, 0, newSViv(r));
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_server_local_host(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &*(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        // newSVpvn (explicit length) — newSVpv would strlen and could read OOB.
        set_return(
            f.base,
            f.ax,
            0,
            newSVpvn(
                srv.local_host.as_ptr() as *const c_char,
                srv.local_host.len(),
            ),
        );
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_server_local_port(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &*(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        set_return(f.base, f.ax, 0, newSViv(srv.port as IV));
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_dispose_server(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let iv = arg_iv(f.base, f.ax, 0);
        if iv != 0 {
            let mut srv = Box::from_raw(iv as *mut NativeServer);
            if let Some(server) = srv.server.as_ref() {
                server.stop();
            }
            if let Some(h) = srv.bg.take() {
                let _ = h.join();
            }
            // srv dropped here.
        }
        xsub_return(0, f.ax);
    });
}

// ── Boot ─────────────────────────────────────────────────────────────────────

/// DynaLoader calls this when Perl runs `CodingAdventures::IrcServerNative->bootstrap`.
///
/// # Safety
/// Invoked once by the Perl loader on the main interpreter thread.
#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__IrcServerNative(cv: *mut CV) {
    let file = b"IrcServerNative.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);

    let reg = |name: &[u8], sub: extern "C" fn(*mut CV)| {
        newXS(name.as_ptr() as *const c_char, sub, file);
    };
    reg(
        b"CodingAdventures::IrcServerNative::Native::new_server\0",
        xs_new_server,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_serve\0",
        xs_server_serve,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_serve_background\0",
        xs_server_serve_background,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_stop\0",
        xs_server_stop,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_running\0",
        xs_server_running,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_local_host\0",
        xs_server_local_host,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::server_local_port\0",
        xs_server_local_port,
    );
    reg(
        b"CodingAdventures::IrcServerNative::Native::dispose_server\0",
        xs_dispose_server,
    );

    xs_boot_finish(ax);
}
