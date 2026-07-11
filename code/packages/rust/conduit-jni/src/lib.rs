// lib.rs — conduit_jni
//
// JNI native library bridging Java to the Conduit web framework. Java handler
// lambdas run on the JVM; routing, lifecycle hooks, and HTTP I/O run in Rust
// (the WEB08 `conduit` facade over `web-core`). Loaded by the JVM via
// `System.loadLibrary("conduit_jni")`.
//
// # Threading model — the JNI cross-thread callback dance
//
// The JVM is multi-threaded and `web-core` dispatches HTTP requests on its own
// background Rust I/O threads — threads the JVM has never seen. Two JNI rules
// drive the whole design:
//
//   1. A `JNIEnv*` is thread-local. A Rust I/O thread cannot reuse the env
//      from the registration call; it must attach itself to the JVM with
//      `AttachCurrentThreadAsDaemon(vm)` to obtain its own env.
//   2. Local references die when the native call returns. Java handler
//      objects must be promoted to *global* references (`NewGlobalRef`) to
//      stay callable for the server's lifetime.
//
// So per-request dispatch on a web-core I/O thread looks like:
//
//   ┌──────────────────────────────────────────────────────────────────┐
//   │  AttachCurrentThreadAsDaemon(vm) → JNIEnv (idempotent, no detach)  │
//   │  PushLocalFrame(env, 16)         → bound this request's local refs │
//   │  build Request jobject (NewObjectA, 9 strings)                     │
//   │  CallObjectMethodA(handlerGlobalRef, handle, [request]) → Response │
//   │  ExceptionCheck → HaltException? other error? (see Outcome)        │
//   │  copy status/body/headers into an owned Rust value                 │
//   │  PopLocalFrame(env)              → free all the local refs         │
//   │  return the owned WebResponse to web-core                          │
//   └──────────────────────────────────────────────────────────────────┘
//
// `AttachCurrentThreadAsDaemon` is used (not plain attach) so the long-lived
// I/O threads never need an explicit detach and don't block JVM shutdown.
// Dispatch is *concurrent* — each I/O thread attaches independently and calls
// Java handlers in parallel. Global refs, classes (held as global refs), and
// method IDs are all valid across threads, which makes this safe. This is
// unlike the Lua port, which serializes everything through one `lua_State`
// lock.
//
// # Peer-pointer model
//
// Java holds a `long` pointing at a heap `Box<NativeApp>` / `Box<NativeServer>`
// (the classic JNI peer-object pattern). `nativeNewApp` returns the pointer;
// subsequent calls pass it back; `nativeNewServer` consumes the app and
// returns a server pointer; `nativeDisposeServer` / `nativeDisposeApp`
// `DeleteGlobalRef` every handler and free the box.
//
// # Marshaling
//
// Maps cross the boundary as percent-encoded `k=v&k2=v2` strings (route
// params, headers). The Java side decodes with `URLDecoder`; the Rust side
// decodes with `pct_decode`. This needs only `NewStringUTF` — no HashMap-
// building JNI chatter and no object-array machinery.

#![allow(non_snake_case)]
// Every exported fn is a `Java_*` JNI entry point invoked only by the JVM,
// which guarantees the pointer/handle contract; the safety obligations are
// uniform and documented in the module header above.
#![allow(clippy::missing_safety_doc)]
// The `guard_ptr!(ptr, ())` macro takes an explicit "value to return on a null
// pointer"; `()` is a required macro argument here, not a redundant unit.
#![allow(clippy::unused_unit)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use conduit::{Application as ConduitApp, Server as ConduitServer};
use jni_bridge::*;
use web_core::{WebRequest, WebResponse};

use embeddable_http_server::HttpServerOptions;

// ─────────────────────────────────────────────────────────────────────────────
// Send+Sync newtypes over raw JNI handles
// ─────────────────────────────────────────────────────────────────────────────
//
// Global refs, jclass (held as global refs), jmethodID, and the JavaVM are all
// documented as usable from any thread. Rust's auto-traits don't know that, so
// we wrap each raw pointer and assert Send + Sync. The closures web-core stores
// must be Send + Sync; capturing these wrappers keeps that property.

