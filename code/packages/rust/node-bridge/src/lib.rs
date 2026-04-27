// We use snake_case names to match the N-API C convention exactly.
#![allow(non_camel_case_types)]

//! # node-bridge — Zero-dependency Rust wrapper for Node.js N-API
//!
//! This crate provides safe Rust wrappers around Node.js N-API using raw
//! `extern "C"` declarations. No napi-sys, no napi-rs, no bindgen, no
//! build-time header requirements. Compiles on any platform with just
//! a Rust toolchain.
//!
//! ## How it works
//!
//! N-API is Node.js's stable C API for native addons. It was specifically
//! designed for ABI stability — addons built against N-API v4 work on any
//! Node.js version that supports v4+. We declare the functions as
//! `extern "C"` and the dynamic linker resolves them at load time.
//!
//! ## Key difference from Python/Ruby
//!
//! N-API is **stateless** — every function takes a `napi_env` handle as
//! its first parameter. This handle represents the current JS execution
//! context. There are no global variables to access.

use std::ffi::{c_char, c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// Opaque types — N-API uses opaque pointer types for everything
// ---------------------------------------------------------------------------

/// Opaque N-API environment handle.
#[repr(C)]
pub struct napi_env__ {
    _opaque: [u8; 0],
}
pub type napi_env = *mut napi_env__;

/// Opaque N-API value (represents any JS value).
#[repr(C)]
pub struct napi_value__ {
    _opaque: [u8; 0],
}
pub type napi_value = *mut napi_value__;

/// Opaque callback info (passed to native function callbacks).
#[repr(C)]
pub struct napi_callback_info__ {
    _opaque: [u8; 0],
}
pub type napi_callback_info = *mut napi_callback_info__;

/// Opaque stable reference to a JS value — survives GC moves.
///
/// `napi_ref` is like a `Pin<Box<napi_value>>`: it keeps the underlying
/// JS value alive and reachable even when the originating `napi_value`
/// (a raw stack pointer) is no longer valid. Use when you need to store
/// a JS value across multiple N-API calls or callbacks.
///
/// Lifecycle: `napi_create_reference` → `napi_get_reference_value`
///          → `napi_delete_reference`.
pub type napi_ref = *mut c_void;

/// Opaque handle to a threadsafe function.
///
/// A `napi_threadsafe_function` wraps a JS callback so that it can be
/// invoked from **any** OS thread. N-API queues a `call_js_cb` onto the
/// V8 event loop; that callback runs on the main thread with a live
/// `napi_env`.
///
/// Lifecycle:
///   `napi_create_threadsafe_function` (on V8 thread)
/// → `napi_acquire_threadsafe_function` (on each background thread that
///    will call it — optional, but required if more than one Rust thread
///    shares the TSFN)
/// → `napi_call_threadsafe_function` (from any thread, blocking or not)
/// → `napi_release_threadsafe_function` (from the last thread that called
///    `acquire`, or from the creating thread)
pub type napi_threadsafe_function = *mut c_void;

/// Whether `napi_call_threadsafe_function` blocks when the queue is full.
pub type napi_threadsafe_function_call_mode = i32;
/// Non-blocking: returns `napi_queue_full` if the queue is at capacity.
pub const NAPI_TSFN_NONBLOCKING: napi_threadsafe_function_call_mode = 0;
/// Blocking: waits until there is space in the queue.
pub const NAPI_TSFN_BLOCKING: napi_threadsafe_function_call_mode = 1;

/// Whether `napi_release_threadsafe_function` aborts or waits.
pub type napi_threadsafe_function_release_mode = i32;
/// Graceful release: outstanding calls still run.
pub const NAPI_TSFN_RELEASE: napi_threadsafe_function_release_mode = 0;
/// Abort: discard queued calls that have not yet run.
pub const NAPI_TSFN_ABORT: napi_threadsafe_function_release_mode = 1;

/// Signature of the callback that runs on the V8 main thread when a
/// threadsafe function fires.
///
/// Parameters:
/// - `env`     — live N-API env (call N-API here, not from other threads)
/// - `js_cb`   — the JS function passed to `napi_create_threadsafe_function`
/// - `context` — the `context` pointer passed at creation time
/// - `data`    — the per-call `data` pointer from `napi_call_threadsafe_function`
pub type napi_threadsafe_function_call_js =
    Option<unsafe extern "C" fn(env: napi_env, js_cb: napi_value, context: *mut c_void, data: *mut c_void)>;

/// JS value type discriminant returned by `napi_typeof`.
///
/// Maps exactly to the `napi_valuetype` enum in `node_api_types.h`.
pub type napi_valuetype = i32;
pub const NAPI_UNDEFINED: napi_valuetype = 0;
pub const NAPI_NULL:      napi_valuetype = 1;
pub const NAPI_BOOLEAN:   napi_valuetype = 2;
pub const NAPI_NUMBER:    napi_valuetype = 3;
pub const NAPI_STRING:    napi_valuetype = 4;
pub const NAPI_SYMBOL:    napi_valuetype = 5;
pub const NAPI_OBJECT:    napi_valuetype = 6;
pub const NAPI_FUNCTION:  napi_valuetype = 7;
pub const NAPI_EXTERNAL:  napi_valuetype = 8;
pub const NAPI_BIGINT:    napi_valuetype = 9;

/// N-API status codes.
pub type napi_status = i32;
pub const NAPI_OK: napi_status = 0;

/// N-API callback function signature.
pub type napi_callback =
    Option<unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value>;

/// N-API destructor for wrapped data.
pub type napi_finalize =
    Option<unsafe extern "C" fn(env: napi_env, data: *mut c_void, hint: *mut c_void)>;

/// Property attributes.
pub type napi_property_attributes = i32;
pub const NAPI_DEFAULT: napi_property_attributes = 0;
pub const NAPI_DEFAULT_METHOD: napi_property_attributes = 0;

/// Property descriptor — describes one method/property on a class.
#[repr(C)]
pub struct napi_property_descriptor {
    pub utf8name: *const c_char,
    pub name: napi_value,
    pub method: napi_callback,
    pub getter: napi_callback,
    pub setter: napi_callback,
    pub value: napi_value,
    pub attributes: napi_property_attributes,
    pub data: *mut c_void,
}

// ---------------------------------------------------------------------------
// N-API extern "C" declarations
// ---------------------------------------------------------------------------

extern "C" {
    // -- String operations -------------------------------------------------
    pub fn napi_create_string_utf8(
        env: napi_env,
        str: *const c_char,
        length: usize,
        result: *mut napi_value,
    ) -> napi_status;

    pub fn napi_get_value_string_utf8(
        env: napi_env,
        value: napi_value,
        buf: *mut c_char,
        bufsize: usize,
        result: *mut usize,
    ) -> napi_status;

    // -- Array operations --------------------------------------------------
    pub fn napi_create_array(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_get_array_length(env: napi_env, value: napi_value, result: *mut u32)
        -> napi_status;
    pub fn napi_get_element(
        env: napi_env,
        object: napi_value,
        index: u32,
        result: *mut napi_value,
    ) -> napi_status;
    pub fn napi_set_element(
        env: napi_env,
        object: napi_value,
        index: u32,
        value: napi_value,
    ) -> napi_status;

    // -- Boolean operations ------------------------------------------------
    pub fn napi_get_boolean(env: napi_env, value: bool, result: *mut napi_value) -> napi_status;

    // -- Number operations -------------------------------------------------
    pub fn napi_create_int64(env: napi_env, value: i64, result: *mut napi_value) -> napi_status;
    pub fn napi_create_double(env: napi_env, value: f64, result: *mut napi_value) -> napi_status;
    pub fn napi_get_value_double(
        env: napi_env,
        value: napi_value,
        result: *mut f64,
    ) -> napi_status;

    // -- Undefined/null ----------------------------------------------------
    pub fn napi_get_undefined(env: napi_env, result: *mut napi_value) -> napi_status;
    pub fn napi_get_null(env: napi_env, result: *mut napi_value) -> napi_status;

    // -- Callback info -----------------------------------------------------
    pub fn napi_get_cb_info(
        env: napi_env,
        cbinfo: napi_callback_info,
        argc: *mut usize,
        argv: *mut napi_value,
        this_arg: *mut napi_value,
        data: *mut *mut c_void,
    ) -> napi_status;

    // -- Object wrapping ---------------------------------------------------
    pub fn napi_wrap(
        env: napi_env,
        js_object: napi_value,
        native_object: *mut c_void,
        finalize_cb: napi_finalize,
        finalize_hint: *mut c_void,
        result: *mut *mut c_void, // napi_ref, but we don't need it
    ) -> napi_status;

    pub fn napi_unwrap(
        env: napi_env,
        js_object: napi_value,
        result: *mut *mut c_void,
    ) -> napi_status;

    // -- Class definition --------------------------------------------------
    pub fn napi_define_class(
        env: napi_env,
        utf8name: *const c_char,
        length: usize,
        constructor: napi_callback,
        data: *mut c_void,
        property_count: usize,
        properties: *const napi_property_descriptor,
        result: *mut napi_value,
    ) -> napi_status;

    // -- Function creation -------------------------------------------------
    //
    // napi_create_function creates a standalone JS function that is NOT
    // attached to any class. This is the right primitive for module-level
    // exports (e.g. `module.exports.markdownToHtml = <function>`).
    // `utf8name` sets the function's `.name` property; `length` is
    // NAPI_AUTO_LENGTH (usize::MAX) to use strlen. `data` is a user context
    // pointer passed to the callback; pass null if unused.
    pub fn napi_create_function(
        env: napi_env,
        utf8name: *const c_char,
        length: usize,
        cb: napi_callback,
        data: *const c_void,
        result: *mut napi_value,
    ) -> napi_status;

    // -- Property setting --------------------------------------------------
    pub fn napi_set_named_property(
        env: napi_env,
        object: napi_value,
        utf8name: *const c_char,
        value: napi_value,
    ) -> napi_status;

    // -- Error handling ----------------------------------------------------
    pub fn napi_throw_error(env: napi_env, code: *const c_char, msg: *const c_char) -> napi_status;

    // -- Value type inspection ---------------------------------------------

    /// Return the JS type of `value` as a `napi_valuetype` discriminant.
    ///
    /// Equivalent to `typeof value` in JS but also distinguishes `null`
    /// (NAPI_NULL) from `undefined` (NAPI_UNDEFINED).
    pub fn napi_typeof(
        env: napi_env,
        value: napi_value,
        result: *mut napi_valuetype,
    ) -> napi_status;

    /// Write `true` into `*result` when `value` is a JS Array.
    pub fn napi_is_array(env: napi_env, value: napi_value, result: *mut bool) -> napi_status;

    // -- Additional number operations --------------------------------------

    /// Extract an `i32` from a JS number value.
    pub fn napi_get_value_int32(env: napi_env, value: napi_value, result: *mut i32) -> napi_status;

    /// Extract a `bool` from a JS boolean value.
    pub fn napi_get_value_bool(env: napi_env, value: napi_value, result: *mut bool) -> napi_status;

    // -- Object operations -------------------------------------------------

    /// Create an empty `{}` JS object.
    pub fn napi_create_object(env: napi_env, result: *mut napi_value) -> napi_status;

    /// Read a named property from `object`, equivalent to `object[utf8name]`.
    pub fn napi_get_named_property(
        env: napi_env,
        object: napi_value,
        utf8name: *const c_char,
        result: *mut napi_value,
    ) -> napi_status;

    /// Read a property using a `napi_value` key: equivalent to `object[key]`.
    ///
    /// Unlike `napi_get_named_property`, the key can be any JS value (string,
    /// symbol, number index).  Use this when iterating property names from
    /// `napi_get_property_names`.
    pub fn napi_get_property(
        env: napi_env,
        object: napi_value,
        key: napi_value,
        result: *mut napi_value,
    ) -> napi_status;

    /// Return an array of all own enumerable property names of `object`.
    ///
    /// The returned `napi_value` is a JS `string[]`.  Equivalent to
    /// `Object.keys(object)`.
    pub fn napi_get_property_names(
        env: napi_env,
        object: napi_value,
        result: *mut napi_value,
    ) -> napi_status;

    /// Set a property using a `napi_value` key: equivalent to `object[key] = value`.
    pub fn napi_set_property(
        env: napi_env,
        object: napi_value,
        key: napi_value,
        value: napi_value,
    ) -> napi_status;

    // -- Function calls ----------------------------------------------------

    /// Call a JS function: equivalent to `func.call(recv, ...argv[0..argc])`.
    ///
    /// - `recv`   — the `this` value (pass `napi_get_undefined` for free functions)
    /// - `argc`   — number of arguments
    /// - `argv`   — pointer to array of `napi_value` arguments
    /// - `result` — receives the return value (may be null if caller doesn't care)
    pub fn napi_call_function(
        env: napi_env,
        recv: napi_value,
        func: napi_value,
        argc: usize,
        argv: *const napi_value,
        result: *mut napi_value,
    ) -> napi_status;

    /// Construct a new instance: equivalent to `new constructor(...argv)`.
    pub fn napi_new_instance(
        env: napi_env,
        constructor: napi_value,
        argc: usize,
        argv: *const napi_value,
        result: *mut napi_value,
    ) -> napi_status;

    // -- Exception / pending error handling --------------------------------

    /// Write `true` into `*result` when a JS exception is pending in `env`.
    ///
    /// A pending exception means the last N-API call that can throw did throw.
    /// You MUST clear it (with `napi_get_and_clear_last_exception`) before
    /// calling most other N-API functions.
    pub fn napi_is_exception_pending(env: napi_env, result: *mut bool) -> napi_status;

    /// Retrieve and clear the pending JS exception.
    ///
    /// Returns the thrown value (any JS value, typically an Error object).
    /// After this call no exception is pending and normal N-API usage resumes.
    pub fn napi_get_and_clear_last_exception(env: napi_env, result: *mut napi_value) -> napi_status;

    // -- Stable references -------------------------------------------------

    /// Create a stable reference to `value` with the given refcount.
    ///
    /// An `napi_ref` keeps the underlying JS object alive even when the
    /// `napi_value` (a raw pointer valid only in the current call frame) goes
    /// out of scope. The reference must be released with `napi_delete_reference`
    /// when no longer needed, or the object will never be GC'd.
    pub fn napi_create_reference(
        env: napi_env,
        value: napi_value,
        initial_refcount: u32,
        result: *mut napi_ref,
    ) -> napi_status;

    /// Retrieve the `napi_value` currently pointed to by a reference.
    pub fn napi_get_reference_value(
        env: napi_env,
        reference: napi_ref,
        result: *mut napi_value,
    ) -> napi_status;

    /// Release a reference, decrementing its refcount. When the refcount
    /// reaches zero the underlying JS object becomes eligible for GC.
    pub fn napi_delete_reference(env: napi_env, reference: napi_ref) -> napi_status;

    // -- Threadsafe functions ----------------------------------------------
    //
    // These allow any OS thread to queue a JS function call onto the V8
    // main thread's event loop. This is the only safe way to call into JS
    // from Rust background threads.

    /// Create a threadsafe wrapper around `func` (a JS function).
    ///
    /// - `async_resource`      — pass `napi_get_undefined(env)` (unused here)
    /// - `async_resource_name` — a JS string for profiling/tracing; usually
    ///                           the handler name
    /// - `max_queue_size`      — 0 means unlimited queue
    /// - `initial_thread_count`— number of threads that will call this TSFN
    ///                           (including the creating thread); typically 1
    /// - `thread_finalize_data`— opaque pointer passed to `thread_finalize_cb`
    /// - `thread_finalize_cb`  — called when all threads have released the TSFN
    /// - `context`             — arbitrary data passed to every `call_js_cb`
    /// - `call_js_cb`          — the function that runs on the V8 thread
    /// - `result`              — receives the new TSFN handle
    pub fn napi_create_threadsafe_function(
        env: napi_env,
        func: napi_value,
        async_resource: napi_value,
        async_resource_name: napi_value,
        max_queue_size: usize,
        initial_thread_count: usize,
        thread_finalize_data: *mut c_void,
        thread_finalize_cb: napi_finalize,
        context: *mut c_void,
        call_js_cb: napi_threadsafe_function_call_js,
        result: *mut napi_threadsafe_function,
    ) -> napi_status;

    /// Increment the thread-use-count of the TSFN so that an additional thread
    /// can safely call `napi_call_threadsafe_function`.
    pub fn napi_acquire_threadsafe_function(tsfn: napi_threadsafe_function) -> napi_status;

    /// Queue a call to the wrapped JS function from any thread.
    ///
    /// - `data`       — arbitrary pointer forwarded to `call_js_cb` as `data`
    /// - `is_blocking`— `NAPI_TSFN_BLOCKING` or `NAPI_TSFN_NONBLOCKING`
    pub fn napi_call_threadsafe_function(
        tsfn: napi_threadsafe_function,
        data: *mut c_void,
        is_blocking: napi_threadsafe_function_call_mode,
    ) -> napi_status;

    /// Release the TSFN from this thread. When all threads have released,
    /// the `thread_finalize_cb` is called and the TSFN is destroyed.
    pub fn napi_release_threadsafe_function(
        tsfn: napi_threadsafe_function,
        mode: napi_threadsafe_function_release_mode,
    ) -> napi_status;

    /// Mark the TSFN as keeping the event loop alive (default after creation).
    /// Paired with `napi_unref_threadsafe_function` to let the loop exit.
    pub fn napi_ref_threadsafe_function(env: napi_env, tsfn: napi_threadsafe_function) -> napi_status;

    /// Allow the event loop to exit even if this TSFN is still alive.
    /// Use this when the TSFN is only needed while an explicit `serve()` is
    /// running and should not prevent the process from exiting after `stop()`.
    pub fn napi_unref_threadsafe_function(env: napi_env, tsfn: napi_threadsafe_function) -> napi_status;
}

// ---------------------------------------------------------------------------
// Status checking
// ---------------------------------------------------------------------------

fn check_status(status: napi_status, msg: &str) {
    if status != NAPI_OK {
        panic!("N-API error (status {}): {}", status, msg);
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Strings
// ---------------------------------------------------------------------------

pub fn str_to_js(env: napi_env, s: &str) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status =
        unsafe { napi_create_string_utf8(env, s.as_ptr() as *const c_char, s.len(), &mut result) };
    check_status(status, "napi_create_string_utf8");
    result
}

pub fn str_from_js(env: napi_env, val: napi_value) -> Option<String> {
    let mut len: usize = 0;
    let status = unsafe { napi_get_value_string_utf8(env, val, ptr::null_mut(), 0, &mut len) };
    if status != NAPI_OK {
        return None;
    }
    let mut buf = vec![0u8; len + 1];
    let mut actual_len: usize = 0;
    let status = unsafe {
        napi_get_value_string_utf8(
            env,
            val,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut actual_len,
        )
    };
    if status != NAPI_OK {
        return None;
    }
    buf.truncate(actual_len);
    String::from_utf8(buf).ok()
}

// ---------------------------------------------------------------------------
// Safe wrappers — Arrays
// ---------------------------------------------------------------------------

pub fn array_new(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_create_array(env, &mut result) },
        "napi_create_array",
    );
    result
}

pub fn array_len(env: napi_env, array: napi_value) -> u32 {
    let mut len: u32 = 0;
    check_status(
        unsafe { napi_get_array_length(env, array, &mut len) },
        "napi_get_array_length",
    );
    len
}

pub fn array_get(env: napi_env, array: napi_value, index: u32) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_get_element(env, array, index, &mut result) },
        "napi_get_element",
    );
    result
}

