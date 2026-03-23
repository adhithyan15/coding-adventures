// lib.rs -- Bitset Ruby native extension using ruby-bridge
// ========================================================
//
// This is a Ruby C extension written in Rust. It wraps the `bitset` crate's
// `Bitset` struct and exposes it to Ruby as:
//
//   CodingAdventures::BitsetNative::Bitset
//
// # Architecture
//
// 1. `Init_bitset_native()` is called by Ruby when the .so is loaded
// 2. We define the module hierarchy and a `Bitset` class under it
// 3. Each Bitset instance wraps a Rust `bitset::Bitset` via `wrap_data`
// 4. Methods extract the Bitset pointer via `unwrap_data`, call Rust, marshal results
//
// # The ruby-bridge approach
//
// Instead of Magnus or rb-sys, we use our own `ruby-bridge` crate that
// declares Ruby's C API functions via `extern "C"`. This gives us:
// - Zero dependencies beyond libruby (linked at load time)
// - Complete visibility into every C API call
// - No build-time header requirements
//
// # Method signatures
//
// Ruby's `rb_define_method` uses a C function pointer + argc count:
// - argc=0: `extern "C" fn(self_val: VALUE) -> VALUE`
// - argc=1: `extern "C" fn(self_val: VALUE, arg: VALUE) -> VALUE`
// - argc=2: `extern "C" fn(self_val: VALUE, arg1: VALUE, arg2: VALUE) -> VALUE`
//
// The `self_val` is always the Ruby receiver (the Bitset object).
//
// # Integer extraction
//
// Ruby integers (Fixnum) use a tagged-pointer representation. On 64-bit
// systems, a Fixnum VALUE stores the integer shifted left by 1 with the
// low bit set: `VALUE = (n << 1) | 1`. We use `rb_num2long` from
// Ruby's C API to extract the actual integer value reliably, handling
// both Fixnum and Bignum transparently.

use std::ffi::{c_char, c_long, c_void, CString};

use bitset::Bitset;
use ruby_bridge::VALUE;

// ---------------------------------------------------------------------------
// Additional Ruby C API functions not in ruby-bridge
// ---------------------------------------------------------------------------
//
// We need rb_num2long to convert Ruby Integer -> Rust integer, and
// rb_path2class to look up Ruby classes by name. Both are stable parts
// of Ruby's C API but not yet wrapped in our ruby-bridge crate.
//
// We use rb_path2class instead of the extern statics (rb_cObject,
// rb_eStandardError) because on Windows with MinGW-built Ruby +
// MSVC Rust toolchain, the static data symbols don't resolve correctly
// through the import library. Function calls always work.

extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_path2class(path: *const c_char) -> VALUE;
}

/// Look up a Ruby class by its fully-qualified name.
///
/// This is a safe wrapper around `rb_path2class`. We use this instead of
/// the extern statics (`rb_cObject`, `rb_eStandardError`) because the
/// statics have linking issues on Windows when using MinGW Ruby with
/// MSVC's linker. Function-based lookups always work correctly.
fn get_ruby_class(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    unsafe { rb_path2class(c_name.as_ptr()) }
}

/// Safely convert a Ruby Integer VALUE to a Rust usize.
///
/// Uses rb_num2long which handles both Fixnum (tagged immediates) and
/// Bignum (heap-allocated large integers). Raises ArgumentError if the
/// value is negative.
fn usize_from_rb(val: VALUE) -> usize {
    let n = unsafe { rb_num2long(val) };
    if n < 0 {
        raise_arg_error("expected a non-negative integer");
    }
    n as usize
}

/// Raise an ArgumentError using function-based class lookup.
///
/// On Windows, `ruby_bridge::raise_arg_error` fails because it reads
/// the `rb_eArgError` extern static, which doesn't resolve correctly
/// with MinGW Ruby + MSVC linker. We use `rb_path2class("ArgumentError")`
/// instead, which always works.
fn raise_arg_error(msg: &str) -> ! {
    ruby_bridge::raise_error(get_ruby_class("ArgumentError"), msg)
}

// ---------------------------------------------------------------------------
// Global: the Bitset class VALUE and error class VALUE
// ---------------------------------------------------------------------------
//
// We store these once during Init and never change them. The alloc function
// needs the class to create instances, and error-raising methods need the
// error class.

static mut BITSET_CLASS: VALUE = 0;
static mut BITSET_ERROR: VALUE = 0;

// ---------------------------------------------------------------------------
// Alloc function -- called by Ruby before `initialize`
// ---------------------------------------------------------------------------
//
// Ruby object creation follows a two-step pattern:
//   1. `allocate` creates the raw object (this function)
//   2. `initialize` fills it in (our `bitset_initialize`)
//
// We wrap a default empty Bitset(0) here. The `initialize` method replaces
// it with the correct size.

