// lib.rs -- Bitset Node.js native extension using node-bridge
// ============================================================
//
// This crate exposes the Rust `bitset::Bitset` to Node.js via N-API,
// using our zero-dependency `node-bridge` crate. No napi-rs, no napi-sys,
// no build-time header requirements -- just raw N-API calls through
// node-bridge's safe wrappers.
//
// # Architecture
//
// 1. `napi_register_module_v1()` is the entry point called by Node.js when
//    the addon is loaded. It defines a "Bitset" class on the exports
//    object with all bitset methods.
//
// 2. The constructor (`bitset_new`) creates a Rust `Bitset` and wraps it
//    inside the JS object using `node_bridge::wrap_data()`. The constructor
//    supports three modes:
//      - new Bitset(size)               -- creates a zero-filled bitset
//      - new Bitset(value, "integer")   -- creates from an integer
//      - new Bitset(str, "binary")      -- creates from a binary string
//
// 3. Each method callback extracts `this` and args via `get_cb_info()`,
//    unwraps the Bitset pointer, calls the Rust method, marshals the
//    result back to a JS value, and returns it.
//
// 4. Binary operations (and, or, xor, andNot) take another Bitset
//    instance as an argument. We unwrap both `this` and the argument
//    to get two Rust Bitset references, perform the operation, and
//    wrap the result in a new JS Bitset instance.
//
// 5. Errors (like invalid binary strings) are turned into JS exceptions
//    via `throw_error()` + returning `undefined()`.
//
// # Method naming
//
// All methods use camelCase to follow JavaScript conventions:
//   set, clear, test, toggle,
//   and, or, xor, not, andNot,
//   popcount, len, capacity,
//   any, all, none, isEmpty,
//   iterSetBits, toInteger, toBinaryStr

use bitset::Bitset;
use node_bridge::*;

