// lib.rs -- ImmutableList Ruby native extension using ruby-bridge
// ================================================================
//
// This is a Ruby C extension written in Rust. It wraps the `immutable-list`
// crate's `ImmutableList` struct and exposes it to Ruby as:
//
//   CodingAdventures::ImmutableListNative::ImmutableList
//
// # Architecture
//
// 1. `Init_immutable_list_native()` is called by Ruby when the .so is loaded
// 2. We define the module hierarchy and an `ImmutableList` class under it
// 3. Each ImmutableList instance wraps a Rust `immutable_list::ImmutableList`
//    via `wrap_data`
// 4. Methods extract the ImmutableList pointer via `unwrap_data`, call Rust,
//    and marshal results back to Ruby
//
// # Immutability Semantics
//
// The key design choice: push, set, and pop do NOT modify the receiver.
// They return **new** Ruby objects wrapping new Rust ImmutableList values.
// This preserves the persistent data structure semantics in Ruby:
//
//   a = ImmutableList.new
//   b = a.push("hello")
//   a.size  # => 0  (unchanged!)
//   b.size  # => 1
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
// The `self_val` is always the Ruby receiver (the ImmutableList object).

use std::ffi::{c_char, c_int, c_long, c_void, CString};

use immutable_list::ImmutableList;
use ruby_bridge::VALUE;

// ---------------------------------------------------------------------------
// Additional Ruby C API functions not in ruby-bridge
// ---------------------------------------------------------------------------
//
// We need:
// - rb_num2long to convert Ruby Integer -> Rust integer
// - rb_path2class to look up Ruby classes by name (avoids extern static ABI
//   issues on Windows with MinGW Ruby + MSVC Rust toolchain)
extern "C" {
    fn rb_num2long(val: VALUE) -> c_long;
    fn rb_path2class(path: *const c_char) -> VALUE;
    fn rb_intern(name: *const c_char) -> VALUE;
    fn rb_funcallv(recv: VALUE, mid: VALUE, argc: c_int, argv: *const VALUE) -> VALUE;
    fn rb_eval_string(str: *const c_char) -> VALUE;
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

/// Extract a Ruby String VALUE to a Rust String, raising ArgumentError
/// if the value is not a valid string.
fn string_from_rb(val: VALUE) -> String {
    match ruby_bridge::str_from_rb(val) {
        Some(s) => s,
        None => raise_arg_error("expected a String argument"),
    }
}

// ---------------------------------------------------------------------------
// Global: the ImmutableList class VALUE
// ---------------------------------------------------------------------------
//
// We store this once during Init and never change it. The alloc function
// and methods that create new list instances need this to wrap data.

static mut IMMUTABLE_LIST_CLASS: VALUE = 0;

/// The real Ruby `nil` VALUE, obtained at runtime via `rb_eval_string("nil")`.
///
/// We cannot use `ruby_bridge::QNIL` (the compile-time constant 0x08) because
/// on Windows with MinGW Ruby + MSVC Rust + /FORCE:UNRESOLVED, the constant
/// may not match the actual nil VALUE. By evaluating `nil` in the running
/// interpreter, we get the correct value regardless of platform.
static mut RUBY_NIL: VALUE = 0;

// ---------------------------------------------------------------------------
// Helper: wrap a Rust ImmutableList as a new Ruby object
// ---------------------------------------------------------------------------
//
// This is used by push, set, pop, and from_array to create new Ruby objects
// that wrap new Rust ImmutableList values. Each call creates a FRESH Ruby
// object -- this is critical for preserving immutability semantics.

fn wrap_list(list: ImmutableList) -> VALUE {
    ruby_bridge::wrap_data(unsafe { IMMUTABLE_LIST_CLASS }, list)
}

/// Extract a reference to the Rust ImmutableList inside a Ruby VALUE.
unsafe fn get_list(self_val: VALUE) -> &'static ImmutableList {
    ruby_bridge::unwrap_data::<ImmutableList>(self_val)
}

