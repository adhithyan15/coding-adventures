//! # Conduit — Perl XS extension wrapping the Rust `web-core` engine
//!
//! Bridges Perl handler subs to the Conduit web framework (via the WEB08
//! `conduit` facade). Boots as `boot_CodingAdventures__Conduit`, which installs
//! XSUBs that Perl calls to build an application and serve it.
//!
//! ## Threading
//!
//! The embeddable HTTP engine runs its reactor **inline on the calling thread**:
//! `HttpServer::bind` uses a single `TcpRuntime` (not the multi-worker
//! `ShardedTcpRuntime`), and `TcpRuntime::serve()` runs the event loop on the
//! thread that called it. So when Perl calls `serve()`, handlers dispatch on the
//! Perl interpreter's **own** thread — exactly what a single-interpreter
//! (non-`MULTIPLICITY`) Perl needs, since such an interpreter is bound to its
//! init thread. No per-request lock or context rebinding is needed on this path;
//! the `Mutex<()>` in `Ctx` guards `dispatch` defensively but is uncontended,
//! and `set_context` is a no-op on a single-interpreter build.
//!
//! `serve_background` is the only path that spawns an OS thread, and is therefore
//! the only one that can corrupt a single-interpreter Perl. It is gated at the
//! Perl layer (croaks) and again here (warns + refuses when the captured context
//! is null, i.e. a non-`MULTIPLICITY` build).
//!
//! ## Peer-pointer model
//!
//! `new_app` / `new_server` return integer handles (`Box::into_raw(...) as IV`)
//! that Perl passes back on every call; `dispose_*` free them.
//!
//! ## Marshaling
//!
//! The request crosses as a flat Perl env hashref (string→string); nested maps
//! (route/query params, headers) are percent-encoded `k=v&…` strings. A handler
//! returns an arrayref `[status, body, headers_enc]` (or `undef` to continue,
//! for before filters). Status is clamped to 100–599; CR/LF and control chars
//! are dropped from headers.

#![allow(non_snake_case, non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use conduit::{Application as ConduitApp, Server as ConduitServer};
use embeddable_http_server::HttpServerOptions;
use perl_bridge::{
    call_coderef, get_context, hv_store, new_hv, new_rv_inc, newSViv, newSVpvn, newXS, set_context,
    sv_2iv, sv_to_string, warn, xs_boot_finish, xs_bootstrap, xsub_frame, xsub_return, CallResult,
    CV, IV, SV,
};
use web_core::{WebRequest, WebResponse};

// ── Send+Sync wrappers over raw Perl handles ────────────────────────────────

#[derive(Clone, Copy)]
struct SendSV(*mut SV);
unsafe impl Send for SendSV {}
unsafe impl Sync for SendSV {}
impl SendSV {
    #[inline(always)]
    fn get(&self) -> *mut SV {
        self.0
    }
}

#[derive(Clone, Copy)]
struct PerlCtx(*mut c_void);
unsafe impl Send for PerlCtx {}
unsafe impl Sync for PerlCtx {}

/// Shared interpreter lock + thread context, captured on the main Perl thread.
struct Ctx {
    lock: Mutex<()>,
    perl: PerlCtx,
}

// ── Native handles ──────────────────────────────────────────────────────────

struct NativeApp {
    routes: Vec<(String, String, SendSV)>,
    befores: Vec<SendSV>,
    afters: Vec<SendSV>,
    not_found: Option<SendSV>,
    error_handler: Option<SendSV>,
    settings: HashMap<String, String>,
    /// Every coderef we SvREFCNT_inc'd, for disposal.
    refs: Vec<SendSV>,
}

impl NativeApp {
    fn new() -> Self {
        NativeApp {
            routes: Vec::new(),
            befores: Vec::new(),
            afters: Vec::new(),
            not_found: None,
            error_handler: None,
            settings: HashMap::new(),
            refs: Vec::new(),
        }
    }
}

struct NativeServer {
    server: Option<ConduitServer>,
    stop: tcp_runtime::StopHandle,
    port: u16,
    running: Arc<AtomicBool>,
    bg: Option<std::thread::JoinHandle<()>>,
    #[allow(dead_code)]
    ctx: Arc<Ctx>,
    refs: Vec<SendSV>,
}
unsafe impl Send for NativeServer {}

