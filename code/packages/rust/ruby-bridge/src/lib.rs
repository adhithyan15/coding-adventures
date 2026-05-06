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
use std::slice;

// ---------------------------------------------------------------------------
// The VALUE type
// ---------------------------------------------------------------------------

/// Ruby's universal value type — a machine word that is either a tagged
/// immediate or a pointer to a heap-allocated object.
pub type VALUE = usize;

/// Ruby's ID type — used for interned symbols (method names, variable names).
/// Same size as VALUE (machine word) on all architectures.
pub type ID = usize;

// ---------------------------------------------------------------------------
// Well-known VALUE constants
// ---------------------------------------------------------------------------
//
// These are the bit patterns Ruby uses for special values. They're fixed
// across all Ruby versions on the same architecture.

/// Ruby `false` (VALUE = 0)
pub const QFALSE: VALUE = 0;
/// Ruby `true` — 0x14 on all 64-bit Ruby builds with USE_FLONUM (the default).
pub const QTRUE: VALUE = 0x14;
/// Ruby `nil` — 0x04 on all 64-bit Ruby builds with USE_FLONUM (the default).
///
/// USE_FLONUM is enabled on every 64-bit Ruby (x86_64 and aarch64) since
/// Ruby 2.x. The special-constant layout with USE_FLONUM is:
///   Qfalse = 0x00, Qnil = 0x04, Qtrue = 0x14, Qundef = 0x24
/// Without USE_FLONUM (32-bit or unusual builds) Qnil = 0x02, but those
/// builds are not supported by this crate.
pub const QNIL: VALUE = 0x04;

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
    // rb_define_module_function defines a function callable both as a module
    // method and as a private instance method. This is the Ruby idiom for
    // module-level utility functions (e.g. `Math.sqrt` / `include Math; sqrt`).
    pub fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: *const c_void,
        argc: c_int,
    );
    // rb_path2class looks up a class or module by its fully-qualified Ruby
    // constant path (e.g. "Encoding::UndefinedConversionError").
    pub fn rb_path2class(path: *const c_char) -> VALUE;

    // -- Allocator binding -------------------------------------------------
    //
    // rb_define_alloc_func tells Ruby how to allocate instances of a class.
    // The alloc function receives the class and must return a new VALUE.
    // This is called *before* `initialize` — it creates the raw object,
    // then `initialize` fills it in.
    pub fn rb_define_alloc_func(
        klass: VALUE,
        func: unsafe extern "C" fn(VALUE) -> VALUE,
    );

    // -- Well-known classes ------------------------------------------------
    pub static rb_cObject: VALUE;
    pub static rb_eStandardError: VALUE;
    pub static rb_eArgError: VALUE;
    pub static rb_eRuntimeError: VALUE;

    // -- String operations -------------------------------------------------
    pub fn rb_str_new(ptr: *const c_char, len: c_long) -> VALUE;
    pub fn rb_utf8_str_new(ptr: *const c_char, len: c_long) -> VALUE;
    fn rb_string_value_ptr(ptr: *mut VALUE) -> *const c_char;
    fn rb_string_value_cstr(ptr: *mut VALUE) -> *const c_char;

    // -- Array operations --------------------------------------------------
    pub fn rb_ary_new() -> VALUE;
    pub fn rb_ary_push(ary: VALUE, item: VALUE) -> VALUE;
    pub fn rb_ary_entry(ary: VALUE, offset: c_long) -> VALUE;

    // -- Symbol interning --------------------------------------------------
    //
    // rb_intern converts a C string to a Ruby ID (interned symbol). IDs are
    // cached after the first call, so repeated calls with the same string are
    // O(1) hash-table lookups. This is how Ruby caches method names.
    pub fn rb_intern(name: *const c_char) -> ID;

    // -- Method dispatch ---------------------------------------------------
    //
    // rb_funcallv is the non-variadic form of rb_funcall. It takes the
    // argument array as a pointer, making it safe to call from Rust FFI.
    // For zero-argument calls, pass argc=0 and argv=null.
    //
    // We prefer rb_funcallv over rb_funcall because rb_funcall is variadic
    // (takes ...) which requires #[no_mangle] + nightly __variadic feature in
    // Rust. rb_funcallv has a fixed signature and works on stable Rust.
    pub fn rb_funcallv(recv: VALUE, mid: ID, argc: c_int, argv: *const VALUE) -> VALUE;

    // rb_num2long converts any Ruby Numeric VALUE to a C long.
    // Used to convert the Fixnum returned by Array#length into a Rust usize.
    pub fn rb_num2long(v: VALUE) -> c_long;

    // -- Numeric operations ------------------------------------------------
    pub fn rb_int2inum(v: c_long) -> VALUE;
    pub fn rb_float_new(v: f64) -> VALUE;
    pub fn rb_num2dbl(v: VALUE) -> f64;

    // -- Hash operations ---------------------------------------------------
    pub fn rb_hash_new() -> VALUE;
    pub fn rb_hash_aset(hash: VALUE, key: VALUE, val: VALUE) -> VALUE;

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

