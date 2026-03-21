//! # node-bridge — Thin safe wrapper over Node.js N-API
//!
//! This crate replaces napi-rs with ~350 lines of explicit, debuggable code.
//! It wraps only the raw N-API functions from `napi-sys` needed to build
//! Node.js native addons.
//!
//! ## How N-API works
//!
//! N-API is Node.js's stable C API for native addons. Unlike Ruby/Python
//! where there's a global interpreter state, N-API is **stateless** — every
//! function takes a `napi_env` handle as its first parameter. This handle
//! represents the current JavaScript execution context.
//!
//! N-API is ABI-stable: an addon built against N-API version 4 works on
//! any Node.js version that supports N-API 4+, without recompilation.
//!
//! ## Example
//!
//! ```rust,ignore
//! use node_bridge::*;
//!
//! #[no_mangle]
//! unsafe extern "C" fn napi_register_module_v1(
//!     env: napi_env,
//!     exports: napi_value,
//! ) -> napi_value {
//!     // define class, methods, etc.
//!     exports
//! }
//! ```

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

pub use napi_sys::{napi_callback_info, napi_env, napi_status, napi_value};

// ---------------------------------------------------------------------------
// Status checking
// ---------------------------------------------------------------------------

/// Check if an N-API call succeeded. Panics with message if not.
fn check_status(status: napi_status, msg: &str) {
    if status != napi_sys::napi_status::napi_ok {
        panic!("N-API error (status {:?}): {}", status, msg);
    }
}

// ---------------------------------------------------------------------------
// String conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` to a JavaScript string.
pub fn str_to_js(env: napi_env, s: &str) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe {
        napi_sys::napi_create_string_utf8(
            env,
            s.as_ptr() as *const c_char,
            s.len(),
            &mut result,
        )
    };
    check_status(status, "napi_create_string_utf8");
    result
}

/// Convert a JavaScript string to a Rust `String`.
///
/// Returns `None` if the value is not a string.
pub fn str_from_js(env: napi_env, val: napi_value) -> Option<String> {
    // First, get the string length.
    let mut len: usize = 0;
    let status = unsafe {
        napi_sys::napi_get_value_string_utf8(env, val, ptr::null_mut(), 0, &mut len)
    };
    if status != napi_sys::napi_status::napi_ok {
        return None;
    }

    // Allocate buffer and read the string.
    let mut buf = vec![0u8; len + 1];
    let mut actual_len: usize = 0;
    let status = unsafe {
        napi_sys::napi_get_value_string_utf8(
            env,
            val,
            buf.as_mut_ptr() as *mut c_char,
            buf.len(),
            &mut actual_len,
        )
    };
    if status != napi_sys::napi_status::napi_ok {
        return None;
    }

    buf.truncate(actual_len);
    String::from_utf8(buf).ok()
}

// ---------------------------------------------------------------------------
// Array conversion
// ---------------------------------------------------------------------------

/// Create an empty JavaScript array.
pub fn array_new(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_sys::napi_create_array(env, &mut result) };
    check_status(status, "napi_create_array");
    result
}

/// Get the length of a JavaScript array.
pub fn array_len(env: napi_env, array: napi_value) -> u32 {
    let mut len: u32 = 0;
    let status = unsafe { napi_sys::napi_get_array_length(env, array, &mut len) };
    check_status(status, "napi_get_array_length");
    len
}

/// Get an element from a JavaScript array by index.
pub fn array_get(env: napi_env, array: napi_value, index: u32) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_sys::napi_get_element(env, array, index, &mut result) };
    check_status(status, "napi_get_element");
    result
}

/// Set an element in a JavaScript array at index.
pub fn array_set(env: napi_env, array: napi_value, index: u32, value: napi_value) {
    let status = unsafe { napi_sys::napi_set_element(env, array, index, value) };
    check_status(status, "napi_set_element");
}

/// Convert a `Vec<String>` to a JavaScript array of strings.
pub fn vec_str_to_js(env: napi_env, items: &[String]) -> napi_value {
    let arr = array_new(env);
    for (i, item) in items.iter().enumerate() {
        array_set(env, arr, i as u32, str_to_js(env, item));
    }
    arr
}

/// Convert a JavaScript array of strings to a `Vec<String>`.
pub fn vec_str_from_js(env: napi_env, val: napi_value) -> Vec<String> {
    let len = array_len(env, val);
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        let elem = array_get(env, val, i);
        if let Some(s) = str_from_js(env, elem) {
            result.push(s);
        }
    }
    result
}

/// Convert a `Vec<Vec<String>>` to a JavaScript array of arrays of strings.
pub fn vec_vec_str_to_js(env: napi_env, items: &[Vec<String>]) -> napi_value {
    let arr = array_new(env);
    for (i, group) in items.iter().enumerate() {
        array_set(env, arr, i as u32, vec_str_to_js(env, group));
    }
    arr
}

/// Convert `Vec<(String, String)>` to a JavaScript array of [from, to] arrays.
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

// ---------------------------------------------------------------------------
// Boolean conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `bool` to a JavaScript boolean.
pub fn bool_to_js(env: napi_env, b: bool) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_sys::napi_get_boolean(env, b, &mut result) };
    check_status(status, "napi_get_boolean");
    result
}