macro_rules! unwrap_ref {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data::<$ty>($env, $value)
            .as_ref()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

macro_rules! unwrap_mut {
    ($env:expr, $value:expr, $ty:ty) => {
        unwrap_data_mut::<$ty>($env, $value)
            .as_mut()
            .expect(concat!(stringify!($ty), " should always be wrapped"))
    };
}

// ---------------------------------------------------------------------------
// Extra N-API externs not in node-bridge
// ---------------------------------------------------------------------------
//
// We need `napi_get_value_int64` to extract numbers from JS, and
// `napi_new_instance` to create new Bitset JS objects from factory-style
// operations (and, or, xor, not, andNot). We also need `napi_typeof` to
// detect argument types in the constructor, and `napi_create_reference` /
// `napi_get_reference_value` to store the constructor for later use.

use std::ffi::c_void;
use std::ptr;

extern "C" {
    fn napi_get_value_int64(
        env: napi_env,
        value: napi_value,
        result: *mut i64,
    ) -> napi_status;

    fn napi_new_instance(
        env: napi_env,
        constructor: napi_value,
        argc: usize,
        argv: *const napi_value,
        result: *mut napi_value,
    ) -> napi_status;

    fn napi_create_reference(
        env: napi_env,
        value: napi_value,
        initial_refcount: u32,
        result: *mut *mut c_void,
    ) -> napi_status;

    fn napi_get_reference_value(
        env: napi_env,
        reference: *mut c_void,
        result: *mut napi_value,
    ) -> napi_status;

    fn napi_get_value_double(
        env: napi_env,
        value: napi_value,
        result: *mut f64,
    ) -> napi_status;
}

// ---------------------------------------------------------------------------
// Global constructor reference
// ---------------------------------------------------------------------------
//
// We store a persistent reference to the Bitset constructor so that binary
// operations (and, or, xor, not, andNot) can create new Bitset instances
// to wrap their results. This is set once during module registration.

static mut CONSTRUCTOR_REF: *mut c_void = ptr::null_mut();

// ---------------------------------------------------------------------------
// Helpers: extract a JS number as usize / i64
// ---------------------------------------------------------------------------

fn i64_from_js(env: napi_env, val: napi_value) -> i64 {
    let mut result: i64 = 0;
    unsafe { napi_get_value_int64(env, val, &mut result) };
    result
}

fn usize_from_js(env: napi_env, val: napi_value) -> usize {
    i64_from_js(env, val) as usize
}

fn f64_from_js(env: napi_env, val: napi_value) -> f64 {
    let mut result: f64 = 0.0;
    unsafe { napi_get_value_double(env, val, &mut result) };
    result
}

// ---------------------------------------------------------------------------
// Helper: create a new JS Bitset instance wrapping a Rust Bitset
// ---------------------------------------------------------------------------
//
// This is used by binary operations (and, or, xor, not, andNot) which
// produce a new Bitset. We call `napi_new_instance` with the saved
// constructor reference, passing a size argument, then replace the
// internal Rust Bitset with the actual result.
//
// The trick: we create `new Bitset(0)` to get a valid JS object with the
// right prototype, then overwrite its wrapped data. But napi_wrap only
// allows wrapping once, so instead we create the instance with the right
// size and then copy data. Actually -- we can't easily replace wrapped
// data. Instead, we'll create a minimal instance and use a different
// approach: we'll pass a special "marker" to the constructor.
//
// Simplest approach: use a two-phase init. The constructor, when called
// with no arguments, creates an empty Bitset(0). Then we unwrap and
// replace its contents via std::mem::replace.

fn wrap_new_bitset(env: napi_env, bs: Bitset) -> napi_value {
    unsafe {
        // Get the constructor from the stored reference.
        let mut constructor: napi_value = ptr::null_mut();
        napi_get_reference_value(env, CONSTRUCTOR_REF, &mut constructor);

        // Create a new instance: `new Bitset(0)`.
        let zero = usize_to_js(env, 0);
        let mut instance: napi_value = ptr::null_mut();
        napi_new_instance(env, constructor, 1, &zero, &mut instance);

        // Replace the inner Bitset with the one we actually want.
        let inner = unwrap_mut!(env, instance, Bitset);
        let _ = std::mem::replace(inner, bs);

        instance
    }
}

// ---------------------------------------------------------------------------
// Constructor: new Bitset(size) | new Bitset(value, "integer") |
//              new Bitset(str, "binary")
// ---------------------------------------------------------------------------
//
// The constructor supports three calling conventions:
//
//   new Bitset(100)              -- creates a 100-bit zero-filled bitset
//   new Bitset(42, "integer")    -- creates a bitset from the integer 42
//   new Bitset("1010", "binary") -- creates a bitset from binary string
//
// We detect the mode by checking argument count and types.

unsafe extern "C" fn bitset_new(env: napi_env, info: napi_callback_info) -> napi_value {
    // Get up to 2 arguments.
    let (this, args) = get_cb_info(env, info, 2);

    let bs = if args.is_empty() {
        // No arguments: create empty bitset.
        Bitset::new(0)
    } else if args.len() == 1 {
        // Single argument: must be a number (size).
        let size = usize_from_js(env, args[0]);
        Bitset::new(size)
    } else {
        // Two arguments: first is value, second is mode string.
        let mode = match str_from_js(env, args[1]) {
            Some(s) => s,
            None => {
                throw_error(env, "second argument must be a mode string: \"integer\" or \"binary\"");
                return undefined(env);
            }
        };

        match mode.as_str() {
            "integer" => {
                // The value might be larger than i64 can hold, but for
                // practical JS usage, numbers are f64 (max safe int 2^53).
                // We use f64 to get the full range, then cast to u128.
                let val = f64_from_js(env, args[0]) as u128;
                Bitset::from_integer(val)
            }
            "binary" => {
                let s = match str_from_js(env, args[0]) {
                    Some(s) => s,
                    None => {
                        throw_error(env, "fromBinaryStr requires a string argument");
                        return undefined(env);
                    }
                };
                match Bitset::from_binary_str(&s) {
                    Ok(bs) => bs,
                    Err(e) => {
                        throw_error(env, &e.to_string());
                        return undefined(env);
                    }
                }
            }
            _ => {
                throw_error(env, &format!("unknown mode: {:?}. Use \"integer\" or \"binary\"", mode));
                return undefined(env);
            }
        }
    };

    wrap_data(env, this, bs);
    this
}

// ---------------------------------------------------------------------------
// Single-bit operations
// ---------------------------------------------------------------------------

/// bitset.set(i) -- set bit i to 1 (auto-grows if needed)
unsafe extern "C" fn bitset_set(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "set requires a number argument");
        return undefined(env);
    }
    let i = usize_from_js(env, args[0]);
    let bs = unwrap_mut!(env, this, Bitset);
    bs.set(i);
    undefined(env)
}