/// Define a module function — callable both as `Module.method(args)` and
/// as a free function when the module is included/extended.
///
/// This is the idiomatic way to expose stateless utility functions from
/// a module (analogous to Python `@staticmethod` or Node.js module exports).
///
/// `argc` specifies the number of required arguments. Use `-1` for variadic.
pub fn define_module_function_raw(module: VALUE, name: &str, func: *const c_void, argc: i32) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_module_function(module, c_name.as_ptr(), func, argc as c_int) }
}

/// Look up a Ruby class or module by its fully-qualified constant path.
///
/// For example, `path2class("CodingAdventures::CommonmarkNative")` returns
/// the VALUE for that module. Raises `ArgumentError` in Ruby if the constant
/// does not exist.
pub fn path2class(path: &str) -> VALUE {
    let c_path = CString::new(path).expect("path must not contain NUL");
    unsafe { rb_path2class(c_path.as_ptr()) }
}

/// Bind a singleton (class-level) method.
pub fn define_singleton_method_raw(class: VALUE, name: &str, func: *const c_void, argc: i32) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    unsafe { rb_define_singleton_method(class, c_name.as_ptr(), func, argc as c_int) }
}

/// Set the allocator function for a Ruby class. The alloc function
/// is called before `initialize` and must return a new VALUE (typically
/// created via `wrap_data`).
pub fn define_alloc_func(class: VALUE, func: unsafe extern "C" fn(VALUE) -> VALUE) {
    unsafe { rb_define_alloc_func(class, func) }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Well-known classes
// ---------------------------------------------------------------------------

pub fn object_class() -> VALUE {
    path2class("Object")
}

pub fn standard_error_class() -> VALUE {
    path2class("StandardError")
}

pub fn arg_error_class() -> VALUE {
    path2class("ArgumentError")
}

pub fn runtime_error_class() -> VALUE {
    path2class("RuntimeError")
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

/// Convert a Rust byte slice to a Ruby String without interpreting it as
/// UTF-8. This is the right helper for compact binary protocols, COBS frames,
/// checksummed payloads, and buffers that may contain NUL bytes.
pub fn bytes_to_rb(bytes: &[u8]) -> VALUE {
    unsafe { rb_str_new(bytes.as_ptr() as *const c_char, bytes.len() as c_long) }
}

/// Convert a Ruby String VALUE to an owned Rust byte buffer.
///
/// Unlike `str_from_rb`, this preserves embedded NUL bytes and invalid UTF-8.
pub fn bytes_from_rb(val: VALUE) -> Option<Vec<u8>> {
    unsafe {
        let mut v = val;
        let ptr = rb_string_value_ptr(&mut v);
        if ptr.is_null() {
            return None;
        }

        let mid = rb_intern(b"bytesize\0".as_ptr() as *const c_char);
        let len_val = rb_funcallv(v, mid, 0, std::ptr::null());
        let len = rb_num2long(len_val);
        if len < 0 {
            return None;
        }

        let ptr = rb_string_value_ptr(&mut v);
        if ptr.is_null() {
            return None;
        }

        Some(slice::from_raw_parts(ptr as *const u8, len as usize).to_vec())
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
///
/// We cannot use `rb_array_len` (static inline in Ruby 3.x headers, not
/// exported) or `rb_ary_length` (defined as a static function in array.c,
/// also not exported in Ruby 3.4). Instead we call the Ruby `length` method
/// via `rb_funcallv`, which is always exported and works on all Ruby versions.
pub fn array_len(array: VALUE) -> usize {
    unsafe {
        // rb_intern caches the ID after the first call — no performance concern.
        let mid = rb_intern(b"length\0".as_ptr() as *const c_char);
        // rb_funcallv with argc=0 and null argv calls array.length with no args.
        let len_val = rb_funcallv(array, mid, 0, std::ptr::null());
        rb_num2long(len_val) as usize
    }
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

pub fn vec_tuple2_str_f64_to_rb(items: &[(String, f64)]) -> VALUE {
    let ary = array_new();
    for (key, value) in items {
        let pair = array_new();
        array_push(pair, str_to_rb(key));
        array_push(pair, f64_to_rb(*value));
        array_push(ary, pair);
    }
    ary
}

pub fn vec_tuple3_str_f64_to_rb(items: &[(String, String, f64)]) -> VALUE {
    let ary = array_new();
    for (left, right, weight) in items {
        let triple = array_new();
        array_push(triple, str_to_rb(left));
        array_push(triple, str_to_rb(right));
        array_push(triple, f64_to_rb(*weight));
        array_push(ary, triple);
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

pub fn f64_to_rb(n: f64) -> VALUE {
    unsafe { rb_float_new(n) }
}

pub fn f64_from_rb(val: VALUE) -> f64 {
    unsafe { rb_num2dbl(val) }
}

pub fn hash_new() -> VALUE {
    unsafe { rb_hash_new() }
}

pub fn hash_aset(hash: VALUE, key: VALUE, val: VALUE) {
    unsafe {
        rb_hash_aset(hash, key, val);
    }
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
/// # How it works
///
/// Ruby's `rb_data_object_wrap` stores a void pointer inside an `RData`
/// struct.  That struct has a well-known, ABI-stable layout:
///
/// ```text
/// struct RData {
///     struct RBasic basic;   // 2 machine words: flags + klass
///     void (*dmark)(void*);  // GC mark function pointer
///     void (*dfree)(void*);  // GC free function pointer
///     void *data;            // <--- our Rust pointer lives here
/// };
/// ```
///
/// On a 64-bit system each field is 8 bytes, so `data` sits at byte
/// offset `4 * 8 = 32`.  On 32-bit it would be `4 * 4 = 16`.  In both
/// cases: **4 machine words from the start of the object**.
///
/// The C macro `DATA_PTR(obj)` does exactly this pointer arithmetic.
/// We replicate it here so we don't need rb-sys or bindgen.
///
/// # Safety
///
/// The VALUE must have been created by `wrap_data<T>` with the same
/// type `T`.  Passing a different type or a non-data VALUE is undefined
/// behavior.
pub unsafe fn unwrap_data<T>(obj: VALUE) -> &'static T {
    // The data pointer is at offset 4 words into the RData struct:
    //   word 0: flags  (RBasic.flags)
    //   word 1: klass  (RBasic.klass)
    //   word 2: dmark  (function pointer)
    //   word 3: dfree  (function pointer)
    //   word 4: data   (void*)  <--- we want this
    let rdata_ptr = obj as *const usize;
    let data_ptr = *rdata_ptr.add(4) as *const T;
    &*data_ptr
}

/// Extract a mutable reference to the Rust data inside a Ruby object.
///
/// Same layout as [`unwrap_data`] but returns `&mut T`.
///
/// # Safety
///
/// Same requirements as `unwrap_data`, plus the caller must ensure no
/// other references to the same data exist simultaneously.
pub unsafe fn unwrap_data_mut<T>(obj: VALUE) -> &'static mut T {
    let rdata_ptr = obj as *const usize;
    let data_ptr = *rdata_ptr.add(4) as *mut T;
    &mut *data_ptr
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