// ---------------------------------------------------------------------------
// Alloc function -- called by Ruby before `initialize`
// ---------------------------------------------------------------------------
//
// Ruby object creation follows a two-step pattern:
//   1. `allocate` creates the raw object (this function)
//   2. `initialize` fills it in (our `list_initialize`)
//
// We wrap a default empty ImmutableList here. Since ImmutableList is
// already immutable, the alloc/init pattern works cleanly -- initialize
// is a no-op (the empty list is the correct default).

unsafe extern "C" fn list_alloc(klass: VALUE) -> VALUE {
    ruby_bridge::wrap_data(klass, ImmutableList::new())
}

// ---------------------------------------------------------------------------
// initialize -- Ruby constructor
// ---------------------------------------------------------------------------
//
// Creates a new empty immutable list. No arguments needed.
//
//   list = CodingAdventures::ImmutableListNative::ImmutableList.new
//   list.size  # => 0

extern "C" fn list_initialize(self_val: VALUE) -> VALUE {
    // The alloc function already created an empty ImmutableList.
    // Nothing to do here.
    self_val
}

// ---------------------------------------------------------------------------
// Class method: from_array(arr) -> ImmutableList
// ---------------------------------------------------------------------------
//
// Creates an ImmutableList from a Ruby Array of Strings.
//
//   list = ImmutableList.from_array(["a", "b", "c"])
//   list.size  # => 3
//   list.get(0)  # => "a"

extern "C" fn list_from_array(_klass: VALUE, arr_val: VALUE) -> VALUE {
    // We avoid ruby_bridge::vec_str_from_rb because it uses rb_array_len,
    // which is not exported from this Ruby build (it's a macro/inline in
    // MinGW Ruby 3.3). Instead we call Array#length via rb_funcallv and
    // read elements with rb_ary_entry.
    let length_id = unsafe {
        let name = CString::new("length").unwrap();
        rb_intern(name.as_ptr())
    };
    let len_val = unsafe { rb_funcallv(arr_val, length_id, 0, std::ptr::null()) };
    let len = unsafe { rb_num2long(len_val) } as usize;

    let mut strings = Vec::with_capacity(len);
    for i in 0..len {
        let entry = ruby_bridge::array_entry(arr_val, i);
        match ruby_bridge::str_from_rb(entry) {
            Some(s) => strings.push(s),
            None => raise_arg_error("from_array: all elements must be Strings"),
        }
    }
    let list = ImmutableList::from_slice(&strings);
    wrap_list(list)
}

// ---------------------------------------------------------------------------
// push(value) -> new ImmutableList
// ---------------------------------------------------------------------------
//
// Appends a string to the end of the list and returns a NEW list.
// The original list is unchanged (structural sharing).
//
//   a = ImmutableList.new
//   b = a.push("hello")
//   a.size  # => 0  (unchanged!)
//   b.size  # => 1
//
// CRITICAL: This creates a new Ruby object wrapping a new Rust ImmutableList.
// It does NOT modify self.

extern "C" fn list_push(self_val: VALUE, value_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    let s = string_from_rb(value_val);
    let new_list = list.push(s);
    wrap_list(new_list)
}

// ---------------------------------------------------------------------------
// get(index) -> String or nil
// ---------------------------------------------------------------------------
//
// Returns the element at the given index, or nil if out of bounds.
//
//   list = ImmutableList.from_array(["a", "b", "c"])
//   list.get(0)  # => "a"
//   list.get(5)  # => nil

extern "C" fn list_get(self_val: VALUE, index_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    let index = usize_from_rb(index_val);
    match list.get(index) {
        Some(s) => ruby_bridge::str_to_rb(s),
        None => unsafe { RUBY_NIL },
    }
}

// ---------------------------------------------------------------------------
// set(index, value) -> new ImmutableList
// ---------------------------------------------------------------------------
//
// Returns a NEW list with the element at `index` replaced by `value`.
// Raises ArgumentError if index is out of bounds.
//
//   a = ImmutableList.from_array(["x", "y"])
//   b = a.set(0, "z")
//   a.get(0)  # => "x"  (unchanged!)
//   b.get(0)  # => "z"
//
// CRITICAL: This creates a new Ruby object wrapping a new Rust ImmutableList.