#[derive(Clone, Copy)]
struct Obj(jobject);
unsafe impl Send for Obj {}
unsafe impl Sync for Obj {}

impl Obj {
    /// Return the raw handle.
    ///
    /// Using a method (rather than touching `.0` directly inside a closure)
    /// matters for Rust 2021 disjoint closure capture: `move || x.get()`
    /// captures the whole `Obj` (which is `Send + Sync`), whereas
    /// `move || x.0` would capture only the inner `*mut c_void` (which is
    /// not), breaking the `Send + Sync` bound web-core requires.
    #[inline(always)]
    fn get(&self) -> jobject {
        self.0
    }
}

#[derive(Clone, Copy)]
struct Mid(jmethodID);
unsafe impl Send for Mid {}
unsafe impl Sync for Mid {}

#[derive(Clone, Copy)]
struct Vm(*mut JavaVM);
unsafe impl Send for Vm {}
unsafe impl Sync for Vm {}

// ─────────────────────────────────────────────────────────────────────────────
// DispatchCtx — classes + method IDs + JavaVM, resolved once at app creation
// ─────────────────────────────────────────────────────────────────────────────

struct DispatchCtx {
    vm: Vm,
    // Request
    request_class: Obj,
    request_ctor: Mid,
    // ConduitHandler.handle(Request) -> Response  (virtual dispatch on the iface)
    handler_mid: Mid,
    // Response getters
    resp_status: Mid,
    resp_body: Mid,
    resp_headers: Mid,
    // HaltException
    halt_class: Obj,
    halt_status: Mid,
    halt_body: Mid,
    halt_headers: Mid,
    // Throwable.getMessage()
    throwable_get_message: Mid,
}

const PKG: &str = "com/codingadventures/conduit";