pub fn array_set(env: napi_env, array: napi_value, index: u32, value: napi_value) {
    check_status(
        unsafe { napi_set_element(env, array, index, value) },
        "napi_set_element",
    );
}

pub fn vec_str_to_js(env: napi_env, items: &[String]) -> napi_value {
    let arr = array_new(env);
    for (i, item) in items.iter().enumerate() {
        array_set(env, arr, i as u32, str_to_js(env, item));
    }
    arr
}

pub fn vec_str_from_js(env: napi_env, val: napi_value) -> Vec<String> {
    let len = array_len(env, val);
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        if let Some(s) = str_from_js(env, array_get(env, val, i)) {
            result.push(s);
        }
    }
    result
}

pub fn vec_vec_str_to_js(env: napi_env, items: &[Vec<String>]) -> napi_value {
    let arr = array_new(env);
    for (i, group) in items.iter().enumerate() {
        array_set(env, arr, i as u32, vec_str_to_js(env, group));
    }
    arr
}

pub fn vec_tuple2_str_to_js(env: napi_env, items: &[(String, String)]) -> napi_value {
    let arr = array_new(env);
    for (i, (a, b)) in items.iter().enumerate() {
        let pair = array_new(env);
        array_set(env, pair, 0, str_to_js(env, a));
        array_set(env, pair, 1, str_to_js(env, b));
        array_set(env, arr, i as u32, pair);
    }
    arr
}

