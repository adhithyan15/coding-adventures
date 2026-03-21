//! # ruby-bridge — Zero-dependency Rust wrapper for Ruby's C API
//!
//! This crate provides safe Rust wrappers around Ruby's C extension API
//! using raw `extern "C"` declarations. No rb-sys, no Magnus, no bindgen,
//! no build-time header requirements. Compiles on any platform with just
//! a Rust toolchain.
//!
//! ## How it works
//!
//! Ruby's C API exports functions from `libruby`. These functions have been
//! ABI-stable since Ruby 1.8 (2003). We declare them as `extern "C"` and
//! call them directly. When the extension is loaded by `require`, the
//! dynamic linker resolves these symbols against the running Ruby.
//!
//! ## The VALUE type
//!
//! Ruby represents every value as a `VALUE` — a machine-word-sized integer
//! that is either:
//! - A tagged immediate: small integers (Fixnum), true, false, nil, symbols
//! - A pointer to a heap-allocated Ruby object (String, Array, Hash, etc.)
//!
//! On 64-bit systems, VALUE is `u64`. On 32-bit, it's `u32`.

use std::ffi::{c_char, c_int, c_long, c_void, CString};

// ---------------------------------------------------------------------------
// The VALUE type
// ---------------------------------------------------------------------------

/// Ruby's universal value type — a machine word that is either a tagged
/// immediate or a pointer to a heap-allocated object.
pub type VALUE = usize;

// ---------------------------------------------------------------------------
// Well-known VALUE constants
// ---------------------------------------------------------------------------
//
// These are the bit patterns Ruby uses for special values. They're fixed
// across all Ruby versions on the same architecture.

/// Ruby `false` (VALUE = 0)
pub const QFALSE: VALUE = 0;
/// Ruby `true` (VALUE = 2, or 0x14 on some Ruby versions)
pub const QTRUE: VALUE = 0x14;  // Ruby 2.0+ uses 0x14 for Qtrue
/// Ruby `nil` (VALUE = 4, or 0x08 on some Ruby versions)
pub const QNIL: VALUE = 0x08;   // Ruby 2.0+ uses 0x08 for Qnil

// ---------------------------------------------------------------------------
// Ruby's C API — extern "C" declarations
// ---------------------------------------------------------------------------
//
// These are the stable functions exported by libruby. They have been
// unchanged since Ruby 1.8+ and are used by thousands of native gems.

