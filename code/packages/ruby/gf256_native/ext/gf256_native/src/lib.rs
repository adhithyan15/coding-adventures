// lib.rs -- GF(2^8) Ruby native extension using ruby-bridge
// ==========================================================
//
// This is a Ruby C extension written in Rust. It wraps the `gf256` crate's
// field arithmetic functions and exposes them to Ruby as module-level functions
// under:
//
//   CodingAdventures::GF256Native
//
// # What is GF(2^8)?
//
// GF(2^8) — "Galois Field of 256 elements" — is the finite field used by:
// - Reed-Solomon error correction (QR codes, CDs, DVDs, hard drives)
// - AES encryption (SubBytes and MixColumns steps)
//
// Elements are the integers 0..=255. The arithmetic is NOT ordinary integer
// arithmetic — it's polynomial arithmetic over GF(2), reduced modulo an
// irreducible polynomial `p(x) = x^8 + x^4 + x^3 + x^2 + 1` (0x11D).
//
// Key fact: addition = subtraction = XOR (characteristic 2 field).
//
// # Architecture
//
// - No stateful objects: GF(256) values are just u8 integers.
// - We define a Ruby Module (not a class) with module functions.
// - Ruby Integer → u8: validated using `rb_num2long` + range check.
// - Module constants ZERO, ONE, PRIMITIVE_POLYNOMIAL are set on the module.
//
// # Panic Safety
//
// `gf256::divide` and `gf256::inverse` panic on zero inputs.
// We use `std::panic::catch_unwind` to convert these panics into Ruby
// `ArgumentError` exceptions.
//
// # Constants
//
//   CodingAdventures::GF256Native::ZERO                = 0
//   CodingAdventures::GF256Native::ONE                 = 1
//   CodingAdventures::GF256Native::PRIMITIVE_POLYNOMIAL = 285  (0x11D)

use std::ffi::{c_char, c_long, c_void, CString};
use std::panic;

use ruby_bridge::VALUE;

// ---------------------------------------------------------------------------
// Additional Ruby C API functions
// ---------------------------------------------------------------------------
//
// `rb_num2long` converts Ruby Integer (Fixnum or Bignum) → C long.
// `rb_define_const` sets a Ruby constant on a module or class.
// `rb_path2class` looks up a class by name — safer than extern statics on Windows.

extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_define_const(module: VALUE, name: *const c_char, val: VALUE);
    fn rb_path2class(path: *const c_char) -> VALUE;
}

/// Look up a Ruby class by its fully-qualified name.
///
/// Used instead of `rb_eArgError` extern static to avoid Windows MinGW + MSVC
/// linker issues where static data symbols from Ruby don't resolve correctly.
fn get_ruby_class(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    unsafe { rb_path2class(c_name.as_ptr()) }
}

/// Raise an ArgumentError with the given message. Does not return.
fn raise_arg_error(msg: &str) -> ! {
    ruby_bridge::raise_error(get_ruby_class("ArgumentError"), msg)
}

/// Define a Ruby constant on a module.
///
/// `rb_define_const` takes the module VALUE, a C string name, and the
/// value as a Ruby VALUE (an integer here, created via `rb_int2inum`).
fn define_const(module: VALUE, name: &str, value: usize) {
    let c_name = CString::new(name).expect("constant name must not contain NUL");
    let rb_val = ruby_bridge::usize_to_rb(value);
    unsafe { rb_define_const(module, c_name.as_ptr(), rb_val) };
}

// ---------------------------------------------------------------------------
// u8 conversion helper
// ---------------------------------------------------------------------------
//
// GF(256) values are bytes (0..=255). Ruby Integers can be arbitrarily large,
// so we must validate the range explicitly.