pub fn vec_tuple2_str_f64_to_js(env: napi_env, items: &[(String, f64)]) -> napi_value {
    let arr = array_new(env);
    for (i, (key, value)) in items.iter().enumerate() {
        let pair = array_new(env);
        array_set(env, pair, 0, str_to_js(env, key));
        array_set(env, pair, 1, f64_to_js(env, *value));
        array_set(env, arr, i as u32, pair);
    }
    arr
}

pub fn vec_tuple3_str_f64_to_js(
    env: napi_env,
    items: &[(String, String, f64)],
) -> napi_value {
    let arr = array_new(env);
    for (i, (left, right, weight)) in items.iter().enumerate() {
        let triple = array_new(env);
        array_set(env, triple, 0, str_to_js(env, left));
        array_set(env, triple, 1, str_to_js(env, right));
        array_set(env, triple, 2, f64_to_js(env, *weight));
        array_set(env, arr, i as u32, triple);
    }
    arr
}

// ---------------------------------------------------------------------------
// Safe wrappers — Boolean, Number, Undefined, Null
// ---------------------------------------------------------------------------

pub fn bool_to_js(env: napi_env, b: bool) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_get_boolean(env, b, &mut result) },
        "napi_get_boolean",
    );
    result
}

pub fn usize_to_js(env: napi_env, n: usize) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_create_int64(env, n as i64, &mut result) },
        "napi_create_int64",
    );
    result
}