unsafe extern "C" fn bitset_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, Bitset::new(0))
}

// ---------------------------------------------------------------------------
// initialize(size) -- Ruby constructor
// ---------------------------------------------------------------------------
//
// Creates a new bitset with `size` bits, all initially zero.
//
//   bs = CodingAdventures::BitsetNative::Bitset.new(100)
//   bs.len  # => 100

extern "C" fn bitset_initialize(self_val: VALUE, size_val: VALUE) -> VALUE {
    let size = usize_from_rb(size_val);
    // Replace the placeholder Bitset(0) from alloc with the real one.
    // We can't truly "replace" the wrapped data after alloc without
    // some trickery, so we use unwrap_data_mut and swap in a new Bitset.
    let bs = unsafe { ruby_bridge::unwrap_data_mut::<Bitset>(self_val) };
    *bs = Bitset::new(size);
    self_val
}

// ---------------------------------------------------------------------------
// Bitset access helpers
// ---------------------------------------------------------------------------
//
// Every method needs to extract the Rust Bitset from the Ruby VALUE.

unsafe fn get_bitset(self_val: VALUE) -> &'static Bitset {
    ruby_bridge::unwrap_data::<Bitset>(self_val)
}

unsafe fn get_bitset_mut(self_val: VALUE) -> &'static mut Bitset {
    ruby_bridge::unwrap_data_mut::<Bitset>(self_val)
}

// ---------------------------------------------------------------------------
// Error handling helper
// ---------------------------------------------------------------------------
//
// Raises a BitsetError (custom Ruby exception class) with the given message.
// Used for errors from the Rust bitset crate (e.g., invalid binary strings).

fn raise_bitset_error(msg: &str) -> ! {
    ruby_bridge::raise_error(unsafe { BITSET_ERROR }, msg)
}

// ---------------------------------------------------------------------------
// Singleton (class) methods -- constructors
// ---------------------------------------------------------------------------

// -- Bitset.from_integer(n) -> Bitset -------------------------------------
//
// Creates a bitset from a non-negative integer. Bit 0 of the bitset is the
// least significant bit of the integer.
//
//   bs = CodingAdventures::BitsetNative::Bitset.from_integer(5)  # binary 101
//   bs.test?(0)  # => true
//   bs.test?(1)  # => false
//   bs.test?(2)  # => true

extern "C" fn bitset_from_integer(_klass: VALUE, int_val: VALUE) -> VALUE {
    let n = usize_from_rb(int_val);
    let bs = Bitset::from_integer(n as u128);
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, bs)
}

// -- Bitset.from_binary_str(s) -> Bitset ----------------------------------
//
// Creates a bitset from a binary string like "1010". The leftmost character
// is the highest-indexed bit (conventional binary notation).
//
//   bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1010")
//   bs.test?(1)  # => true
//   bs.test?(3)  # => true

extern "C" fn bitset_from_binary_str(_klass: VALUE, str_val: VALUE) -> VALUE {
    let s = match ruby_bridge::str_from_rb(str_val) {
        Some(s) => s,
        None => raise_arg_error("from_binary_str: argument must be a String"),
    };
    match Bitset::from_binary_str(&s) {
        Ok(bs) => ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, bs),
        Err(e) => raise_bitset_error(&e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Single-bit operations
// ---------------------------------------------------------------------------

// -- set(i) -> nil --------------------------------------------------------
//
// Sets bit i to 1. Auto-grows if i >= len.
extern "C" fn bitset_set(self_val: VALUE, idx_val: VALUE) -> VALUE {
    let i = usize_from_rb(idx_val);
    unsafe { get_bitset_mut(self_val).set(i) };
    ruby_bridge::QNIL
}

// -- clear(i) -> nil ------------------------------------------------------
//
// Sets bit i to 0. No-op if i >= len (does not grow).
extern "C" fn bitset_clear(self_val: VALUE, idx_val: VALUE) -> VALUE {
    let i = usize_from_rb(idx_val);
    unsafe { get_bitset_mut(self_val).clear(i) };
    ruby_bridge::QNIL
}

// -- test?(i) -> true/false -----------------------------------------------
//
// Returns whether bit i is set. Returns false if i >= len.
extern "C" fn bitset_test(self_val: VALUE, idx_val: VALUE) -> VALUE {
    let i = usize_from_rb(idx_val);
    ruby_bridge::bool_to_rb(unsafe { get_bitset(self_val).test(i) })
}

// -- toggle(i) -> nil -----------------------------------------------------
//
// Flips bit i. Auto-grows if i >= len.
extern "C" fn bitset_toggle(self_val: VALUE, idx_val: VALUE) -> VALUE {
    let i = usize_from_rb(idx_val);
    unsafe { get_bitset_mut(self_val).toggle(i) };
    ruby_bridge::QNIL
}

// ---------------------------------------------------------------------------
// Bulk bitwise operations
// ---------------------------------------------------------------------------
//
// All bulk operations take another Bitset as argument and return a NEW Bitset.
// They don't modify either operand.

// -- and(other) -> Bitset -------------------------------------------------
//
// Bitwise AND (intersection).
extern "C" fn bitset_and(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_bitset(self_val) };
    let b = unsafe { get_bitset(other_val) };
    let result = a.and(b);
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, result)
}

