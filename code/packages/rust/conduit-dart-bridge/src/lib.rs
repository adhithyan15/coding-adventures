// conduit-dart-bridge — Dart FFI bridge for conduit-capi (WEB17)
//
// PROBLEM
// ───────
// Dart's `NativeCallable.isolateLocal` is only safe to call from the Dart
// isolate's own thread (for synchronous re-entrant callbacks during an FFI
// call). When conduit-capi dispatches handlers from a background Rust OS
// thread, the Dart isolate is NOT current on that thread, so
// isolateLocal trampolines crash with "Cannot invoke native callback outside
// an isolate."
//
// SOLUTION
// ────────
// This bridge crate implements a thread-safe request/response channel between
// conduit-capi's background threads and Dart's event loop:
//
//   1. Dart calls `conduit_dart_init(NativeApi.initializeApiDLData)` once
//      at startup to load the Dart DL API function table.
//   2. Dart creates a `RawReceivePort` and calls `conduit_dart_set_port(id)`
//      to tell the bridge where to post messages.
//   3. Dart registers the bridge's handler/before/after/ctx_free function
//      pointers (returned by `conduit_dart_*_fn()`) with conduit-capi
//      instead of NativeCallable function pointers.
//   4. When conduit-capi calls a bridge handler from a Rust OS thread:
//        a. The bridge allocates a pending slot with a Condvar for the response.
//        b. It posts a `List<int>` message to Dart's port via
//           `Dart_PostCObject_DL` (safe from any thread).
//        c. It blocks on the Condvar waiting for Dart to deliver a response.
//   5. Dart's event loop receives the port message, looks up the right
//      closure, calls it, and calls `conduit_dart_complete(slot_id, resp_ptr)`
//      which signals the Condvar.
//   6. The blocked Rust thread wakes up and returns the response to
//      conduit-capi's HTTP layer.
//
// This crate also statically links conduit-capi, so Dart only needs to load
// ONE library (libconduit_dart_bridge.dylib / .so) that provides all symbols.

#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

// ── Dart DL API declarations ───────────────────────────────────────────────
//
// These functions are provided by dart_api_dl.c (compiled by build.rs).
// They are initialised when Dart calls Dart_InitializeApiDL from our
// `conduit_dart_init` export.

#[allow(non_camel_case_types)]
type Dart_Port_DL = i64;

#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
enum Dart_CObject_Type {
    Null = 0,
    Bool = 1,
    Int32 = 2,
    Int64 = 3,
    Double = 4,
    String = 5,
    Array = 6,
    // (further variants exist but we don't use them)
}