/// Convert a Ruby Integer VALUE to a Rust `u8`.
///
/// Uses `rb_num2long` to extract the integer (works for Fixnum and Bignum).
/// Raises `ArgumentError` if the value is outside the valid GF(256) range [0, 255].
///
/// # Why rb_num2long?
///
/// On 64-bit systems, small Ruby integers (Fixnum) are tagged-pointer immediates:
/// `VALUE = (n << 1) | 1`. We cannot simply cast the VALUE to an integer. The
/// `rb_num2long` C API function handles the untagging transparently for both
/// Fixnum and heap-allocated Bignum.
fn u8_from_rb(val: VALUE, arg_name: &str) -> u8 {
    let n = unsafe { rb_num2long(val) };
    if n < 0 || n > 255 {
        raise_arg_error(&format!(
            "{}: expected an integer in range 0..255, got {}",
            arg_name, n
        ));
    }
    n as u8
}

// ---------------------------------------------------------------------------
// Module functions
// ---------------------------------------------------------------------------
//
// Each function is registered with `rb_define_module_function`. The first
// parameter `_module` is the module itself (Ruby's `self` in module context).
// We name it with a leading underscore since we never use it.

// -- add(a, b) -> Integer ---------------------------------------------------
//
// Add two GF(256) elements. In characteristic-2 arithmetic, addition is XOR:
//
//   add(0x53, 0xCA) = 0x53 ^ 0xCA = 0x99
//   add(x, x)       = 0   for all x  (every element is its own additive inverse)
//
//   CodingAdventures::GF256Native.add(0x53, 0xCA)  #=> 153
extern "C" fn gf_add(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = u8_from_rb(a_val, "add(a, b): a");
    let b = u8_from_rb(b_val, "add(a, b): b");
    ruby_bridge::usize_to_rb(gf256::add(a, b) as usize)
}

// -- subtract(a, b) -> Integer -----------------------------------------------
//
// Subtract two GF(256) elements. In characteristic 2, subtraction equals addition:
//
//   subtract(a, b) = a XOR b   (same as add)
//
//   CodingAdventures::GF256Native.subtract(0x53, 0xCA)  #=> 153  (same as add)
extern "C" fn gf_subtract(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = u8_from_rb(a_val, "subtract(a, b): a");
    let b = u8_from_rb(b_val, "subtract(a, b): b");
    ruby_bridge::usize_to_rb(gf256::subtract(a, b) as usize)
}

// -- multiply(a, b) -> Integer -----------------------------------------------
//
// Multiply two GF(256) elements using logarithm/antilogarithm tables.
//
// Mathematical identity: a × b = g^(log(a) + log(b)) where g = 2 is the
// generator. Special case: 0 × anything = 0.
//
//   CodingAdventures::GF256Native.multiply(2, 4)   #=> 8   (simple: 2^1 * 2^2 = 2^3)
//   CodingAdventures::GF256Native.multiply(128, 2)  #=> 29  (overflow, reduce mod 0x11D)
extern "C" fn gf_multiply(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = u8_from_rb(a_val, "multiply(a, b): a");
    let b = u8_from_rb(b_val, "multiply(a, b): b");
    ruby_bridge::usize_to_rb(gf256::multiply(a, b) as usize)
}

// -- divide(a, b) -> Integer -------------------------------------------------
//
// Divide a by b in GF(256): a / b = g^(log(a) - log(b)).
//
// Raises `ArgumentError` if b == 0 (division by zero is undefined in any field).
//
//   CodingAdventures::GF256Native.divide(1, 1)    #=> 1
//   CodingAdventures::GF256Native.divide(0, 5)    #=> 0   (0 divided by anything is 0)
//   CodingAdventures::GF256Native.divide(1, 0)    # raises ArgumentError
extern "C" fn gf_divide(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = u8_from_rb(a_val, "divide(a, b): a");
    let b = u8_from_rb(b_val, "divide(a, b): b");

    // Catch the panic from gf256::divide when b == 0.
    let result = panic::catch_unwind(|| gf256::divide(a, b));
    match result {
        Ok(val) => ruby_bridge::usize_to_rb(val as usize),
        Err(_) => raise_arg_error("divide: divisor must not be zero"),
    }
}