pub fn f64_to_js(env: napi_env, n: f64) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_create_double(env, n, &mut result) },
        "napi_create_double",
    );
    result
}

pub fn f64_from_js(env: napi_env, val: napi_value) -> Option<f64> {
    let mut result = 0.0;
    let status = unsafe { napi_get_value_double(env, val, &mut result) };
    if status != NAPI_OK {
        return None;
    }
    Some(result)
}

pub fn undefined(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_get_undefined(env, &mut result) },
        "napi_get_undefined",
    );
    result
}

pub fn null(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(unsafe { napi_get_null(env, &mut result) }, "napi_get_null");
    result
}

// ---------------------------------------------------------------------------
// Safe wrappers — Callback info
// ---------------------------------------------------------------------------

pub fn get_cb_info(
    env: napi_env,
    info: napi_callback_info,
    max_args: usize,
) -> (napi_value, Vec<napi_value>) {
    let mut this: napi_value = ptr::null_mut();
    let mut argc = max_args;
    let mut argv: Vec<napi_value> = vec![ptr::null_mut(); max_args];
    check_status(
        unsafe {
            napi_get_cb_info(
                env,
                info,
                &mut argc,
                argv.as_mut_ptr(),
                &mut this,
                ptr::null_mut(),
            )
        },
        "napi_get_cb_info",
    );
    argv.truncate(argc);
    (this, argv)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Data wrapping
// ---------------------------------------------------------------------------

pub fn wrap_data<T>(env: napi_env, this: napi_value, data: T) {
    let boxed = Box::into_raw(Box::new(data));
    check_status(
        unsafe {
            napi_wrap(
                env,
                this,
                boxed as *mut c_void,
                Some(free_data::<T>),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        },
        "napi_wrap",
    );
}

pub unsafe fn unwrap_data<T>(env: napi_env, this: napi_value) -> *const T {
    let mut ptr: *mut c_void = ptr::null_mut();
    check_status(napi_unwrap(env, this, &mut ptr), "napi_unwrap");
    assert!(!ptr.is_null(), "napi_unwrap returned a null data pointer");
    ptr as *const T
}

pub unsafe fn unwrap_data_mut<T>(env: napi_env, this: napi_value) -> *mut T {
    let mut ptr: *mut c_void = ptr::null_mut();
    check_status(napi_unwrap(env, this, &mut ptr), "napi_unwrap");
    assert!(!ptr.is_null(), "napi_unwrap returned a null data pointer");
    ptr as *mut T
}

unsafe extern "C" fn free_data<T>(_env: napi_env, data: *mut c_void, _hint: *mut c_void) {
    if !data.is_null() {
        let _ = Box::from_raw(data as *mut T);
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Error handling
// ---------------------------------------------------------------------------

pub fn throw_error(env: napi_env, msg: &str) {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    unsafe {
        napi_throw_error(env, ptr::null(), c_msg.as_ptr());
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Class definition
// ---------------------------------------------------------------------------

pub fn method_property(name: &str, method: napi_callback) -> napi_property_descriptor {
    let c_name = CString::new(name).expect("name must not contain NUL");
    napi_property_descriptor {
        utf8name: c_name.into_raw(),
        name: ptr::null_mut(),
        method,
        getter: None,
        setter: None,
        value: ptr::null_mut(),
        attributes: NAPI_DEFAULT_METHOD,
        data: ptr::null_mut(),
    }
}

pub fn define_class(
    env: napi_env,
    name: &str,
    constructor: napi_callback,
    properties: &[napi_property_descriptor],
) -> napi_value {
    let c_name = CString::new(name).expect("name must not contain NUL");
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe {
            napi_define_class(
                env,
                c_name.as_ptr(),
                usize::MAX, // NAPI_AUTO_LENGTH
                constructor,
                ptr::null_mut(),
                properties.len(),
                properties.as_ptr(),
                &mut result,
            )
        },
        "napi_define_class",
    );
    result
}

pub fn set_named_property(env: napi_env, object: napi_value, name: &str, value: napi_value) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    check_status(
        unsafe { napi_set_named_property(env, object, c_name.as_ptr(), value) },
        "napi_set_named_property",
    );
}

// ---------------------------------------------------------------------------
// Safe wrappers — Standalone function creation
// ---------------------------------------------------------------------------

/// Create a standalone JS function (not attached to any class).
///
/// Use this to expose module-level functions on the addon's exports object:
///
/// ```rust,ignore
/// let f = create_function(env, "markdownToHtml", Some(my_callback));
/// set_named_property(env, exports, "markdownToHtml", f);
/// ```
///
/// The function's `.name` property in JS will be set to `name`.
/// `data` context pointer is `null` — the callback can retrieve it via
/// `napi_get_cb_info` if needed.
pub fn create_function(env: napi_env, name: &str, cb: napi_callback) -> napi_value {
    let c_name = CString::new(name).expect("name must not contain NUL");
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe {
            napi_create_function(
                env,
                c_name.as_ptr(),
                usize::MAX,
                cb,
                ptr::null(),
                &mut result,
            )
        },
        "napi_create_function",
    );
    result
}

// ---------------------------------------------------------------------------
// Safe wrappers — Value type inspection
// ---------------------------------------------------------------------------

/// Return the JS type of `value` (NAPI_UNDEFINED, NAPI_NULL, NAPI_BOOLEAN,
/// NAPI_NUMBER, NAPI_STRING, NAPI_OBJECT, NAPI_FUNCTION, …).
pub fn value_type(env: napi_env, value: napi_value) -> napi_valuetype {
    let mut t: napi_valuetype = NAPI_UNDEFINED;
    check_status(unsafe { napi_typeof(env, value, &mut t) }, "napi_typeof");
    t
}

/// Return `true` when `value` is a JS Array (regular or typed).
pub fn is_array(env: napi_env, value: napi_value) -> bool {
    let mut result = false;
    check_status(
        unsafe { napi_is_array(env, value, &mut result) },
        "napi_is_array",
    );
    result
}

// ---------------------------------------------------------------------------
// Safe wrappers — Additional number extraction
// ---------------------------------------------------------------------------

/// Extract an `i32` from a JS number. Returns `None` if `value` is not a
/// number (e.g. undefined, string, etc.).
pub fn i32_from_js(env: napi_env, value: napi_value) -> Option<i32> {
    let mut n: i32 = 0;
    let status = unsafe { napi_get_value_int32(env, value, &mut n) };
    if status != NAPI_OK { return None; }
    Some(n)
}

/// Extract a `bool` from a JS boolean. Returns `None` if `value` is not a
/// boolean.
pub fn bool_from_js(env: napi_env, value: napi_value) -> Option<bool> {
    let mut b = false;
    let status = unsafe { napi_get_value_bool(env, value, &mut b) };
    if status != NAPI_OK { return None; }
    Some(b)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Objects
// ---------------------------------------------------------------------------

/// Create an empty `{}` JS object.
pub fn object_new(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_create_object(env, &mut result) },
        "napi_create_object",
    );
    result
}

/// Read `object[name]`, returning `undefined` if the property does not exist.
pub fn get_property(env: napi_env, object: napi_value, name: &str) -> napi_value {
    let c_name = CString::new(name).expect("property name must not contain NUL");
    let mut result: napi_value = ptr::null_mut();
    // On failure (e.g. object is not an object) we return undefined.
    let status = unsafe { napi_get_named_property(env, object, c_name.as_ptr(), &mut result) };
    if status != NAPI_OK {
        result = ptr::null_mut();
        unsafe { napi_get_undefined(env, &mut result) };
    }
    result
}

/// Read a property using a `napi_value` key.
///
/// Equivalent to `object[key]` where `key` is a JS value (typically a string
/// returned from `get_property_names`).  Returns undefined on failure.
pub fn get_property_by_key(env: napi_env, object: napi_value, key: napi_value) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_get_property(env, object, key, &mut result) };
    if status != NAPI_OK {
        result = ptr::null_mut();
        unsafe { napi_get_undefined(env, &mut result) };
    }
    result
}

