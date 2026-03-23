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
pub struct napi_env__{
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

/// N-API status codes.
pub type napi_status = i32;
pub const NAPI_OK: napi_status = 0;

/// N-API callback function signature.
pub type napi_callback =
    Option<unsafe extern "C" fn(env: napi_env, info: napi_callback_info) -> napi_value>;

/// N-API destructor for wrapped data.
pub type napi_finalize = Option<
    unsafe extern "C" fn(env: napi_env, data: *mut c_void, hint: *mut c_void),
>;

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
    pub fn napi_get_array_length(env: napi_env, value: napi_value, result: *mut u32) -> napi_status;
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

    // -- Property setting --------------------------------------------------
    pub fn napi_set_named_property(
        env: napi_env,
        object: napi_value,
        utf8name: *const c_char,
        value: napi_value,
    ) -> napi_status;

    // -- Error handling ----------------------------------------------------
    pub fn napi_throw_error(
        env: napi_env,
        code: *const c_char,
        msg: *const c_char,
    ) -> napi_status;
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
    let status = unsafe {
        napi_create_string_utf8(env, s.as_ptr() as *const c_char, s.len(), &mut result)
    };
    check_status(status, "napi_create_string_utf8");
    result
}

pub fn str_from_js(env: napi_env, val: napi_value) -> Option<String> {
    let mut len: usize = 0;
    let status = unsafe {
        napi_get_value_string_utf8(env, val, ptr::null_mut(), 0, &mut len)
    };
    if status != NAPI_OK {
        return None;
    }
    let mut buf = vec![0u8; len + 1];
    let mut actual_len: usize = 0;
    let status = unsafe {
        napi_get_value_string_utf8(
            env, val,
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
    check_status(unsafe { napi_create_array(env, &mut result) }, "napi_create_array");
    result
}

pub fn array_len(env: napi_env, array: napi_value) -> u32 {
    let mut len: u32 = 0;
    check_status(unsafe { napi_get_array_length(env, array, &mut len) }, "napi_get_array_length");
    len
}

pub fn array_get(env: napi_env, array: napi_value, index: u32) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(unsafe { napi_get_element(env, array, index, &mut result) }, "napi_get_element");
    result
}

pub fn array_set(env: napi_env, array: napi_value, index: u32, value: napi_value) {
    check_status(unsafe { napi_set_element(env, array, index, value) }, "napi_set_element");
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

// ---------------------------------------------------------------------------
// Safe wrappers — Boolean, Number, Undefined, Null
// ---------------------------------------------------------------------------

pub fn bool_to_js(env: napi_env, b: bool) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(unsafe { napi_get_boolean(env, b, &mut result) }, "napi_get_boolean");
    result
}

pub fn usize_to_js(env: napi_env, n: usize) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(unsafe { napi_create_int64(env, n as i64, &mut result) }, "napi_create_int64");
    result
}

pub fn undefined(env: napi_env) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    check_status(unsafe { napi_get_undefined(env, &mut result) }, "napi_get_undefined");
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
            napi_get_cb_info(env, info, &mut argc, argv.as_mut_ptr(), &mut this, ptr::null_mut())
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
            napi_wrap(env, this, boxed as *mut c_void, Some(free_data::<T>), ptr::null_mut(), ptr::null_mut())
        },
        "napi_wrap",
    );
}

pub unsafe fn unwrap_data<T>(env: napi_env, this: napi_value) -> &'static T {
    let mut ptr: *mut c_void = ptr::null_mut();
    check_status(napi_unwrap(env, this, &mut ptr), "napi_unwrap");
    &*(ptr as *const T)
}

pub unsafe fn unwrap_data_mut<T>(env: napi_env, this: napi_value) -> &'static mut T {
    let mut ptr: *mut c_void = ptr::null_mut();
    check_status(napi_unwrap(env, this, &mut ptr), "napi_unwrap");
    &mut *(ptr as *mut T)
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
    unsafe { napi_throw_error(env, ptr::null(), c_msg.as_ptr()); }
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