extern "C" fn list_set(self_val: VALUE, index_val: VALUE, value_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    let index = usize_from_rb(index_val);
    let s = string_from_rb(value_val);

    if index >= list.len() {
        raise_arg_error(&format!(
            "index out of bounds: {} >= {}",
            index,
            list.len()
        ));
    }

    let new_list = list.set(index, s);
    wrap_list(new_list)
}

// ---------------------------------------------------------------------------
// pop -> [new_list, value]
// ---------------------------------------------------------------------------
//
// Removes the last element and returns a two-element Ruby Array:
// [new_list, removed_value]. Raises ArgumentError if the list is empty.
//
//   list = ImmutableList.from_array(["a", "b", "c"])
//   result = list.pop
//   result[0].size  # => 2  (new list)
//   result[1]       # => "c" (removed value)
//   list.size       # => 3  (unchanged!)
//
// CRITICAL: The new list in result[0] is a new Ruby object.

extern "C" fn list_pop(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };

    if list.is_empty() {
        raise_arg_error("cannot pop from an empty list");
    }

    let (new_list, removed) = list.pop();
    let result = ruby_bridge::array_new();
    ruby_bridge::array_push(result, wrap_list(new_list));
    ruby_bridge::array_push(result, ruby_bridge::str_to_rb(&removed));
    result
}

// ---------------------------------------------------------------------------
// size -> Integer
// ---------------------------------------------------------------------------
//
// Returns the number of elements in the list. O(1).
//
//   list = ImmutableList.from_array(["a", "b"])
//   list.size  # => 2

extern "C" fn list_size(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    ruby_bridge::usize_to_rb(list.len())
}

// ---------------------------------------------------------------------------
// empty? -> true/false
// ---------------------------------------------------------------------------
//
// Returns true if the list has zero elements.
//
//   ImmutableList.new.empty?  # => true
//   ImmutableList.from_array(["a"]).empty?  # => false

extern "C" fn list_empty(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    ruby_bridge::bool_to_rb(list.is_empty())
}

// ---------------------------------------------------------------------------
// to_a -> Array<String>
// ---------------------------------------------------------------------------
//
// Converts the immutable list to a Ruby Array of Strings.
//
//   list = ImmutableList.from_array(["a", "b", "c"])
//   list.to_a  # => ["a", "b", "c"]

extern "C" fn list_to_a(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    let vec = list.to_vec();
    ruby_bridge::vec_str_to_rb(&vec)
}

// ---------------------------------------------------------------------------
// each -> Array<String>
// ---------------------------------------------------------------------------
//
// Returns a Ruby Array of all elements in order. This is equivalent to
// `to_a` and exists so that Ruby code can iterate via `list.each.each { }`.
//
// We avoid using rb_block_given_p / rb_yield because on Windows with
// MinGW Ruby + MSVC Rust, these symbols may not be exported from the
// Ruby DLL's import library. Returning an array is a portable alternative
// that lets Ruby-side code handle block iteration.
//
//   list = ImmutableList.from_array(["a", "b", "c"])
//   list.each { |e| puts e }
//   # prints: a, b, c (via Array#each)

extern "C" fn list_each(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    let vec = list.to_vec();
    ruby_bridge::vec_str_to_rb(&vec)
}

// ---------------------------------------------------------------------------
// inspect -> String
// ---------------------------------------------------------------------------
//
// Returns a human-readable string representation of the list.
//
//   list = ImmutableList.from_array(["a", "b"])
//   list.inspect  # => "ImmutableList[a, b]"
//
// This uses the Rust Display implementation which formats as
// "ImmutableList[elem0, elem1, ...]".

extern "C" fn list_inspect(self_val: VALUE) -> VALUE {
    let list = unsafe { get_list(self_val) };
    ruby_bridge::str_to_rb(&list.to_string())
}

// ---------------------------------------------------------------------------
// ==(other) -> true/false
// ---------------------------------------------------------------------------
//
// Two ImmutableLists are equal if they contain the same elements in the
// same order.
//
//   a = ImmutableList.from_array(["x", "y"])
//   b = ImmutableList.from_array(["x", "y"])
//   a == b  # => true
//
//   c = ImmutableList.from_array(["x"])
//   a == c  # => false