// The C Dart_CObject union in dart_native_api.h has many variants.
// The largest variant is `as_external_typed_data`:
//   {Dart_TypedData_Type(4) + pad(4) + intptr_t(8) + *u8(8) + *void(8) + fn_ptr(8)} = 40 bytes
// We use _pad to ensure our Rust union is exactly this size, so Dart can
// safely make temporary in-place modifications during Dart_PostCObject_DL.
#[repr(C)]
#[derive(Copy, Clone)]
union Dart_CObject_Value {
    as_int32: i32,
    as_int64: i64,
    as_array: Dart_CObjectArray,
    // Padding to match the full C union size (40 bytes on 64-bit)
    _pad: [u8; 40],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Dart_CObjectArray {
    length: isize,
    values: *mut *mut Dart_CObject,
}

#[repr(C)]
struct Dart_CObject {
    type_: Dart_CObject_Type,
    value: Dart_CObject_Value,
}

extern "C" {
    // Initialise the DL API from the data provided by NativeApi.initializeApiDLData.
    // This IS a real function in dart_api_dl.c — safe to call as fn.
    fn Dart_InitializeApiDL(data: *mut c_void) -> isize;

    // Dart_PostCObject_DL is a FUNCTION-POINTER VARIABLE in dart_api_dl.c
    // (not a function). Declaring it as `fn` here would call the DATA address
    // directly — a bus error on ARM64 (W^X). Instead we call the thin C shim
    // conduit_dart_post_cobject() defined in conduit_dart_bridge.c, which
    // includes dart_api_dl.h and correctly calls through the pointer.
    fn conduit_dart_post_cobject(port_id: Dart_Port_DL, message: *mut Dart_CObject) -> bool;
}

// ── Globals ────────────────────────────────────────────────────────────────

/// Whether Dart_InitializeApiDL succeeded. Guards all DL function calls.
static DL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// The Dart `SendPort` native port ID. Set once by `conduit_dart_set_port`.
static DART_PORT: AtomicI64 = AtomicI64::new(-1);

/// Monotonically increasing slot IDs.
static NEXT_SLOT_ID: AtomicU64 = AtomicU64::new(1);

// Slot type codes sent to Dart in the message.
const SLOT_TYPE_HANDLER: i32 = 0;
const SLOT_TYPE_BEFORE: i32 = 1;
const SLOT_TYPE_AFTER: i32 = 2;
const SLOT_TYPE_CTX_FREE: i32 = 255; // fire-and-forget, no response needed

/// A pending request slot: Dart reads the info, writes the response, and
/// signals the Condvar to wake the blocked Rust thread.
struct Slot {
    response: Arc<(Mutex<Option<*mut c_void>>, Condvar)>,
}

unsafe impl Send for Slot {}

// Global map of pending slots (slot_id → Slot).
static PENDING: Mutex<Option<HashMap<u64, Slot>>> = Mutex::new(None);

fn ensure_map() {
    let mut guard = PENDING.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

// ── Post a message to Dart and block until Dart signals the response ───────

fn dispatch(ctx: *mut c_void, slot_type: i32, req_ptr: *const c_void, current_resp: *mut c_void)
    -> *mut c_void
{
    ensure_map();

    let slot_id = NEXT_SLOT_ID.fetch_add(1, Ordering::SeqCst);
    // The payload holds a raw `*mut c_void` (hence !Send + !Sync), but the Arc is
    // deliberately shared between the calling thread and the Dart callback thread,
    // which coordinate ownership of that pointer via the Mutex + Condvar. The
    // pointer is only ever dereferenced under the lock, so this cross-thread Arc
    // is intentional rather than the mistake this lint targets.
    #[allow(clippy::arc_with_non_send_sync)]
    let arc = Arc::new((Mutex::new(None::<*mut c_void>), Condvar::new()));

    {
        let mut guard = PENDING.lock().unwrap();
        guard.as_mut().unwrap().insert(slot_id, Slot { response: Arc::clone(&arc) });
    }

    // Build a Dart array: [slot_id (i64), ctx_as_i64, type (i32), req_ptr_as_i64, current_resp_as_i64]
    let mut obj_slot_id = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: slot_id as i64 },
    };
    let mut obj_ctx = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: ctx as i64 },
    };
    let mut obj_type = Dart_CObject {
        type_: Dart_CObject_Type::Int32,
        value: Dart_CObject_Value { as_int32: slot_type },
    };
    let mut obj_req = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: req_ptr as i64 },
    };
    let mut obj_cresp = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: current_resp as i64 },
    };

    let mut ptrs: [*mut Dart_CObject; 5] = [
        &mut obj_slot_id,
        &mut obj_ctx,
        &mut obj_type,
        &mut obj_req,
        &mut obj_cresp,
    ];

    let mut msg = Dart_CObject {
        type_: Dart_CObject_Type::Array,
        value: Dart_CObject_Value {
            as_array: Dart_CObjectArray {
                length: 5,
                values: ptrs.as_mut_ptr(),
            },
        },
    };

    let port = DART_PORT.load(Ordering::SeqCst);
    let posted = DL_INITIALIZED.load(Ordering::SeqCst)
        && port > 0
        && unsafe { conduit_dart_post_cobject(port, &mut msg) };

    if !posted {
        // Port not set or isolate is shutting down — remove the slot and
        // return NULL so conduit-capi uses its error path.
        let mut guard = PENDING.lock().unwrap();
        guard.as_mut().unwrap().remove(&slot_id);
        return std::ptr::null_mut();
    }

    // Block until Dart delivers the response, with a 30-second deadline.
    // Without a timeout, a Dart crash or exception that skips conduit_dart_complete
    // would permanently park this thread, eventually consuming all Tokio workers.
    let timeout = std::time::Duration::from_secs(30);
    let (mutex, cvar) = &*arc;
    let mut guard = mutex.lock().unwrap();
    loop {
        if guard.is_some() {
            return guard.take().unwrap();
        }
        let (new_guard, timed_out) = cvar.wait_timeout(guard, timeout).unwrap();
        guard = new_guard;
        if timed_out.timed_out() && guard.is_none() {
            // Dart never responded. Drop the arc mutex BEFORE taking the PENDING
            // lock (consistent ordering: PENDING > arc) to avoid deadlock.
            drop(guard);
            let mut pending_guard = PENDING.lock().unwrap();
            pending_guard.as_mut().unwrap().remove(&slot_id);
            return std::ptr::null_mut();
        }
    }
}

// ── Ctx-free dispatch (fire-and-forget — Dart just removes from its map) ──

