//! # ruby-bridge — Thin safe wrapper over Ruby's C API
//!
//! This crate replaces Magnus (~15,000 lines) with ~350 lines of explicit,
//! debuggable code. It wraps only the raw C API functions from `rb-sys`
//! that are needed to build Ruby native extensions.
//!
//! ## What Ruby's C API looks like
//!
//! Ruby represents every value as a `VALUE` — a machine-word-sized integer
//! that is either a tagged immediate (small integers, true, false, nil,
//! symbols) or a pointer to a heap-allocated object. The C API provides
//! functions to create modules, define classes, bind methods, convert
//! types, and raise exceptions.
//!
//! ## How this crate works
//!
//! Each public function in this crate wraps one or more unsafe C API calls
//! in a safe Rust interface. The pattern is always:
//!
//! 1. Convert Rust types → C types (e.g., `&str` → `*const c_char`)
//! 2. Call the unsafe C function
//! 3. Convert the result back to Rust types
//!
//! ## Example: Building a native extension
//!
//! ```rust,ignore
//! use ruby_bridge::*;
//!
//! #[no_mangle]
//! pub extern "C" fn Init_my_extension() {
//!     let module = define_module("MyModule");
//!     let klass = define_class_under(module, "MyClass", object_class());
//!     define_method(klass, "hello", my_hello_method, 0);
//! }
//! ```

use std::ffi::{c_char, c_int, c_long, CStr, CString};
use std::os::raw::c_void;

// Re-export VALUE so consumers don't need to depend on rb-sys directly.
pub use rb_sys::VALUE;

// ---------------------------------------------------------------------------
// Module and class definition
// ---------------------------------------------------------------------------

/// Define a top-level Ruby module (e.g., `CodingAdventures`).
///
/// Equivalent to `module CodingAdventures; end` in Ruby.
pub fn define_module(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("module name must not contain NUL");
    unsafe { rb_sys::rb_define_module(c_name.as_ptr()) }
}

/// Define a module nested under a parent module.
///
/// Equivalent to `module Parent::Child; end` in Ruby.
pub fn define_module_under(parent: VALUE, name: &str) -> VALUE {
    let c_name = CString::new(name).expect("module name must not contain NUL");
    unsafe { rb_sys::rb_define_module_under(parent, c_name.as_ptr()) }
}

/// Define a class nested under a module, inheriting from `superclass`.
///
/// Use `object_class()` as the superclass for a basic class.
/// Use `standard_error_class()` for exception classes.
pub fn define_class_under(parent: VALUE, name: &str, superclass: VALUE) -> VALUE {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    unsafe { rb_sys::rb_define_class_under(parent, c_name.as_ptr(), superclass) }
}

/// Bind an instance method on a class.
///
/// `argc` is the number of arguments the method takes (not counting `self`).
/// The function signature must be `extern "C" fn(VALUE, ...) -> VALUE` with
/// exactly `argc` VALUE parameters after `self`.
///
/// For variable arguments, use `argc = -1` and accept `(c_int, *const VALUE, VALUE)`.
pub fn define_method(
    class: VALUE,
    name: &str,
    func: unsafe extern "C" fn() -> VALUE,
    argc: i32,
) {
    let c_name = CString::new(name).expect("method name must not contain NUL");
    unsafe {
        rb_sys::rb_define_method(
            class,
            c_name.as_ptr(),
            Some(std::mem::transmute::<
                unsafe extern "C" fn() -> VALUE,
                unsafe extern "C" fn() -> VALUE,
            >(func)),
            argc as c_int,
        );
    }
}

/// Bind a singleton (class-level) method.
pub fn define_singleton_method(
    class: VALUE,
    name: &str,
    func: unsafe extern "C" fn() -> VALUE,
    argc: i32,
) {
    let c_name = CString::new(name).expect("method name must not contain NUL");
    unsafe {
        rb_sys::rb_define_singleton_method(
            class,
            c_name.as_ptr(),
            Some(std::mem::transmute::<
                unsafe extern "C" fn() -> VALUE,
                unsafe extern "C" fn() -> VALUE,
            >(func)),
            argc as c_int,
        );
    }
}

// ---------------------------------------------------------------------------
// Well-known classes and constants
// ---------------------------------------------------------------------------

/// Ruby's `Object` class — the root of the class hierarchy.
pub fn object_class() -> VALUE {
    unsafe { rb_sys::rb_cObject }
}

/// Ruby's `StandardError` class — base for custom exception classes.
pub fn standard_error_class() -> VALUE {
    unsafe { rb_sys::rb_eStandardError }
}

/// Ruby's `ArgumentError` class.
pub fn arg_error_class() -> VALUE {
    unsafe { rb_sys::rb_eArgError }
}

/// Ruby's `RuntimeError` class.
pub fn runtime_error_class() -> VALUE {
    unsafe { rb_sys::rb_eRuntimeError }
}

/// Ruby `true`.
pub fn qtrue() -> VALUE {
    unsafe { rb_sys::RUBY_Qtrue as VALUE }
}

/// Ruby `false`.
pub fn qfalse() -> VALUE {
    unsafe { rb_sys::RUBY_Qfalse as VALUE }
}

/// Ruby `nil`.
pub fn qnil() -> VALUE {
    unsafe { rb_sys::RUBY_Qnil as VALUE }
}