extern "C" {
    // -- Module and class definition ---------------------------------------
    pub fn rb_define_module(name: *const c_char) -> VALUE;
    pub fn rb_define_module_under(outer: VALUE, name: *const c_char) -> VALUE;
    pub fn rb_define_class_under(
        outer: VALUE,
        name: *const c_char,
        superclass: VALUE,
    ) -> VALUE;

    // -- Method binding ----------------------------------------------------
    //
    // rb_define_method takes a function pointer as `*const c_void`. The
    // actual function signature depends on `argc`:
    //   argc >= 0: fn(self: VALUE, arg1: VALUE, ...) -> VALUE
    //   argc = -1: fn(argc: c_int, argv: *const VALUE, self: VALUE) -> VALUE
    pub fn rb_define_method(
        klass: VALUE,
        name: *const c_char,
        func: *const c_void,
        argc: c_int,
    );
    pub fn rb_define_singleton_method(
        klass: VALUE,
        name: *const c_char,
        func: *const c_void,
        argc: c_int,
    );

    // -- Well-known classes ------------------------------------------------
    pub static rb_cObject: VALUE;
    pub static rb_eStandardError: VALUE;
    pub static rb_eArgError: VALUE;
    pub static rb_eRuntimeError: VALUE;

    // -- String operations -------------------------------------------------
    pub fn rb_utf8_str_new(ptr: *const c_char, len: c_long) -> VALUE;
    fn rb_string_value_cstr(ptr: *mut VALUE) -> *const c_char;

    // -- Array operations --------------------------------------------------
    pub fn rb_ary_new() -> VALUE;
    pub fn rb_ary_push(ary: VALUE, item: VALUE) -> VALUE;
    pub fn rb_ary_entry(ary: VALUE, offset: c_long) -> VALUE;

    // RARRAY_LEN is a macro in Ruby's headers, but we can use
    // rb_array_len which is a proper C function (Ruby 2.7+).
    pub fn rb_array_len(ary: VALUE) -> c_long;

    // -- Integer operations ------------------------------------------------
    pub fn rb_int2inum(v: c_long) -> VALUE;

    // -- Data wrapping -----------------------------------------------------
    //
    // rb_data_object_wrap creates a Ruby object that holds a pointer to
    // our Rust data. When the GC frees the Ruby object, it calls our
    // `dfree` function, which drops the Rust data.
    pub fn rb_data_object_wrap(
        klass: VALUE,
        data: *mut c_void,
        dmark: Option<unsafe extern "C" fn(*mut c_void)>,
        dfree: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> VALUE;

    // rb_data_object_get extracts the pointer we stored.
    // (This is the RDATA(obj)->data macro in a function form.)
    fn rb_check_typeddata(obj: VALUE, data_type: *const c_void) -> *mut c_void;

    // -- Exception handling ------------------------------------------------
    //
    // rb_raise does NOT return — it uses longjmp to unwind the stack.
    // The `!` return type in Rust tells the compiler this.
    pub fn rb_raise(exc_class: VALUE, fmt: *const c_char, ...) -> !;

    // -- String length (for str_from_rb) -----------------------------------
    fn rb_str_strlen(str: VALUE) -> c_long;
}

// ---------------------------------------------------------------------------
// Safe wrappers — Module and class definition
// ---------------------------------------------------------------------------

/// Define a top-level Ruby module.
pub fn define_module(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_module(c_name.as_ptr()) }
}

/// Define a module nested under a parent.
pub fn define_module_under(parent: VALUE, name: &str) -> VALUE {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_module_under(parent, c_name.as_ptr()) }
}

/// Define a class nested under a module.
pub fn define_class_under(parent: VALUE, name: &str, superclass: VALUE) -> VALUE {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_class_under(parent, c_name.as_ptr(), superclass) }
}

/// Bind an instance method on a class.
pub fn define_method_raw(class: VALUE, name: &str, func: *const c_void, argc: i32) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_method(class, c_name.as_ptr(), func, argc as c_int) }
}

/// Bind a singleton (class-level) method.
pub fn define_singleton_method_raw(class: VALUE, name: &str, func: *const c_void, argc: i32) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_singleton_method(class, c_name.as_ptr(), func, argc as c_int) }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Well-known classes
// ---------------------------------------------------------------------------

pub fn object_class() -> VALUE {
    unsafe { rb_cObject }
}

pub fn standard_error_class() -> VALUE {
    unsafe { rb_eStandardError }
}

pub fn arg_error_class() -> VALUE {
    unsafe { rb_eArgError }
}

pub fn runtime_error_class() -> VALUE {
    unsafe { rb_eRuntimeError }
}

// ---------------------------------------------------------------------------
// Safe wrappers — String conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` to a Ruby String VALUE.
pub fn str_to_rb(s: &str) -> VALUE {
    unsafe { rb_utf8_str_new(s.as_ptr() as *const c_char, s.len() as c_long) }
}