/// Return `Object.keys(object)` as a `napi_value` array of strings.
///
/// Returns an empty array on failure.
pub fn get_property_names(env: napi_env, object: napi_value) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_get_property_names(env, object, &mut result) };
    if status != NAPI_OK {
        // Return an empty array.  napi_create_array() creates a zero-length
        // array; napi_create_array_with_length() would also work but requires
        // an extra declaration — use the already-declared zero-arg form.
        let mut arr: napi_value = ptr::null_mut();
        check_status(
            unsafe { napi_create_array(env, &mut arr) },
            "napi_create_array (empty fallback)",
        );
        return arr;
    }
    result
}

// ---------------------------------------------------------------------------
// Safe wrappers — Function calls
// ---------------------------------------------------------------------------

/// Call `func` with `this = recv` and the given arguments.
///
/// Returns `Some(result)` on success or `None` if an exception was thrown
/// (the exception remains pending; caller must handle it).
pub fn call_function(
    env: napi_env,
    recv: napi_value,
    func: napi_value,
    args: &[napi_value],
) -> Option<napi_value> {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe {
        napi_call_function(
            env,
            recv,
            func,
            args.len(),
            if args.is_empty() { ptr::null() } else { args.as_ptr() },
            &mut result,
        )
    };
    if status != NAPI_OK { None } else { Some(result) }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Exception handling
// ---------------------------------------------------------------------------

/// Return `true` if a JS exception is currently pending in `env`.
pub fn exception_pending(env: napi_env) -> bool {
    let mut pending = false;
    unsafe { napi_is_exception_pending(env, &mut pending) };
    pending
}

/// Retrieve and clear the pending JS exception.
///
/// Returns the thrown JS value. After this call no exception is pending.
/// Panics if no exception is pending (call `exception_pending` first).
pub fn clear_exception(env: napi_env) -> napi_value {
    let mut ex: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_get_and_clear_last_exception(env, &mut ex) },
        "napi_get_and_clear_last_exception",
    );
    ex
}