// ---------------------------------------------------------------------------
// String conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` to a Ruby String VALUE.
pub fn str_to_rb(s: &str) -> VALUE {
    unsafe {
        rb_sys::rb_utf8_str_new(s.as_ptr() as *const c_char, s.len() as c_long)
    }
}

/// Convert a Ruby String VALUE to a Rust `String`.
///
/// Returns `None` if the VALUE is not a string or contains invalid UTF-8.
pub fn str_from_rb(val: VALUE) -> Option<String> {
    unsafe {
        // RSTRING_PTR and RSTRING_LEN are the fastest way to access string data.
        let ptr = rb_sys::RSTRING_PTR(val);
        let len = rb_sys::RSTRING_LEN(val);
        if ptr.is_null() || len < 0 {
            return None;
        }
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        String::from_utf8(slice.to_vec()).ok()
    }
}

// ---------------------------------------------------------------------------
// Array conversion
// ---------------------------------------------------------------------------

/// Create an empty Ruby Array.
pub fn array_new() -> VALUE {
    unsafe { rb_sys::rb_ary_new() }
}

/// Push a VALUE onto a Ruby Array.
pub fn array_push(array: VALUE, item: VALUE) {
    unsafe {
        rb_sys::rb_ary_push(array, item);
    }
}

/// Get the length of a Ruby Array.
pub fn array_len(array: VALUE) -> usize {
    unsafe { rb_sys::RARRAY_LEN(array) as usize }
}

/// Get an element from a Ruby Array by index.
pub fn array_entry(array: VALUE, index: usize) -> VALUE {
    unsafe { rb_sys::rb_ary_entry(array, index as c_long) }
}

/// Convert a `Vec<String>` to a Ruby Array of Strings.
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

/// Convert a `Vec<Vec<String>>` to a Ruby Array of Arrays of Strings.
pub fn vec_vec_str_to_rb(items: &[Vec<String>]) -> VALUE {
    let ary = array_new();
    for group in items {
        array_push(ary, vec_str_to_rb(group));
    }
    ary
}

/// Convert a `Vec<(String, String)>` to a Ruby Array of [from, to] Arrays.
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
// Boolean conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `bool` to a Ruby VALUE (Qtrue/Qfalse).
pub fn bool_to_rb(b: bool) -> VALUE {
    if b { qtrue() } else { qfalse() }
}

// ---------------------------------------------------------------------------
// Integer conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `usize` to a Ruby Integer VALUE.
pub fn usize_to_rb(n: usize) -> VALUE {
    unsafe { rb_sys::rb_int2inum(n as c_long) }
}

// ---------------------------------------------------------------------------
// Data wrapping — store a Rust struct inside a Ruby object
// ---------------------------------------------------------------------------
//
// Ruby's TypedData API lets us embed a pointer to a Rust struct inside
// a Ruby object. When the Ruby GC frees the object, it calls our `dfree`
// function, which drops the Rust struct.
//
// The lifecycle:
// 1. Box::new(data) → Box::into_raw(box) → raw pointer
// 2. TypedData_Wrap_Struct wraps the pointer in a Ruby object
// 3. TypedData_Get_Struct extracts the pointer back
// 4. On GC, dfree is called → Box::from_raw → drop

/// Wrap a Rust value inside a Ruby object of the given class.
///
/// The data is heap-allocated (Box) and the Ruby GC will call `drop`
/// when the object is collected.
pub fn wrap_data<T>(class: VALUE, data: T) -> VALUE {
    let boxed = Box::into_raw(Box::new(data));
    unsafe {
        let obj = rb_sys::rb_data_object_wrap(
            class,
            boxed as *mut c_void,
            None, // mark function (not needed — we don't hold Ruby references)
            Some(free_data::<T>),
        );
        obj
    }
}

/// Extract a reference to the Rust data stored inside a Ruby object.
///
/// # Safety
/// The caller must ensure the VALUE was created by `wrap_data<T>` with
/// the same type T.
pub unsafe fn unwrap_data<T>(obj: VALUE) -> &'static T {
    let ptr = rb_sys::rb_data_object_get(obj) as *const T;
    &*ptr
}

/// Extract a mutable reference to the Rust data stored inside a Ruby object.
///
/// # Safety
/// The caller must ensure the VALUE was created by `wrap_data<T>` with
/// the same type T, and that no other references exist.
pub unsafe fn unwrap_data_mut<T>(obj: VALUE) -> &'static mut T {
    let ptr = rb_sys::rb_data_object_get(obj) as *mut T;
    &mut *ptr
}

/// Free function called by Ruby's GC when the object is collected.
/// Reconstructs the Box and drops it, running T's destructor.
unsafe extern "C" fn free_data<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr as *mut T);
    }
}

// ---------------------------------------------------------------------------
// Exception handling
// ---------------------------------------------------------------------------

/// Raise a Ruby exception and abort the current method.
///
/// This function does NOT return — Ruby uses longjmp to unwind the stack.
/// The `!` return type makes this explicit to the Rust compiler.
pub fn raise(exc_class: VALUE, msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    unsafe {
        rb_sys::rb_raise(exc_class, c_msg.as_ptr());
    }
    // rb_raise never returns, but the compiler doesn't know that.
    unreachable!()
}

/// Raise a RuntimeError.
pub fn raise_runtime_error(msg: &str) -> ! {
    raise(runtime_error_class(), msg)
}

/// Raise an ArgumentError.
pub fn raise_arg_error(msg: &str) -> ! {
    raise(arg_error_class(), msg)
}