fn dispatch_free(ctx: *mut c_void) {
    if !DL_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }
    let port = DART_PORT.load(Ordering::SeqCst);
    if port <= 0 {
        return;
    }
    let mut obj_ctx = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: ctx as i64 },
    };
    let mut obj_type = Dart_CObject {
        type_: Dart_CObject_Type::Int32,
        value: Dart_CObject_Value { as_int32: SLOT_TYPE_CTX_FREE },
    };
    // For ctx_free we don't need slot_id, req_ptr, or current_resp.
    // Send: [0 (slot_id=0 means free), ctx, type=255, 0, 0]
    let zero = 0i64;
    let mut obj_zero1 = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: zero },
    };
    let mut obj_zero2 = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: zero },
    };
    let mut obj_zero3 = Dart_CObject {
        type_: Dart_CObject_Type::Int64,
        value: Dart_CObject_Value { as_int64: zero },
    };
    let mut ptrs: [*mut Dart_CObject; 5] = [
        &mut obj_zero1,
        &mut obj_ctx,
        &mut obj_type,
        &mut obj_zero2,
        &mut obj_zero3,
    ];
    let mut msg = Dart_CObject {
        type_: Dart_CObject_Type::Array,
        value: Dart_CObject_Value {
            as_array: Dart_CObjectArray {
                length: 5,
                values: ptrs.as_mut_ptr(),
            },
        },
    };
    unsafe { conduit_dart_post_cobject(port, &mut msg) };
}

// ── Bridge trampoline functions ────────────────────────────────────────────
//
// These are the C function pointers Dart registers with conduit-capi via
// conduit_app_add_route / conduit_app_add_before / conduit_app_add_after.

extern "C" fn bridge_handler(ctx: *mut c_void, req: *const c_void) -> *mut c_void {
    dispatch(ctx, SLOT_TYPE_HANDLER, req, std::ptr::null_mut())
}

extern "C" fn bridge_before(ctx: *mut c_void, req: *const c_void) -> *mut c_void {
    dispatch(ctx, SLOT_TYPE_BEFORE, req, std::ptr::null_mut())
}

extern "C" fn bridge_after(ctx: *mut c_void, req: *const c_void, current: *mut c_void) -> *mut c_void {
    dispatch(ctx, SLOT_TYPE_AFTER, req, current)
}

extern "C" fn bridge_ctx_free(ctx: *mut c_void) {
    dispatch_free(ctx);
}

// ── Exported C API ─────────────────────────────────────────────────────────

/// Initialise the Dart DL API. Dart calls this once at startup:
///   `conduit_dart_init(NativeApi.initializeApiDLData)`
///
/// Returns 0 on success. Sets DL_INITIALIZED so that dispatch functions
/// know they can safely call Dart_PostCObject_DL.
#[no_mangle]
pub unsafe extern "C" fn conduit_dart_init(data: *mut c_void) -> isize {
    let rc = Dart_InitializeApiDL(data);
    if rc == 0 {
        DL_INITIALIZED.store(true, Ordering::SeqCst);
    }
    rc
}

/// Register the Dart receive port. Dart calls this once after creating a
/// `RawReceivePort` and passing its `.sendPort.nativePort`.
#[no_mangle]
pub extern "C" fn conduit_dart_set_port(port_id: i64) {
    DART_PORT.store(port_id, Ordering::SeqCst);
}

/// Return the bridge handler function pointer (for route handlers).
/// Dart passes this instead of a NativeCallable to `conduit_app_add_route`.
#[no_mangle]
pub extern "C" fn conduit_dart_handler_fn() -> *const c_void {
    bridge_handler as *const c_void
}

/// Return the bridge before-filter function pointer.
#[no_mangle]
pub extern "C" fn conduit_dart_before_fn() -> *const c_void {
    bridge_before as *const c_void
}

/// Return the bridge after-hook function pointer.
#[no_mangle]
pub extern "C" fn conduit_dart_after_fn() -> *const c_void {
    bridge_after as *const c_void
}

/// Return the bridge ctx-free function pointer.
#[no_mangle]
pub extern "C" fn conduit_dart_ctx_free_fn() -> *const c_void {
    bridge_ctx_free as *const c_void
}

/// Deliver a response from Dart. Dart calls this after processing a request
/// message from the receive port. `slot_id` is element [0] of the message;
/// `response_ptr` is the native ConduitResponse*.
///
/// Signals the Condvar on the waiting Rust thread.
#[no_mangle]
pub unsafe extern "C" fn conduit_dart_complete(slot_id: u64, response_ptr: *mut c_void) {
    let arc = {
        let mut guard = PENDING.lock().unwrap();
        guard.as_mut().and_then(|m| m.remove(&slot_id)).map(|s| s.response)
    };
    if let Some(arc) = arc {
        let (mutex, cvar) = &*arc;
        let mut guard = mutex.lock().unwrap();
        *guard = Some(response_ptr);
        cvar.notify_one();
    }
}