// ---------------------------------------------------------------------------
// Safe wrappers — Stable references
// ---------------------------------------------------------------------------

/// Create a stable reference to a JS value.
///
/// The underlying object stays alive until `delete_ref` is called, regardless
/// of whether any other JS code holds a reference.
pub fn create_ref(env: napi_env, value: napi_value) -> napi_ref {
    let mut r: napi_ref = ptr::null_mut();
    check_status(
        unsafe { napi_create_reference(env, value, 1, &mut r) },
        "napi_create_reference",
    );
    r
}

/// Retrieve the current `napi_value` from a stable reference.
pub fn deref(env: napi_env, r: napi_ref) -> napi_value {
    let mut val: napi_value = ptr::null_mut();
    check_status(
        unsafe { napi_get_reference_value(env, r, &mut val) },
        "napi_get_reference_value",
    );
    val
}

/// Release a stable reference, allowing the underlying JS object to be GC'd.
pub fn delete_ref(env: napi_env, r: napi_ref) {
    check_status(
        unsafe { napi_delete_reference(env, r) },
        "napi_delete_reference",
    );
}

// ---------------------------------------------------------------------------
// Safe wrappers — Threadsafe functions
// ---------------------------------------------------------------------------

/// Create a threadsafe wrapper around `js_fn`.
///
/// - `name`                — a label used in async profiling output
/// - `max_queue`           — 0 = unlimited; >0 caps the pending-call queue
/// - `initial_thread_count`— how many threads will call `tsfn_call`
///   (including the thread that creates the TSFN, i.e. the V8 thread).
///   Each thread that will call `tsfn_call` must either be counted here or
///   must call `tsfn_acquire` before the first call.
/// - `context`             — arbitrary pointer passed to every `call_js_cb`
/// - `call_js_cb`          — the callback that runs on the V8 thread
pub fn tsfn_create(
    env: napi_env,
    js_fn: napi_value,
    name: &str,
    max_queue: usize,
    initial_thread_count: usize,
    context: *mut c_void,
    call_js_cb: napi_threadsafe_function_call_js,
) -> napi_threadsafe_function {
    // The `async_resource` parameter must be NULL or a JS Object.
    //
    // Node.js v25 source: `RETURN_STATUS_IF_FALSE(env, v8_resource->IsObject(),
    // napi_invalid_arg)` — passing JS `undefined` (which is not an Object) causes
    // `napi_invalid_arg`.  Passing C NULL causes Node.js to create a fresh Object
    // internally, which is the correct way to opt out of async-hook tracking.
    let name_str = str_to_js(env, name);
    let mut tsfn: napi_threadsafe_function = ptr::null_mut();
    check_status(
        unsafe {
            napi_create_threadsafe_function(
                env,
                js_fn,
                ptr::null_mut(), // async_resource = NULL (Node.js creates one internally)
                name_str,        // async_resource_name for profiling / DevTools
                max_queue,
                initial_thread_count,
                ptr::null_mut(), // thread_finalize_data
                None,            // thread_finalize_cb
                context,
                call_js_cb,
                &mut tsfn,
            )
        },
        "napi_create_threadsafe_function",
    );
    tsfn
}