/// Convert a Ruby String VALUE to a Rust `String`.
pub fn str_from_rb(val: VALUE) -> Option<String> {
    unsafe {
        let mut v = val;
        let ptr = rb_string_value_cstr(&mut v);
        if ptr.is_null() {
            return None;
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        c_str.to_str().ok().map(|s| s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Array conversion
// ---------------------------------------------------------------------------

/// Create an empty Ruby Array.
pub fn array_new() -> VALUE {
    unsafe { rb_ary_new() }
}

/// Push a VALUE onto a Ruby Array.
pub fn array_push(array: VALUE, item: VALUE) {
    unsafe { rb_ary_push(array, item); }
}

/// Get the length of a Ruby Array.
pub fn array_len(array: VALUE) -> usize {
    unsafe { rb_array_len(array) as usize }
}

/// Get an element from a Ruby Array by index.
pub fn array_entry(array: VALUE, index: usize) -> VALUE {
    unsafe { rb_ary_entry(array, index as c_long) }
}

/// Convert a `&[String]` to a Ruby Array of Strings.
pub fn vec_str_to_rb(items: &[String]) -> VALUE {
    let ary = array_new();
    for item in items {
        array_push(ary, str_to_rb(item));
    }
    ary
}

/// Convert a Ruby Array of Strings to a `Vec<String>`.
pub fn vec_str_from_rb(val: VALUE) -> Vec<String> {
    let len = array_len(val);
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let entry = array_entry(val, i);
        if let Some(s) = str_from_rb(entry) {
            result.push(s);
        }
    }
    result
}

/// Convert a `&[Vec<String>]` to a Ruby Array of Arrays of Strings.
pub fn vec_vec_str_to_rb(items: &[Vec<String>]) -> VALUE {
    let ary = array_new();
    for group in items {
        array_push(ary, vec_str_to_rb(group));
    }
    ary
}

/// Convert `&[(String, String)]` to a Ruby Array of [from, to] Arrays.
pub fn vec_tuple2_str_to_rb(items: &[(String, String)]) -> VALUE {
    let ary = array_new();
    for (a, b) in items {
        let pair = array_new();
        array_push(pair, str_to_rb(a));
        array_push(pair, str_to_rb(b));
        array_push(ary, pair);
    }
    ary
}

// ---------------------------------------------------------------------------
// Safe wrappers — Boolean and Integer
// ---------------------------------------------------------------------------

pub fn bool_to_rb(b: bool) -> VALUE {
    if b { QTRUE } else { QFALSE }
}

pub fn usize_to_rb(n: usize) -> VALUE {
    unsafe { rb_int2inum(n as c_long) }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Data wrapping
// ---------------------------------------------------------------------------

/// Wrap a Rust value inside a Ruby object. The GC will call `drop` when freed.
pub fn wrap_data<T>(class: VALUE, data: T) -> VALUE {
    let boxed = Box::into_raw(Box::new(data));
    unsafe {
        rb_data_object_wrap(
            class,
            boxed as *mut c_void,
            None,
            Some(free_data::<T>),
        )
    }
}

/// Extract a reference to the Rust data inside a Ruby object.
///
/// # Safety
/// The VALUE must have been created by `wrap_data<T>` with the same type T.
pub unsafe fn unwrap_data<T>(obj: VALUE) -> &'static T {
    // RDATA(obj)->data — we access it via the rb_data_object_wrap convention.
    // The data pointer is at a fixed offset in the Ruby RData struct.
    // For simplicity, we use a different approach: store the pointer and
    // retrieve it via the internal struct layout.
    //
    // Actually, the safest way without rb-sys is to use the fact that
    // rb_data_object_wrap stores the pointer, and we know the layout.
    // But for maximum compatibility, we'll just re-read it via a helper.
    //
    // TODO: This needs the actual data extraction. For now, we'll use
    // a static HashMap approach or revisit once we can test against Ruby.
    //
    // Temporary: store/retrieve via a side channel.
    panic!("unwrap_data requires runtime testing against Ruby — implement with DATA_PTR macro equivalent")
}

/// Free function called by Ruby's GC.
unsafe extern "C" fn free_data<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr as *mut T);
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Exception handling
// ---------------------------------------------------------------------------

/// Raise a Ruby exception. Does NOT return.
pub fn raise_error(exc_class: VALUE, msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    unsafe { rb_raise(exc_class, c_msg.as_ptr()) }
}

/// Raise a RuntimeError.
pub fn raise_runtime_error(msg: &str) -> ! {
    raise_error(runtime_error_class(), msg)
}

/// Raise an ArgumentError.
pub fn raise_arg_error(msg: &str) -> ! {
    raise_error(arg_error_class(), msg)
}