/// Resolve every class + method ID we need and pin the classes as global refs.
/// Runs on a JVM thread (a native call), so `env` is valid here.
unsafe fn build_dispatch_ctx(env: *mut JNIEnv) -> Option<DispatchCtx> {
    let vm = jni_get_java_vm(env);
    if vm.is_null() {
        return None;
    }

    // Helper: FindClass then promote to a global ref so it survives across calls
    // and threads. Returns null on failure.
    let global_class = |name: &str| -> jclass {
        let local = jni_find_class(env, name);
        if local.is_null() {
            return std::ptr::null_mut();
        }
        jni_new_global_ref(env, local)
    };

    let request_class = global_class(&format!("{PKG}/Request"));
    let response_class = global_class(&format!("{PKG}/Response"));
    let handler_class = global_class(&format!("{PKG}/ConduitHandler"));
    let halt_class = global_class(&format!("{PKG}/HaltException"));
    let throwable_class = jni_find_class(env, "java/lang/Throwable");

    if request_class.is_null()
        || response_class.is_null()
        || handler_class.is_null()
        || halt_class.is_null()
        || throwable_class.is_null()
    {
        return None;
    }

    // Request(String x9) constructor.
    let nine_strings = "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;\
Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;\
Ljava/lang/String;Ljava/lang/String;)V";
    let request_ctor = jni_get_method_id(env, request_class, "<init>", nine_strings);

    let req_sig = format!("(L{PKG}/Request;)L{PKG}/Response;");
    let handler_mid = jni_get_method_id(env, handler_class, "handle", &req_sig);

    let resp_status = jni_get_method_id(env, response_class, "status", "()I");
    let resp_body = jni_get_method_id(env, response_class, "body", "()Ljava/lang/String;");
    let resp_headers =
        jni_get_method_id(env, response_class, "headersEncoded", "()Ljava/lang/String;");

    let halt_status = jni_get_method_id(env, halt_class, "status", "()I");
    let halt_body = jni_get_method_id(env, halt_class, "body", "()Ljava/lang/String;");
    let halt_headers =
        jni_get_method_id(env, halt_class, "headersEncoded", "()Ljava/lang/String;");

    let throwable_get_message =
        jni_get_method_id(env, throwable_class, "getMessage", "()Ljava/lang/String;");

    if request_ctor.is_null()
        || handler_mid.is_null()
        || resp_status.is_null()
        || resp_body.is_null()
        || resp_headers.is_null()
        || halt_status.is_null()
        || halt_body.is_null()
        || halt_headers.is_null()
        || throwable_get_message.is_null()
    {
        return None;
    }

    Some(DispatchCtx {
        vm: Vm(vm),
        request_class: Obj(request_class),
        request_ctor: Mid(request_ctor),
        handler_mid: Mid(handler_mid),
        resp_status: Mid(resp_status),
        resp_body: Mid(resp_body),
        resp_headers: Mid(resp_headers),
        halt_class: Obj(halt_class),
        halt_status: Mid(halt_status),
        halt_body: Mid(halt_body),
        halt_headers: Mid(halt_headers),
        throwable_get_message: Mid(throwable_get_message),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Native handles
// ─────────────────────────────────────────────────────────────────────────────

struct NativeApp {
    app: ConduitApp,
    ctx: Arc<DispatchCtx>,
    /// Global refs to every handler/filter, freed on disposal.
    handler_refs: Vec<Obj>,
    /// Shared slot for the error handler, read at dispatch time (it may be
    /// registered after routes). `Obj(null)` means "no error handler".
    error_handler: Arc<std::sync::Mutex<Obj>>,
}

struct NativeServer {
    server: Option<ConduitServer>,
    stop: tcp_runtime::StopHandle,
    port: u16,
    running: Arc<AtomicBool>,
    bg: Option<std::thread::JoinHandle<()>>,
    #[allow(dead_code)]
    ctx: Arc<DispatchCtx>,
    handler_refs: Vec<Obj>,
}

// SAFETY: ConduitServer is moved into a background thread by serve_background;
// the facade's own tests do the same, confirming Send.
unsafe impl Send for NativeServer {}

// ─────────────────────────────────────────────────────────────────────────────
// Percent-encoding for the map wire format
// ─────────────────────────────────────────────────────────────────────────────

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
}

/// Percent-encode `s` so it can be embedded in a `k=v&…` string and decoded by
/// `java.net.URLDecoder`. Everything outside the URI unreserved set becomes
/// `%XX`; a literal `+` becomes `%2B` so URLDecoder's `+`→space rule can't
/// corrupt the value.
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

/// Decode a percent-encoded byte sequence (the inverse of `pct_encode`,
/// also accepts `+` as space for robustness against URLEncoder output).
fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
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

/// Encode an iterator of (key, value) pairs as `k=v&k2=v2` (both pct-encoded).
fn encode_pairs<'a, I: Iterator<Item = (&'a str, &'a str)>>(pairs: I) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in pairs {
        parts.push(format!("{}={}", pct_encode(k), pct_encode(v)));
    }
    parts.join("&")
}

/// Parse a `k=v&k2=v2` string into (key, value) pairs (both pct-decoded).
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

// ─────────────────────────────────────────────────────────────────────────────
// Dispatch — call a Java handler on the current (attached) thread
// ─────────────────────────────────────────────────────────────────────────────

enum Outcome {
    /// Handler returned `null` (a before-filter "continue" signal).
    None,
    /// Handler returned a Response (or threw a HaltException).
    Resp(WebResponse),
    /// Handler threw a non-halt exception; carries the message.
    Err(String),
    /// A JNI-level failure (attach/build failed) — should be rare.
    Fatal(String),
}