/// bitset.clear(i) -- set bit i to 0 (no-op if out of range)
unsafe extern "C" fn bitset_clear(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "clear requires a number argument");
        return undefined(env);
    }
    let i = usize_from_js(env, args[0]);
    let bs = unwrap_mut!(env, this, Bitset);
    bs.clear(i);
    undefined(env)
}

/// bitset.test(i) -- returns true if bit i is set
unsafe extern "C" fn bitset_test(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "test requires a number argument");
        return undefined(env);
    }
    let i = usize_from_js(env, args[0]);
    let bs = unwrap_ref!(env, this, Bitset);
    bool_to_js(env, bs.test(i))
}

/// bitset.toggle(i) -- flip bit i (auto-grows if needed)
unsafe extern "C" fn bitset_toggle(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "toggle requires a number argument");
        return undefined(env);
    }
    let i = usize_from_js(env, args[0]);
    let bs = unwrap_mut!(env, this, Bitset);
    bs.toggle(i);
    undefined(env)
}

// ---------------------------------------------------------------------------
// Bulk bitwise operations
// ---------------------------------------------------------------------------
//
// These take another Bitset instance as an argument and return a new Bitset.
// We unwrap both `this` and the argument to get Rust references, perform
// the operation, and wrap the result in a new JS Bitset instance.

/// bitset.and(other) -- returns a new bitset = this AND other
unsafe extern "C" fn bitset_and(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "and requires a Bitset argument");
        return undefined(env);
    }
    let a = unwrap_ref!(env, this, Bitset);
    let b = unwrap_ref!(env, args[0], Bitset);
    wrap_new_bitset(env, a.and(b))
}

/// bitset.or(other) -- returns a new bitset = this OR other
unsafe extern "C" fn bitset_or(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "or requires a Bitset argument");
        return undefined(env);
    }
    let a = unwrap_ref!(env, this, Bitset);
    let b = unwrap_ref!(env, args[0], Bitset);
    wrap_new_bitset(env, a.or(b))
}

/// bitset.xor(other) -- returns a new bitset = this XOR other
unsafe extern "C" fn bitset_xor(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "xor requires a Bitset argument");
        return undefined(env);
    }
    let a = unwrap_ref!(env, this, Bitset);
    let b = unwrap_ref!(env, args[0], Bitset);
    wrap_new_bitset(env, a.xor(b))
}

/// bitset.not() -- returns a new bitset with all bits flipped
unsafe extern "C" fn bitset_not(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let a = unwrap_ref!(env, this, Bitset);
    wrap_new_bitset(env, a.not())
}

/// bitset.andNot(other) -- returns a new bitset = this AND (NOT other)
unsafe extern "C" fn bitset_and_not(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "andNot requires a Bitset argument");
        return undefined(env);
    }
    let a = unwrap_ref!(env, this, Bitset);
    let b = unwrap_ref!(env, args[0], Bitset);
    wrap_new_bitset(env, a.and_not(b))
}

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

/// bitset.popcount() -- returns the number of set bits
unsafe extern "C" fn bitset_popcount(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    usize_to_js(env, bs.popcount())
}