// -- power(base, exp) -> Integer ---------------------------------------------
//
// Raise a GF(256) element to a non-negative integer power.
//
// Uses the log table: base^exp = ALOG[(LOG[base] * exp) % 255].
//
// Special cases:
//   - 0^0 = 1 (by convention)
//   - 0^n = 0 for n > 0
//   - x^0 = 1 for any x
//
// Note: `exp` is a Ruby Integer that must fit in a u32. We use `rb_num2long`
// and validate the range.
//
//   CodingAdventures::GF256Native.power(2, 8)    #=> 29  (2^8 mod 0x11D)
//   CodingAdventures::GF256Native.power(2, 0)    #=> 1
//   CodingAdventures::GF256Native.power(0, 0)    #=> 1
extern "C" fn gf_power(_module: VALUE, base_val: VALUE, exp_val: VALUE) -> VALUE {
    let base = u8_from_rb(base_val, "power(base, exp): base");

    // exp must be a non-negative u32
    let exp_long = unsafe { rb_num2long(exp_val) };
    if exp_long < 0 {
        raise_arg_error("power(base, exp): exp must be non-negative");
    }
    let exp = exp_long as u32;

    ruby_bridge::usize_to_rb(gf256::power(base, exp) as usize)
}

// -- inverse(a) -> Integer ---------------------------------------------------
//
// Compute the multiplicative inverse of a GF(256) element.
//
// The inverse of `a` satisfies: `a * inverse(a) = 1`.
// Computed as: ALOG[255 - LOG[a]]
//
// Raises `ArgumentError` if a == 0 (zero has no multiplicative inverse —
// it is the additive identity, and no element times zero can equal 1).
//
//   CodingAdventures::GF256Native.inverse(1)    #=> 1   (1 is its own inverse)
//   CodingAdventures::GF256Native.inverse(2)    #=> 142 (2 * 142 = 1 in GF(256))
//   CodingAdventures::GF256Native.inverse(0)    # raises ArgumentError
extern "C" fn gf_inverse(_module: VALUE, a_val: VALUE) -> VALUE {
    let a = u8_from_rb(a_val, "inverse(a): a");

    // Catch the panic from gf256::inverse when a == 0.
    let result = panic::catch_unwind(|| gf256::inverse(a));
    match result {
        Ok(val) => ruby_bridge::usize_to_rb(val as usize),
        Err(_) => raise_arg_error("inverse: argument must not be zero (zero has no multiplicative inverse)"),
    }
}

// ---------------------------------------------------------------------------
// Init_gf256_native -- Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_gf256_native` because Ruby derives the
// init function name from the .so filename. When Ruby loads
// `gf256_native.so`, it calls `Init_gf256_native()`.
//
// We set up the module hierarchy, constants, and module functions here:
//
//   module CodingAdventures
//     module GF256Native
//       ZERO                = 0
//       ONE                 = 1
//       PRIMITIVE_POLYNOMIAL = 285  # 0x11D
//
//       def self.add(a, b)      ... end
//       def self.subtract(a, b) ... end
//       def self.multiply(a, b) ... end
//       def self.divide(a, b)   ... end
//       def self.power(b, e)    ... end
//       def self.inverse(a)     ... end
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_gf256_native() {
    // -- Module hierarchy ---------------------------------------------------
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let gf256_native = ruby_bridge::define_module_under(coding_adventures, "GF256Native");

    // -- Constants ----------------------------------------------------------
    //
    // Expose the fundamental GF(256) constants so Ruby callers can reference
    // them by name rather than magic numbers.
    //
    //   CodingAdventures::GF256Native::ZERO                = 0
    //   CodingAdventures::GF256Native::ONE                 = 1
    //   CodingAdventures::GF256Native::PRIMITIVE_POLYNOMIAL = 285
    define_const(gf256_native, "ZERO", gf256::ZERO as usize);
    define_const(gf256_native, "ONE", gf256::ONE as usize);
    define_const(
        gf256_native,
        "PRIMITIVE_POLYNOMIAL",
        gf256::PRIMITIVE_POLYNOMIAL as usize,
    );

    // -- Field operations ---------------------------------------------------
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "add",
        gf_add as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "subtract",
        gf_subtract as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "multiply",
        gf_multiply as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "divide",
        gf_divide as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "power",
        gf_power as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        gf256_native,
        "inverse",
        gf_inverse as *const c_void,
        1,
    );
}