// ── Percent-encoding (matches the Rust conduit-jni / Perl side) ─────────────

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
}

fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn encode_pairs<'a, I: Iterator<Item = (&'a str, &'a str)>>(pairs: I) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in pairs {
        parts.push(format!("{}={}", pct_encode(k), pct_encode(v)));
    }
    parts.join("&")
}

fn decode_pairs(s: &str) -> Vec<(String, String)> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split('&')
        .filter(|seg| !seg.is_empty())
        .map(|seg| match seg.split_once('=') {
            Some((k, v)) => (pct_decode(k), pct_decode(v)),
            None => (pct_decode(seg), String::new()),
        })
        .collect()
}

fn header_safe(name: &str, value: &str) -> bool {
    let bad_name = |b: u8| b < 0x20 || b == 0x7f || b == b':';
    let bad_value = |b: u8| b < 0x20 || b == 0x7f;
    !name.is_empty() && !name.bytes().any(bad_name) && !value.bytes().any(bad_value)
}

// ── Dispatch ────────────────────────────────────────────────────────────────

enum Outcome {
    None,
    Resp(WebResponse),
    Err(String),
}

/// Build the env hashref for `req`. Must run under the interpreter lock with
/// the context bound.
unsafe fn build_env(req: &WebRequest, error_msg: Option<&str>) -> *mut SV {
    let hv = new_hv();
    // NOTE: use newSVpvn (explicit length), NOT newSVpv — the latter treats a
    // length of 0 as "call strlen()", which reads past the non-NUL-terminated
    // pointer of an empty Rust &str and segfaults (e.g. an empty QUERY_STRING).
    let put = |k: &str, v: &str| {
        hv_store(hv, k, newSVpvn(v.as_ptr() as *const c_char, v.len()));
    };

    put("REQUEST_METHOD", req.method());
    put("PATH_INFO", req.path());

    let target = req.http.head.target.as_str();
    let qs = target.find('?').map(|i| &target[i + 1..]).unwrap_or("");
    put("QUERY_STRING", qs);

    let peer = req.peer_addr();
    let addr = peer.ip().to_string();
    put("REMOTE_ADDR", &addr);

    if let Some(ct) = req.content_type() {
        put("conduit.content_type", ct);
    }
    if let Some(cl) = req.content_length() {
        put("conduit.content_length", &cl.to_string());
    }
    let body = String::from_utf8_lossy(req.body()).into_owned();
    put("conduit.body", &body);

    let route_enc = encode_pairs(req.route_params.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    put("conduit.route_params", &route_enc);
    let query_enc = encode_pairs(req.query_params.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    put("conduit.query_params", &query_enc);

    // headers (lowercase, first wins)
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut hp: Vec<(String, String)> = Vec::new();
    for h in &req.http.head.headers {
        let name = h.name.to_lowercase();
        if seen.insert(name.clone()) {
            hp.push((name, h.value.clone()));
        }
    }
    let headers_enc = encode_pairs(hp.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    put("conduit.headers", &headers_enc);

    if let Some(msg) = error_msg {
        put("conduit.error", msg);
    }

    new_rv_inc(hv as *mut SV)
}

/// Parse the arrayref `[status, body, headers_enc]` returned by a handler.
unsafe fn parse_response(arrayref: *mut SV) -> WebResponse {
    use perl_bridge::{av_fetch, SvRV, SvROK};
    if SvROK(arrayref) == 0 {
        return WebResponse::internal_error("handler returned a non-reference");
    }
    let av = SvRV(arrayref) as *mut perl_bridge::AV;

    let fetch = |i: isize| -> *mut SV {
        let slot = av_fetch(av, i, 0);
        if slot.is_null() {
            std::ptr::null_mut()
        } else {
            *slot
        }
    };

    let status_sv = fetch(0);
    let raw_status = if status_sv.is_null() { 500 } else { sv_2iv(status_sv) };
    let status: u16 = if (100..=599).contains(&raw_status) {
        raw_status as u16
    } else {
        500
    };

    let body_sv = fetch(1);
    let body = if body_sv.is_null() {
        String::new()
    } else {
        sv_to_string(body_sv).unwrap_or_default()
    };

    let headers_sv = fetch(2);
    let headers_enc = if headers_sv.is_null() {
        String::new()
    } else {
        sv_to_string(headers_sv).unwrap_or_default()
    };

    let mut resp = WebResponse::new(status, body.into_bytes());
    for (k, v) in decode_pairs(&headers_enc) {
        if header_safe(&k, &v) {
            resp = resp.with_header(k, v);
        }
    }
    resp
}

/// Call a Perl handler coderef for `req`. Serialized + context-bound.
fn dispatch(ctx: &Ctx, coderef: *mut SV, req: &WebRequest, error_msg: Option<&str>) -> Outcome {
    let _guard = ctx.lock.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        set_context(ctx.perl.0);
        let env = build_env(req, error_msg);
        let result = call_coderef(coderef, &[env]);
        // env's hashref was a fresh RV we created; let Perl GC it (it's mortal-
        // free but refcount 1). Decrement to avoid a leak.
        perl_bridge::SvREFCNT_dec(env);
        match result {
            CallResult::Died(msg) => Outcome::Err(msg),
            CallResult::Empty => Outcome::None,
            CallResult::Ok(sv) => {
                let outcome = if perl_bridge::SvROK(sv) != 0 {
                    Outcome::Resp(parse_response(sv))
                } else {
                    Outcome::None
                };
                perl_bridge::SvREFCNT_dec(sv);
                outcome
            }
        }
    }
}

// ── XSUB argument helpers ───────────────────────────────────────────────────

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

/// Store a coderef arg: SvREFCNT_inc to keep it alive, record for disposal.
unsafe fn pin_coderef(app: &mut NativeApp, sv: *mut SV) -> SendSV {
    let kept = perl_bridge::SvREFCNT_inc(sv);
    let s = SendSV(kept);
    app.refs.push(s);
    s
}

// ── XSUBs ───────────────────────────────────────────────────────────────────

extern "C" fn xs_new_app(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let app = Box::new(NativeApp::new());
        let iv = Box::into_raw(app) as IV;
        set_return(f.base, f.ax, 0, newSViv(iv));
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_app_add_route(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 4 {
            perl_bridge::die("app_add_route(app, method, pattern, coderef)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let method = arg_string(f.base, f.ax, 1);
        let pattern = arg_string(f.base, f.ax, 2);
        let cr = pin_coderef(app, arg_sv(f.base, f.ax, 3));
        app.routes.push((method, pattern, cr));
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_add_before(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 2 {
            perl_bridge::die("app_add_before(app, coderef)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let cr = pin_coderef(app, arg_sv(f.base, f.ax, 1));
        app.befores.push(cr);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_add_after(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 2 {
            perl_bridge::die("app_add_after(app, coderef)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let cr = pin_coderef(app, arg_sv(f.base, f.ax, 1));
        app.afters.push(cr);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_set_not_found(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 2 {
            perl_bridge::die("app_set_not_found(app, coderef)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let cr = pin_coderef(app, arg_sv(f.base, f.ax, 1));
        app.not_found = Some(cr);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_set_error_handler(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 2 {
            perl_bridge::die("app_set_error_handler(app, coderef)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let cr = pin_coderef(app, arg_sv(f.base, f.ax, 1));
        app.error_handler = Some(cr);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_set_setting(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 3 {
            perl_bridge::die("app_set_setting(app, key, value)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let key = arg_string(f.base, f.ax, 1);
        let value = arg_string(f.base, f.ax, 2);
        app.settings.insert(key, value);
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_app_get_setting(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 2 {
            perl_bridge::die("app_get_setting(app, key)");
        }
        let app = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeApp);
        let key = arg_string(f.base, f.ax, 1);
        match app.settings.get(&key) {
            Some(v) => {
                set_return(f.base, f.ax, 0, newSVpvn(v.as_ptr() as *const c_char, v.len()));
                xsub_return(1, f.ax);
            }
            None => {
                // Return an empty list → undef in Perl scalar context.
                xsub_return(0, f.ax);
            }
        }
    });
}

extern "C" fn xs_new_server(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        if f.items < 4 {
            perl_bridge::die("new_server(app, host, port, max_conn)");
        }
        let app_ptr = arg_iv(f.base, f.ax, 0) as *mut NativeApp;
        let host = arg_string(f.base, f.ax, 1);
        let port_iv = arg_iv(f.base, f.ax, 2);
        let port = if (0..=65535).contains(&port_iv) { port_iv as u16 } else { 0 };
        let max_iv = arg_iv(f.base, f.ax, 3);
        let max_conn = if max_iv > 0 { max_iv as usize } else { 128 };

        let app_box = Box::from_raw(app_ptr);
        let NativeApp {
            routes,
            befores,
            afters,
            not_found,
            error_handler,
            settings,
            refs,
        } = *app_box;

        // Capture the interpreter context + lock NOW, on the main Perl thread.
        let ctx = Arc::new(Ctx {
            lock: Mutex::new(()),
            perl: PerlCtx(get_context()),
        });

        let mut web_app = ConduitApp::new();
        for (k, v) in &settings {
            web_app.set(k.clone(), v.clone());
        }

        let eh = error_handler;
        for (method, pattern, cr) in &routes {
            let c = Arc::clone(&ctx);
            let handler = *cr;
            let err = eh;
            web_app.route(method.clone(), pattern, move |req| {
                match dispatch(&c, handler.get(), req, None) {
                    Outcome::Resp(r) => r,
                    Outcome::None => WebResponse::internal_error("handler returned undef"),
                    Outcome::Err(msg) => {
                        if let Some(e) = err {
                            match dispatch(&c, e.get(), req, Some(&msg)) {
                                Outcome::Resp(r) => r,
                                _ => WebResponse::internal_error(&msg),
                            }
                        } else {
                            WebResponse::internal_error(&msg)
                        }
                    }
                }
            });
        }
        for cr in &befores {
            let c = Arc::clone(&ctx);
            let handler = *cr;
            web_app.before(move |req| match dispatch(&c, handler.get(), req, None) {
                Outcome::Resp(r) => Some(r),
                Outcome::None => None,
                Outcome::Err(msg) => Some(WebResponse::internal_error(&msg)),
            });
        }
        for cr in &afters {
            let c = Arc::clone(&ctx);
            let handler = *cr;
            web_app.after_response(move |req, prev| match dispatch(&c, handler.get(), req, None) {
                Outcome::Resp(r) => r,
                _ => prev,
            });
        }
        if let Some(nf) = not_found {
            let c = Arc::clone(&ctx);
            web_app.not_found(move |req| match dispatch(&c, nf.get(), req, None) {
                Outcome::Resp(r) => r,
                _ => WebResponse::not_found(),
            });
        }
        if let Some(e) = error_handler {
            let c = Arc::clone(&ctx);
            web_app.on_error(move |req, msg| match dispatch(&c, e.get(), req, Some(msg)) {
                Outcome::Resp(r) => r,
                _ => WebResponse::internal_error(msg),
            });
        }

        let mut opts = HttpServerOptions::default();
        opts.tcp.max_connections = max_conn;
        let server = match ConduitServer::bind_with_options(&host, port, opts, web_app) {
            Ok(s) => s,
            Err(e) => {
                perl_bridge::die(&format!("conduit: bind failed: {e}"));
            }
        };
        let bound = server.local_addr().port();
        let stop = server.stop_handle();
        let native = NativeServer {
            server: Some(server),
            stop,
            port: bound,
            running: Arc::new(AtomicBool::new(false)),
            bg: None,
            ctx,
            refs,
        };
        set_return(f.base, f.ax, 0, newSViv(Box::into_raw(Box::new(native)) as IV));
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_server_serve(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        if let Some(mut s) = srv.server.take() {
            srv.running.store(true, Ordering::SeqCst);
            let _ = s.serve();
            srv.running.store(false, Ordering::SeqCst);
        }
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_server_serve_background(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        // Defense-in-depth: serve_background spawns an OS thread that dispatches
        // into the Perl interpreter. On a single-interpreter (non-MULTIPLICITY)
        // build the interpreter is bound to its init thread, so calling it from a
        // spawned thread corrupts/crashes it. On such a build the captured
        // context is NULL — refuse to spawn rather than crash. (warn, not croak:
        // a Perl croak longjmps, which is UB across this catch_unwind frame. The
        // Perl Server::serve_background wrapper already croaks before reaching
        // here; this guards a direct CodingAdventures::Conduit::Native:: call.)
        if srv.ctx.perl.0.is_null() {
            warn(
                "Conduit: serve_background requires a MULTIPLICITY/ithreads Perl; \
                 refusing to spawn on a single-interpreter build — use serve() in the foreground",
            );
            xsub_return(0, f.ax);
            return;
        }
        if let Some(mut s) = srv.server.take() {
            let running = Arc::clone(&srv.running);
            running.store(true, Ordering::SeqCst);
            let h = std::thread::spawn(move || {
                let _ = s.serve();
                running.store(false, Ordering::SeqCst);
            });
            srv.bg = Some(h);
        }
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_server_stop(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &mut *(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        srv.stop.stop();
        if let Some(h) = srv.bg.take() {
            let _ = h.join();
        }
        xsub_return(0, f.ax);
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

extern "C" fn xs_server_running(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let srv = &*(arg_iv(f.base, f.ax, 0) as *mut NativeServer);
        let r = if srv.running.load(Ordering::SeqCst) { 1 } else { 0 };
        set_return(f.base, f.ax, 0, newSViv(r));
        xsub_return(1, f.ax);
    });
}

extern "C" fn xs_dispose_server(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let iv = arg_iv(f.base, f.ax, 0);
        if iv != 0 {
            let mut srv = Box::from_raw(iv as *mut NativeServer);
            srv.stop.stop();
            if let Some(h) = srv.bg.take() {
                let _ = h.join();
            }
            for r in &srv.refs {
                perl_bridge::SvREFCNT_dec(r.get());
            }
        }
        xsub_return(0, f.ax);
    });
}

extern "C" fn xs_dispose_app(_cv: *mut CV) {
    let _ = catch_unwind(|| unsafe {
        let f = xsub_frame();
        let iv = arg_iv(f.base, f.ax, 0);
        if iv != 0 {
            let app = Box::from_raw(iv as *mut NativeApp);
            for r in &app.refs {
                perl_bridge::SvREFCNT_dec(r.get());
            }
        }
        xsub_return(0, f.ax);
    });
}

// ── Boot ────────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__Conduit(cv: *mut CV) {
    let file = b"Conduit.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);

    let reg = |name: &[u8], sub: extern "C" fn(*mut CV)| {
        newXS(name.as_ptr() as *const c_char, sub, file);
    };
    reg(b"CodingAdventures::Conduit::Native::new_app\0", xs_new_app);
    reg(b"CodingAdventures::Conduit::Native::app_add_route\0", xs_app_add_route);
    reg(b"CodingAdventures::Conduit::Native::app_add_before\0", xs_app_add_before);
    reg(b"CodingAdventures::Conduit::Native::app_add_after\0", xs_app_add_after);
    reg(b"CodingAdventures::Conduit::Native::app_set_not_found\0", xs_app_set_not_found);
    reg(b"CodingAdventures::Conduit::Native::app_set_error_handler\0", xs_app_set_error_handler);
    reg(b"CodingAdventures::Conduit::Native::app_set_setting\0", xs_app_set_setting);
    reg(b"CodingAdventures::Conduit::Native::app_get_setting\0", xs_app_get_setting);
    reg(b"CodingAdventures::Conduit::Native::new_server\0", xs_new_server);
    reg(b"CodingAdventures::Conduit::Native::server_serve\0", xs_server_serve);
    reg(b"CodingAdventures::Conduit::Native::server_serve_background\0", xs_server_serve_background);
    reg(b"CodingAdventures::Conduit::Native::server_stop\0", xs_server_stop);
    reg(b"CodingAdventures::Conduit::Native::server_local_port\0", xs_server_local_port);
    reg(b"CodingAdventures::Conduit::Native::server_running\0", xs_server_running);
    reg(b"CodingAdventures::Conduit::Native::dispose_server\0", xs_dispose_server);
    reg(b"CodingAdventures::Conduit::Native::dispose_app\0", xs_dispose_app);

    xs_boot_finish(ax);
}