// ---------------------------------------------------------------------------
// Number conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `usize` to a JavaScript number.
pub fn usize_to_js(env: napi_env, n: usize) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe {
        napi_sys::napi_create_int64(env, n as i64, &mut result)
    };
    check_status(status, "napi_create_int64");
    result
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

/// Extract arguments from a N-API callback.
///
/// Returns (this, args) where `this` is the JS `this` value and `args`
/// are the function arguments.
pub fn get_cb_info(
    env: napi_env,
    info: napi_callback_info,
    max_args: usize,
) -> (napi_value, Vec<napi_value>) {
    let mut this: napi_value = ptr::null_mut();
    let mut argc = max_args;
    let mut argv: Vec<napi_value> = vec![ptr::null_mut(); max_args];

    let status = unsafe {
        napi_sys::napi_get_cb_info(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            &mut this,
            ptr::null_mut(),
        )
    };
    check_status(status, "napi_get_cb_info");
    argv.truncate(argc);
    (this, argv)
}

// ---------------------------------------------------------------------------
// Data wrapping — store a Rust struct inside a JS object
// ---------------------------------------------------------------------------

/// Wrap a Rust value inside a JavaScript object.
///
/// The data is heap-allocated (Box) and the destructor runs when the
/// JS object is garbage collected.
pub fn wrap_data<T>(env: napi_env, this: napi_value, data: T) {
    let boxed = Box::into_raw(Box::new(data));
    let status = unsafe {
        napi_sys::napi_wrap(
            env,
            this,
            boxed as *mut c_void,
            Some(free_data::<T>),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    check_status(status, "napi_wrap");
}

/// Extract a reference to the Rust data wrapped inside a JS object.
pub unsafe fn unwrap_data<T>(env: napi_env, this: napi_value) -> &'static T {
    let mut ptr: *mut c_void = ptr::null_mut();
    let status = napi_sys::napi_unwrap(env, this, &mut ptr);
    check_status(status, "napi_unwrap");
    &*(ptr as *const T)
}

/// Extract a mutable reference to the Rust data wrapped inside a JS object.
pub unsafe fn unwrap_data_mut<T>(env: napi_env, this: napi_value) -> &'static mut T {
    let mut ptr: *mut c_void = ptr::null_mut();
    let status = napi_sys::napi_unwrap(env, this, &mut ptr);
    check_status(status, "napi_unwrap");
    &mut *(ptr as *mut T)
}

/// Destructor called by V8's GC when the JS object is collected.
unsafe extern "C" fn free_data<T>(
    _env: napi_env,
    data: *mut c_void,
    _hint: *mut c_void,
) {
    if !data.is_null() {
        let _ = Box::from_raw(data as *mut T);
    }
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

/// Throw a JavaScript Error with the given message.
pub fn throw_error(env: napi_env, msg: &str) {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    unsafe {
        napi_sys::napi_throw_error(env, ptr::null(), c_msg.as_ptr());
    }
}

/// Return JavaScript `undefined`.
pub fn undefined(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_sys::napi_get_undefined(env, &mut result) };
    check_status(status, "napi_get_undefined");
    result
}

/// Return JavaScript `null`.
pub fn null(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe { napi_sys::napi_get_null(env, &mut result) };
    check_status(status, "napi_get_null");
    result
}

// ---------------------------------------------------------------------------
// Class definition
// ---------------------------------------------------------------------------

/// Define a property descriptor for a class method.
pub fn method_property(
    env: napi_env,
    name: &str,
    method: napi_sys::napi_callback,
) -> napi_sys::napi_property_descriptor {
    let c_name = CString::new(name).expect("method name must not contain NUL");
    napi_sys::napi_property_descriptor {
        utf8name: c_name.into_raw(),
        name: ptr::null_mut(),
        method: Some(method),
        getter: None,
        setter: None,
        value: ptr::null_mut(),
        attributes: napi_sys::napi_property_attributes::napi_default_method,
        data: ptr::null_mut(),
    }
}

/// Define a JavaScript class with a constructor and methods.
pub fn define_class(
    env: napi_env,
    name: &str,
    constructor: napi_sys::napi_callback,
    properties: &[napi_sys::napi_property_descriptor],
) -> napi_value {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    let mut result: napi_value = ptr::null_mut();
    let status = unsafe {
        napi_sys::napi_define_class(
            env,
            c_name.as_ptr(),
            napi_sys::NAPI_AUTO_LENGTH,
            Some(constructor),
            ptr::null_mut(),
            properties.len(),
            properties.as_ptr(),
            &mut result,
        )
    };
    check_status(status, "napi_define_class");
    result
}

/// Set a named property on a JavaScript object (e.g., module.exports.MyClass = ...).
pub fn set_named_property(env: napi_env, object: napi_value, name: &str, value: napi_value) {
    let c_name = CString::new(name).expect("property name must not contain NUL");
    let status = unsafe {
        napi_sys::napi_set_named_property(env, object, c_name.as_ptr(), value)
    };
    check_status(status, "napi_set_named_property");
}