/// Build the Java `Request` object for `req`. Returns null on allocation error.
unsafe fn build_request(
    env: *mut JNIEnv,
    ctx: &DispatchCtx,
    req: &WebRequest,
    error_msg: Option<&str>,
) -> jobject {
    let method = req.method().to_string();
    let path = req.path().to_string();

    // QUERY_STRING from the raw target ("/p?a=1" → "a=1").
    let target = req.http.head.target.as_str();
    let query_string = target.find('?').map(|i| &target[i + 1..]).unwrap_or("");

    let body = String::from_utf8_lossy(req.body()).into_owned();
    let content_type = req.content_type().unwrap_or("").to_string();
    let peer = req.peer_addr();
    let remote_addr = peer.ip().to_string();

    let route_params_enc = encode_pairs(
        req.route_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str())),
    );

    // Headers: lowercase names, first value wins for duplicates.
    let mut header_pairs: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for h in &req.http.head.headers {
        let name = h.name.to_lowercase();
        if seen.insert(name.clone()) {
            header_pairs.push((name, h.value.clone()));
        }
    }
    let headers_enc = encode_pairs(
        header_pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str())),
    );

    let error = error_msg.unwrap_or("");

    // Build the 9 jstrings.
    let s = |v: &str| jni_new_string_utf(env, v);
    let args = [
        jvalue { l: s(&method) },
        jvalue { l: s(&path) },
        jvalue { l: s(query_string) },
        jvalue { l: s(&body) },
        jvalue { l: s(&content_type) },
        jvalue { l: s(&remote_addr) },
        jvalue { l: s(&route_params_enc) },
        jvalue { l: s(&headers_enc) },
        jvalue { l: s(error) },
    ];

    jni_new_object_a(env, ctx.request_class.0, ctx.request_ctor.0, args.as_ptr())
}

/// Read a Java Response/HaltException object into a `WebResponse`, using the
/// given status/body/headers method IDs.
unsafe fn read_response(
    env: *mut JNIEnv,
    obj: jobject,
    status_mid: jmethodID,
    body_mid: jmethodID,
    headers_mid: jmethodID,
) -> WebResponse {
    let raw_status = jni_call_int_method_a(env, obj, status_mid, std::ptr::null());
    // Clamp to a valid HTTP status; anything out of range collapses to 500.
    let status: u16 = if (100..=599).contains(&raw_status) {
        raw_status as u16
    } else {
        500
    };

    let body_obj = jni_call_object_method_a(env, obj, body_mid, std::ptr::null());
    let body = jni_get_string_utf(env, body_obj).unwrap_or_default();

    let headers_obj = jni_call_object_method_a(env, obj, headers_mid, std::ptr::null());
    let headers_enc = jni_get_string_utf(env, headers_obj).unwrap_or_default();

    let mut resp = WebResponse::new(status, body.into_bytes());
    for (k, v) in decode_pairs(&headers_enc) {
        // Defense-in-depth against response splitting: drop any header whose
        // name or value carries CR/LF (the Java side strips these too).
        if header_safe(&k, &v) {
            resp = resp.with_header(k, v);
        }
    }
    resp
}

fn header_safe(name: &str, value: &str) -> bool {
    // Reject ALL C0 control bytes (< 0x20) and DEL (0x7F), not only CR/LF/NUL:
    // any control char in a header line is a response-splitting / smuggling
    // risk depending on the downstream serializer. Names additionally forbid
    // ':' (the name/value delimiter). A ':' in a *value* is fine (URLs, times).
    let bad_name = |b: u8| b < 0x20 || b == 0x7f || b == b':';
    let bad_value = |b: u8| b < 0x20 || b == 0x7f;
    !name.is_empty() && !name.bytes().any(bad_name) && !value.bytes().any(bad_value)
}