// -- or(other) -> Bitset --------------------------------------------------
//
// Bitwise OR (union).
extern "C" fn bitset_or(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_bitset(self_val) };
    let b = unsafe { get_bitset(other_val) };
    let result = a.or(b);
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, result)
}

// -- xor(other) -> Bitset -------------------------------------------------
//
// Bitwise XOR (symmetric difference).
extern "C" fn bitset_xor(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_bitset(self_val) };
    let b = unsafe { get_bitset(other_val) };
    let result = a.xor(b);
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, result)
}

// -- not -> Bitset --------------------------------------------------------
//
// Bitwise NOT (complement). Flips every bit within len.
extern "C" fn bitset_not(self_val: VALUE) -> VALUE {
    let bs = unsafe { get_bitset(self_val) };
    let result = bs.not();
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, result)
}

// -- and_not(other) -> Bitset ---------------------------------------------
//
// Set difference: bits in self that are NOT in other. Equivalent to self & ~other.
extern "C" fn bitset_and_not(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_bitset(self_val) };
    let b = unsafe { get_bitset(other_val) };
    let result = a.and_not(b);
    ruby_bridge::wrap_data(unsafe { BITSET_CLASS }, result)
}

// ---------------------------------------------------------------------------
// Counting and query operations
// ---------------------------------------------------------------------------

// -- popcount -> Integer --------------------------------------------------
//
// Returns the number of set (1) bits.
extern "C" fn bitset_popcount(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_bitset(self_val).popcount() })
}

// -- len -> Integer -------------------------------------------------------
//
// Returns the logical length (number of addressable bits).
extern "C" fn bitset_len(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_bitset(self_val).len() })
}

// -- capacity -> Integer --------------------------------------------------
//
// Returns the total allocated bits (always a multiple of 64).
extern "C" fn bitset_capacity(self_val: VALUE) -> VALUE {
    ruby_bridge::usize_to_rb(unsafe { get_bitset(self_val).capacity() })
}

// -- any? -> true/false ---------------------------------------------------
//
// Returns true if at least one bit is set.
extern "C" fn bitset_any(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_bitset(self_val).any() })
}

// -- all? -> true/false ---------------------------------------------------
//
// Returns true if ALL bits within len are set.
extern "C" fn bitset_all(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_bitset(self_val).all() })
}

// -- none? -> true/false --------------------------------------------------
//
// Returns true if no bits are set.
extern "C" fn bitset_none(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_bitset(self_val).none() })
}

// -- empty? -> true/false -------------------------------------------------
//
// Returns true if the bitset has zero length.
extern "C" fn bitset_empty(self_val: VALUE) -> VALUE {
    ruby_bridge::bool_to_rb(unsafe { get_bitset(self_val).is_empty() })
}

// ---------------------------------------------------------------------------
// Iteration
// ---------------------------------------------------------------------------

// -- each_set_bit -> Array<Integer> ---------------------------------------
//
// Returns an array of the indices of all set bits in ascending order.
// Uses the efficient trailing-zero-count iteration from the Rust crate.
//
//   bs = CodingAdventures::BitsetNative::Bitset.from_integer(0b10100101)
//   bs.each_set_bit  # => [0, 2, 5, 7]
extern "C" fn bitset_each_set_bit(self_val: VALUE) -> VALUE {
    let bs = unsafe { get_bitset(self_val) };
    let ary = ruby_bridge::array_new();
    for idx in bs.iter_set_bits() {
        ruby_bridge::array_push(ary, ruby_bridge::usize_to_rb(idx));
    }
    ary
}

// ---------------------------------------------------------------------------
// Conversion operations
// ---------------------------------------------------------------------------