/// bitset.len() -- returns the logical length (number of addressable bits)
unsafe extern "C" fn bitset_len(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    usize_to_js(env, bs.len())
}

/// bitset.capacity() -- returns the allocated capacity in bits
unsafe extern "C" fn bitset_capacity(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    usize_to_js(env, bs.capacity())
}

/// bitset.any() -- returns true if at least one bit is set
unsafe extern "C" fn bitset_any(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    bool_to_js(env, bs.any())
}

/// bitset.all() -- returns true if all bits are set
unsafe extern "C" fn bitset_all(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    bool_to_js(env, bs.all())
}

/// bitset.none() -- returns true if no bits are set
unsafe extern "C" fn bitset_none(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    bool_to_js(env, bs.none())
}

/// bitset.isEmpty() -- returns true if len is 0
unsafe extern "C" fn bitset_is_empty(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    bool_to_js(env, bs.is_empty())
}

// ---------------------------------------------------------------------------
// Iteration and conversion
// ---------------------------------------------------------------------------

/// bitset.iterSetBits() -- returns an array of indices where bits are set
///
/// In JS, we return a plain array instead of an iterator, since N-API
/// doesn't have a convenient iterator protocol. The array contains the
/// indices in ascending order.
unsafe extern "C" fn bitset_iter_set_bits(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);

    let indices: Vec<usize> = bs.iter_set_bits().collect();
    let arr = array_new(env);
    for (i, &idx) in indices.iter().enumerate() {
        array_set(env, arr, i as u32, usize_to_js(env, idx));
    }
    arr
}

/// bitset.toInteger() -- returns the bitset as a number, or null if too large
///
/// Returns null if the bitset has set bits beyond position 63 (doesn't fit
/// in a JS safe integer). Returns 0 for an empty bitset.
unsafe extern "C" fn bitset_to_integer(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    match bs.to_integer() {
        Some(val) => usize_to_js(env, val as usize),
        None => null(env),
    }
}

/// bitset.toBinaryStr() -- returns the bitset as a binary string ("1010")
unsafe extern "C" fn bitset_to_binary_str(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let bs = unwrap_ref!(env, this, Bitset);
    str_to_js(env, &bs.to_binary_str())
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We define a Bitset class with all its methods and attach it to the
// exports object.

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Define all instance methods using node-bridge's method_property helper.
    let properties = [
        // -- Single-bit operations --
        method_property("set", Some(bitset_set)),
        method_property("clear", Some(bitset_clear)),
        method_property("test", Some(bitset_test)),
        method_property("toggle", Some(bitset_toggle)),
        // -- Bulk bitwise operations --
        method_property("and", Some(bitset_and)),
        method_property("or", Some(bitset_or)),
        method_property("xor", Some(bitset_xor)),
        method_property("not", Some(bitset_not)),
        method_property("andNot", Some(bitset_and_not)),
        // -- Query operations --
        method_property("popcount", Some(bitset_popcount)),
        method_property("len", Some(bitset_len)),
        method_property("capacity", Some(bitset_capacity)),
        method_property("any", Some(bitset_any)),
        method_property("all", Some(bitset_all)),
        method_property("none", Some(bitset_none)),
        method_property("isEmpty", Some(bitset_is_empty)),
        // -- Iteration and conversion --
        method_property("iterSetBits", Some(bitset_iter_set_bits)),
        method_property("toInteger", Some(bitset_to_integer)),
        method_property("toBinaryStr", Some(bitset_to_binary_str)),
    ];

    // Create the class with constructor and all methods.
    let class = define_class(env, "Bitset", Some(bitset_new), &properties);

    // Store a persistent reference to the constructor so binary operations
    // can create new instances (see wrap_new_bitset).
    napi_create_reference(env, class, 1, &raw mut CONSTRUCTOR_REF);

    // Attach the class constructor to exports.
    set_named_property(env, exports, "Bitset", class);

    exports
}