/// Invoke a Java handler global ref with `req`, returning an owned `Outcome`.
/// All JNI local refs are freed before returning (PushLocalFrame/PopLocalFrame).
fn dispatch(ctx: &DispatchCtx, handler: jobject, req: &WebRequest, error_msg: Option<&str>) -> Outcome {
    unsafe {
        let env = jni_attach_current_thread_as_daemon(ctx.vm.0);
        if env.is_null() {
            return Outcome::Fatal("AttachCurrentThreadAsDaemon failed".to_string());
        }

        if jni_push_local_frame(env, 16) != 0 {
            jni_exception_clear(env);
            return Outcome::Fatal("PushLocalFrame failed".to_string());
        }

        let outcome = (|| {
            let req_obj = build_request(env, ctx, req, error_msg);
            if req_obj.is_null() {
                jni_exception_clear(env);
                return Outcome::Fatal("failed to build Request".to_string());
            }

            let args = [jvalue { l: req_obj }];
            let result = jni_call_object_method_a(env, handler, ctx.handler_mid.0, args.as_ptr());

            if jni_exception_check(env) {
                let thr = jni_exception_occurred(env);
                jni_exception_clear(env);
                if !thr.is_null() && jni_is_instance_of(env, thr, ctx.halt_class.0) {
                    let resp = read_response(
                        env,
                        thr,
                        ctx.halt_status.0,
                        ctx.halt_body.0,
                        ctx.halt_headers.0,
                    );
                    return Outcome::Resp(resp);
                }
                // Any other Throwable → route to the error handler.
                let msg = if thr.is_null() {
                    "handler threw an exception".to_string()
                } else {
                    let m = jni_call_object_method_a(
                        env,
                        thr,
                        ctx.throwable_get_message.0,
                        std::ptr::null(),
                    );
                    jni_get_string_utf(env, m).unwrap_or_else(|| "handler error".to_string())
                };
                return Outcome::Err(msg);
            }

            if result.is_null() {
                return Outcome::None;
            }

            let resp = read_response(
                env,
                result,
                ctx.resp_status.0,
                ctx.resp_body.0,
                ctx.resp_headers.0,
            );
            Outcome::Resp(resp)
        })();

        jni_pop_local_frame(env);
        outcome
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JNI native methods
// ─────────────────────────────────────────────────────────────────────────────
//
// Symbol naming: Java_<package_with_underscores>_<Class>_<method>.
// The Java side declares these on `com.codingadventures.conduit.Native`.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeNewApp(
    env: *mut JNIEnv,
    _class: jclass,
) -> jlong {
    let ctx = match build_dispatch_ctx(env) {
        Some(c) => Arc::new(c),
        None => {
            jni_throw_new(
                env,
                "java/lang/RuntimeException",
                "conduit_jni: failed to resolve Java classes/methods (is the conduit package on the classpath?)",
            );
            return 0;
        }
    };
    let app = NativeApp {
        app: ConduitApp::new(),
        ctx,
        handler_refs: Vec::new(),
        error_handler: Arc::new(std::sync::Mutex::new(Obj(std::ptr::null_mut()))),
    };
    Box::into_raw(Box::new(app)) as jlong
}

/// Pin a handler as a global ref and record it on the app for disposal.
unsafe fn pin_handler(env: *mut JNIEnv, app: &mut NativeApp, handler: jobject) -> Obj {
    let g = jni_new_global_ref(env, handler);
    app.handler_refs.push(Obj(g));
    Obj(g)
}

/// Early-return from a native method when the peer pointer is 0.
///
/// The Java wrappers already guard every call with `checkOpen()` (handle != 0),
/// so a zero pointer only reaches here through a lifecycle bug (e.g. a
/// use-after-close race driven from multiple Java threads). This macro degrades
/// that into a safe no-op instead of dereferencing a zero/dangling pointer.
macro_rules! guard_ptr {
    ($ptr:expr, $ret:expr) => {
        if $ptr == 0 {
            return $ret;
        }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeAddRoute(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    method: jstring,
    pattern: jstring,
    handler: jstring, // actually a ConduitHandler object
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let method = jni_get_string_utf(env, method).unwrap_or_default();
    let pattern = jni_get_string_utf(env, pattern).unwrap_or_default();
    let h = pin_handler(env, app, handler);
    let ctx = Arc::clone(&app.ctx);
    let err = Arc::clone(&app.error_handler);

    app.app.route(method, &pattern, move |req| {
        match dispatch(&ctx, h.get(), req, None) {
            Outcome::Resp(r) => r,
            Outcome::None => WebResponse::internal_error("handler returned null"),
            Outcome::Fatal(m) => WebResponse::internal_error(&m),
            Outcome::Err(msg) => {
                // Try the registered error handler; otherwise a plain 500.
                // Poison-tolerant: a panic in another thread must never make
                // this `.lock()` panic — a panic unwinding out of an extern "C"
                // boundary (e.g. nativeSetErrorHandler on a JVM thread) is UB.
                let eh = *err.lock().unwrap_or_else(|e| e.into_inner());
                if !eh.get().is_null() {
                    match dispatch(&ctx, eh.get(), req, Some(&msg)) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeAddBefore(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    handler: jstring,
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let h = pin_handler(env, app, handler);
    let ctx = Arc::clone(&app.ctx);
    app.app.before(move |req| match dispatch(&ctx, h.get(), req, None) {
        Outcome::None => None,
        Outcome::Resp(r) => Some(r),
        Outcome::Err(msg) => Some(WebResponse::internal_error(&msg)),
        Outcome::Fatal(m) => Some(WebResponse::internal_error(&m)),
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeAddAfter(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    handler: jstring,
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let h = pin_handler(env, app, handler);
    let ctx = Arc::clone(&app.ctx);
    // After filter: returning a Response replaces the previous one; null keeps it.
    app.app.after_response(move |req, prev| match dispatch(&ctx, h.get(), req, None) {
        Outcome::Resp(r) => r,
        _ => prev,
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeSetNotFound(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    handler: jstring,
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let h = pin_handler(env, app, handler);
    let ctx = Arc::clone(&app.ctx);
    app.app.not_found(move |req| match dispatch(&ctx, h.get(), req, None) {
        Outcome::Resp(r) => r,
        _ => WebResponse::not_found(),
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeSetErrorHandler(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    handler: jstring,
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let h = pin_handler(env, app, handler);
    // Record in the shared slot so route closures can find it, and also wire
    // the facade's on_error (which fires for web-core-level handler panics).
    // Poison-tolerant lock: never panic out of this extern "C" function.
    *app.error_handler.lock().unwrap_or_else(|e| e.into_inner()) = h;
    let ctx = Arc::clone(&app.ctx);
    app.app.on_error(move |req, msg| match dispatch(&ctx, h.get(), req, Some(msg)) {
        Outcome::Resp(r) => r,
        _ => WebResponse::internal_error(msg),
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeSetSetting(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    key: jstring,
    value: jstring,
) {
    guard_ptr!(app_ptr, ());
    let app = &mut *(app_ptr as *mut NativeApp);
    let key = jni_get_string_utf(env, key).unwrap_or_default();
    let value = jni_get_string_utf(env, value).unwrap_or_default();
    app.app.set(key, value);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeGetSetting(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    key: jstring,
) -> jstring {
    guard_ptr!(app_ptr, std::ptr::null_mut());
    let app = &mut *(app_ptr as *mut NativeApp);
    let key = jni_get_string_utf(env, key).unwrap_or_default();
    match app.app.setting(&key) {
        Some(v) => jni_new_string_utf(env, v),
        None => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeNewServer(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
    host: jstring,
    port: jint,
    max_conn: jint,
) -> jlong {
    // Consume the NativeApp box.
    guard_ptr!(app_ptr, 0);
    let app_box = Box::from_raw(app_ptr as *mut NativeApp);
    let NativeApp {
        app,
        ctx,
        handler_refs,
        error_handler: _,
    } = *app_box;

    let host = jni_get_string_utf(env, host).unwrap_or_else(|| "127.0.0.1".to_string());
    let port = if (0..=65535).contains(&port) { port as u16 } else { 0 };
    let max_conn = if max_conn > 0 { max_conn as usize } else { 128 };

    let mut opts = HttpServerOptions::default();
    opts.tcp.max_connections = max_conn;

    let server = match ConduitServer::bind_with_options(&host, port, opts, app) {
        Ok(s) => s,
        Err(e) => {
            // Free the global refs we'd otherwise leak, then throw.
            for r in &handler_refs {
                jni_delete_global_ref(env, r.0);
            }
            jni_throw_new(
                env,
                "java/lang/RuntimeException",
                &format!("conduit_jni: bind failed: {e}"),
            );
            return 0;
        }
    };

    let port_bound = server.local_addr().port();
    let stop = server.stop_handle();

    let native_server = NativeServer {
        server: Some(server),
        stop,
        port: port_bound,
        running: Arc::new(AtomicBool::new(false)),
        bg: None,
        ctx,
        handler_refs,
    };
    Box::into_raw(Box::new(native_server)) as jlong
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeServe(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    if let Some(mut s) = srv.server.take() {
        srv.running.store(true, Ordering::SeqCst);
        let _ = s.serve();
        srv.running.store(false, Ordering::SeqCst);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeServeBackground(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    if let Some(mut s) = srv.server.take() {
        let running = Arc::clone(&srv.running);
        running.store(true, Ordering::SeqCst);
        let handle = std::thread::spawn(move || {
            let _ = s.serve();
            running.store(false, Ordering::SeqCst);
        });
        srv.bg = Some(handle);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeStop(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    guard_ptr!(server_ptr, ());
    let srv = &mut *(server_ptr as *mut NativeServer);
    srv.stop.stop();
    if let Some(h) = srv.bg.take() {
        let _ = h.join();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeLocalPort(
    _env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) -> jint {
    guard_ptr!(server_ptr, 0);
    let srv = &*(server_ptr as *mut NativeServer);
    srv.port as jint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeRunning(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeDisposeServer(
    env: *mut JNIEnv,
    _class: jclass,
    server_ptr: jlong,
) {
    if server_ptr == 0 {
        return;
    }
    let mut srv = Box::from_raw(server_ptr as *mut NativeServer);
    srv.stop.stop();
    if let Some(h) = srv.bg.take() {
        let _ = h.join();
    }
    for r in &srv.handler_refs {
        jni_delete_global_ref(env, r.0);
    }
    // srv (and any remaining ConduitServer) dropped here.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_codingadventures_conduit_Native_nativeDisposeApp(
    env: *mut JNIEnv,
    _class: jclass,
    app_ptr: jlong,
) {
    if app_ptr == 0 {
        return;
    }
    let app = Box::from_raw(app_ptr as *mut NativeApp);
    for r in &app.handler_refs {
        jni_delete_global_ref(env, r.0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure-Rust unit tests (no JVM required)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_round_trips_reserved_chars() {
        for s in ["", "abc", "a=b&c", "hello world", "+plus", "ünïcode", "a/b:c?d"] {
            assert_eq!(pct_decode(&pct_encode(s)), s, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn pct_decode_handles_trailing_escape() {
        // A value ENDING in a reserved char encodes to a trailing %XX; the
        // decoder must still read it. (Guard `i + 2 < len` ⟺ i+2 is a valid
        // index — correct for an escape at the end of the string.)
        assert_eq!(pct_decode("trailing%3D"), "trailing=");
        assert_eq!(pct_decode("%2B"), "+");
        assert_eq!(pct_decode("a%26"), "a&");
        // a dangling/short escape at the very end stays literal (no panic)
        assert_eq!(pct_decode("x%4"), "x%4");
        assert_eq!(pct_decode("x%"), "x%");
        // round-trip for strings that end in reserved chars
        for s in ["k=", "a&", "end/", "100%"] {
            assert_eq!(pct_decode(&pct_encode(s)), s, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn pct_encode_escapes_separators_and_plus() {
        let e = pct_encode("a=b&c d+e");
        assert!(!e.contains('='));
        assert!(!e.contains('&'));
        assert!(!e.contains(' '));
        // literal '+' must be encoded so URLDecoder doesn't turn it into space
        assert!(e.contains("%2B"));
    }

    #[test]
    fn encode_decode_pairs_round_trip() {
        let pairs = vec![
            ("name".to_string(), "Adhithya".to_string()),
            ("q".to_string(), "a=b&c".to_string()),
            ("empty".to_string(), String::new()),
        ];
        let enc = encode_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        let dec = decode_pairs(&enc);
        assert_eq!(dec, pairs);
    }

    #[test]
    fn decode_pairs_handles_empty_and_malformed() {
        assert_eq!(decode_pairs(""), Vec::<(String, String)>::new());
        assert_eq!(decode_pairs("bare"), vec![("bare".to_string(), String::new())]);
        assert_eq!(
            decode_pairs("a=1&&b=2"),
            vec![("a".to_string(), "1".to_string()), ("b".to_string(), "2".to_string())]
        );
    }

    #[test]
    fn header_safe_rejects_crlf_and_colon_in_name() {
        assert!(header_safe("content-type", "text/html"));
        assert!(!header_safe("bad\r\nname", "v"));
        assert!(!header_safe("x:y", "v"));
        assert!(!header_safe("ok", "line1\r\nline2"));
        assert!(!header_safe("", "v"));
        // a colon in the VALUE is fine (e.g. a URL or time)
        assert!(header_safe("location", "http://host:8080/x"));
    }
}