// -- to_integer -> Integer ------------------------------------------------
//
// Converts the bitset to an integer, if it fits in a u64. Returns -1
// if the bitset is too large (has set bits beyond position 63).
//
// We return -1 instead of nil because Ruby's QNIL constant has ABI
// compatibility issues on Windows when mixing MinGW Ruby with MSVC-compiled
// Rust extensions (the /FORCE:UNRESOLVED linker flag causes QNIL to
// become an invalid VALUE).
extern "C" fn bitset_to_integer(self_val: VALUE) -> VALUE {
    let bs = unsafe { get_bitset(self_val) };
    match bs.to_integer() {
        Some(n) => ruby_bridge::usize_to_rb(n as usize),
        None => unsafe { ruby_bridge::rb_int2inum(-1) },
    }
}

// -- to_binary_str -> String ----------------------------------------------
//
// Converts the bitset to a binary string with the highest bit on the left.
extern "C" fn bitset_to_binary_str(self_val: VALUE) -> VALUE {
    let bs = unsafe { get_bitset(self_val) };
    ruby_bridge::str_to_rb(&bs.to_binary_str())
}

// -- to_s -> String -------------------------------------------------------
//
// Human-readable representation like "Bitset(101)".
extern "C" fn bitset_to_s(self_val: VALUE) -> VALUE {
    let bs = unsafe { get_bitset(self_val) };
    ruby_bridge::str_to_rb(&bs.to_string())
}

// -- ==(other) -> true/false ----------------------------------------------
//
// Equality comparison. Two bitsets are equal if they have the same len
// and the same bits set.
extern "C" fn bitset_eq(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_bitset(self_val) };
    let b = unsafe { get_bitset(other_val) };
    ruby_bridge::bool_to_rb(a == b)
}

// ---------------------------------------------------------------------------
// Init_bitset_native -- Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_bitset_native` because Ruby derives the
// init function name from the .so filename. When Ruby loads
// `bitset_native.so`, it calls `Init_bitset_native()`.
//
// We set up the module hierarchy and bind all methods here:
//
//   module CodingAdventures
//     module BitsetNative
//       class BitsetError < StandardError; end
//       class Bitset
//         # ... methods ...
//       end
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_bitset_native() {
    // -- Module hierarchy ---------------------------------------------------
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let bitset_native =
        ruby_bridge::define_module_under(coding_adventures, "BitsetNative");

    // -- Error class --------------------------------------------------------
    //
    // Define BitsetError < StandardError for domain-specific errors
    // (e.g., invalid binary strings).
    let error_class = ruby_bridge::define_class_under(
        bitset_native,
        "BitsetError",
        get_ruby_class("StandardError"),
    );
    unsafe { BITSET_ERROR = error_class };

    // -- Bitset class -------------------------------------------------------
    let bitset_class = ruby_bridge::define_class_under(
        bitset_native,
        "Bitset",
        get_ruby_class("Object"),
    );
    unsafe { BITSET_CLASS = bitset_class };

    // -- Allocator ----------------------------------------------------------
    ruby_bridge::define_alloc_func(bitset_class, bitset_alloc);

    // -- initialize(size) ---------------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "initialize",
        bitset_initialize as *const c_void,
        1,
    );

    // -- Singleton (class) methods ------------------------------------------
    //
    // These are factory methods that create new Bitset instances.
    ruby_bridge::define_singleton_method_raw(
        bitset_class,
        "from_integer",
        bitset_from_integer as *const c_void,
        1,
    );
    ruby_bridge::define_singleton_method_raw(
        bitset_class,
        "from_binary_str",
        bitset_from_binary_str as *const c_void,
        1,
    );

    // -- Single-bit operations ----------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "set",
        bitset_set as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "clear",
        bitset_clear as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "test?",
        bitset_test as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "toggle",
        bitset_toggle as *const c_void,
        1,
    );

    // -- Bulk bitwise operations --------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "and",
        bitset_and as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "or",
        bitset_or as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "xor",
        bitset_xor as *const c_void,
        1,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "not",
        bitset_not as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "and_not",
        bitset_and_not as *const c_void,
        1,
    );

    // -- Counting and query operations --------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "popcount",
        bitset_popcount as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "len",
        bitset_len as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "capacity",
        bitset_capacity as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "any?",
        bitset_any as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "all?",
        bitset_all as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "none?",
        bitset_none as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "empty?",
        bitset_empty as *const c_void,
        0,
    );

    // -- Iteration ----------------------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "each_set_bit",
        bitset_each_set_bit as *const c_void,
        0,
    );

    // -- Conversion operations ----------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "to_integer",
        bitset_to_integer as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "to_binary_str",
        bitset_to_binary_str as *const c_void,
        0,
    );
    ruby_bridge::define_method_raw(
        bitset_class,
        "to_s",
        bitset_to_s as *const c_void,
        0,
    );

    // -- Equality -----------------------------------------------------------
    ruby_bridge::define_method_raw(
        bitset_class,
        "==",
        bitset_eq as *const c_void,
        1,
    );
}