extern "C" fn list_eq(self_val: VALUE, other_val: VALUE) -> VALUE {
    let a = unsafe { get_list(self_val) };
    let b = unsafe { get_list(other_val) };
    ruby_bridge::bool_to_rb(a == b)
}

// ---------------------------------------------------------------------------
// Init_immutable_list_native -- Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_immutable_list_native` because Ruby
// derives the init function name from the .so filename. When Ruby loads
// `immutable_list_native.so`, it calls `Init_immutable_list_native()`.
//
// We set up the module hierarchy and bind all methods here:
//
//   module CodingAdventures
//     module ImmutableListNative
//       class ImmutableList
//         # ... methods ...
//       end
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_immutable_list_native() {
    // -- Obtain the real nil VALUE at runtime --------------------------------
    //
    // On Windows with MinGW Ruby + MSVC Rust + /FORCE:UNRESOLVED, the
    // compile-time constant ruby_bridge::QNIL (0x08) may not match the
    // actual nil VALUE. We evaluate `nil` in the running interpreter to
    // get the correct value.
    unsafe {
        let nil_str = CString::new("nil").unwrap();
        RUBY_NIL = rb_eval_string(nil_str.as_ptr());
    }

    // -- Module hierarchy ---------------------------------------------------
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let immutable_list_native =
        ruby_bridge::define_module_under(coding_adventures, "ImmutableListNative");

    // -- ImmutableList class ------------------------------------------------
    let list_class = ruby_bridge::define_class_under(
        immutable_list_native,
        "ImmutableList",
        get_ruby_class("Object"),
    );
    unsafe { IMMUTABLE_LIST_CLASS = list_class };

    // -- Allocator ----------------------------------------------------------
    ruby_bridge::define_alloc_func(list_class, list_alloc);

    // -- initialize() -------------------------------------------------------
    ruby_bridge::define_method_raw(
        list_class,
        "initialize",
        list_initialize as *const c_void,
        0,
    );

    // -- Class (singleton) methods ------------------------------------------
    ruby_bridge::define_singleton_method_raw(
        list_class,
        "from_array",
        list_from_array as *const c_void,
        1,
    );

    // -- Instance methods ---------------------------------------------------

    // push(value) -> new ImmutableList
    ruby_bridge::define_method_raw(
        list_class,
        "push",
        list_push as *const c_void,
        1,
    );

    // get(index) -> String or nil
    ruby_bridge::define_method_raw(
        list_class,
        "get",
        list_get as *const c_void,
        1,
    );

    // set(index, value) -> new ImmutableList
    ruby_bridge::define_method_raw(
        list_class,
        "set",
        list_set as *const c_void,
        2,
    );

    // pop -> [new_list, value]
    ruby_bridge::define_method_raw(
        list_class,
        "pop",
        list_pop as *const c_void,
        0,
    );

    // size -> Integer
    ruby_bridge::define_method_raw(
        list_class,
        "size",
        list_size as *const c_void,
        0,
    );

    // empty? -> true/false
    ruby_bridge::define_method_raw(
        list_class,
        "empty?",
        list_empty as *const c_void,
        0,
    );

    // to_a -> Array<String>
    ruby_bridge::define_method_raw(
        list_class,
        "to_a",
        list_to_a as *const c_void,
        0,
    );

    // each { |elem| ... } -> self
    ruby_bridge::define_method_raw(
        list_class,
        "each",
        list_each as *const c_void,
        0,
    );

    // inspect -> String
    ruby_bridge::define_method_raw(
        list_class,
        "inspect",
        list_inspect as *const c_void,
        0,
    );

    // to_s -> String (alias for inspect)
    ruby_bridge::define_method_raw(
        list_class,
        "to_s",
        list_inspect as *const c_void,
        0,
    );

    // ==(other) -> true/false
    ruby_bridge::define_method_raw(
        list_class,
        "==",
        list_eq as *const c_void,
        1,
    );
}