/// Acquire additional access to `tsfn` from the current thread.
///
/// Call this from each background thread that will call `tsfn_call`,
/// unless the thread was already counted in `initial_thread_count` at
/// creation time.
pub fn tsfn_acquire(tsfn: napi_threadsafe_function) {
    check_status(
        unsafe { napi_acquire_threadsafe_function(tsfn) },
        "napi_acquire_threadsafe_function",
    );
}

/// Queue a call to the TSFN's JS callback from any thread.
///
/// `data` is forwarded to `call_js_cb` as its `data` parameter.
/// Pass `NAPI_TSFN_BLOCKING` to block if the queue is full.
pub fn tsfn_call(
    tsfn: napi_threadsafe_function,
    data: *mut c_void,
    mode: napi_threadsafe_function_call_mode,
) {
    check_status(
        unsafe { napi_call_threadsafe_function(tsfn, data, mode) },
        "napi_call_threadsafe_function",
    );
}

/// Release this thread's hold on the TSFN.
///
/// When all threads have released and the refcount reaches zero,
/// the `thread_finalize_cb` is called and the TSFN is destroyed.
pub fn tsfn_release(tsfn: napi_threadsafe_function, mode: napi_threadsafe_function_release_mode) {
    check_status(
        unsafe { napi_release_threadsafe_function(tsfn, mode) },
        "napi_release_threadsafe_function",
    );
}

/// Mark the TSFN as keeping the event loop alive.
///
/// This is the default after creation. Use `tsfn_unref` to allow the
/// process to exit even while the TSFN is alive.
pub fn tsfn_ref(env: napi_env, tsfn: napi_threadsafe_function) {
    check_status(
        unsafe { napi_ref_threadsafe_function(env, tsfn) },
        "napi_ref_threadsafe_function",
    );
}

/// Allow the event loop to exit even if this TSFN is still alive.
pub fn tsfn_unref(env: napi_env, tsfn: napi_threadsafe_function) {
    check_status(
        unsafe { napi_unref_threadsafe_function(env, tsfn) },
        "napi_unref_threadsafe_function",
    );
}
