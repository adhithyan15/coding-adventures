// lib.rs -- ImmutableList Node.js native extension using node-bridge
// ==================================================================
//
// This crate exposes the Rust `immutable_list::ImmutableList` to Node.js
// via N-API, using our zero-dependency `node-bridge` crate. No napi-rs,
// no napi-sys, no build-time header requirements -- just raw N-API calls
// through node-bridge's safe wrappers.
//
// # Architecture
//
// 1. `napi_register_module_v1()` is the entry point called by Node.js when
//    the addon is loaded. It defines an "ImmutableList" class on the exports
//    object with all list methods.
//
// 2. The constructor (`list_new`) creates a Rust `ImmutableList` and wraps
//    it inside the JS object using `node_bridge::wrap_data()`. The
//    constructor supports two modes:
//      - new ImmutableList()                       -- creates an empty list
//      - new ImmutableList(["a", "b", "c"])        -- creates from an array
//
// 3. Each method callback extracts `this` and args via `get_cb_info()`,
//    unwraps the ImmutableList pointer, calls the Rust method, marshals the
//    result back to a JS value, and returns it.
//
// 4. Mutation operations (push, set, pop) return *new* JS ImmutableList
//    objects wrapping the new Rust ImmutableList, preserving the persistent
//    data structure semantics. The original JS object is not modified.
//
// # Key design: constructor reference for creating new instances
//
// push, set, and pop all produce new Rust ImmutableList values. To return
// these to JS, we need to create new JS ImmutableList instances. We store
// a persistent reference to the constructor function during module init,
// then use `napi_new_instance` to create empty instances and replace their
// inner data via `std::mem::replace`.
//
// # Method naming
//
// All methods use camelCase to follow JavaScript conventions:
//   push, get, set, pop, length, isEmpty, toArray

use immutable_list::ImmutableList;
use node_bridge::*;

// ---------------------------------------------------------------------------
// Extra N-API externs not in node-bridge
// ---------------------------------------------------------------------------
//
// We need `napi_new_instance` to create new ImmutableList JS objects from
// mutation operations (push, set, pop). We also need `napi_create_reference`
// and `napi_get_reference_value` to store the constructor for later use,
// and `napi_get_value_int64` to extract numbers from JS.

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

    fn napi_is_array(
        env: napi_env,
        value: napi_value,
        result: *mut bool,
    ) -> napi_status;
}

// ---------------------------------------------------------------------------
// Global constructor reference
// ---------------------------------------------------------------------------
//
// We store a persistent reference to the ImmutableList constructor so that
// mutation operations (push, set, pop) can create new ImmutableList instances
// to wrap their results. This is set once during module registration.

static mut CONSTRUCTOR_REF: *mut c_void = ptr::null_mut();

// ---------------------------------------------------------------------------
// Helpers: extract a JS number as usize
// ---------------------------------------------------------------------------

fn i64_from_js(env: napi_env, val: napi_value) -> i64 {
    let mut result: i64 = 0;
    unsafe { napi_get_value_int64(env, val, &mut result) };
    result
}

fn usize_from_js(env: napi_env, val: napi_value) -> usize {
    i64_from_js(env, val) as usize
}

/// Check whether a JS value is an array.
fn is_array(env: napi_env, val: napi_value) -> bool {
    let mut result = false;
    unsafe { napi_is_array(env, val, &mut result) };
    result
}

// ---------------------------------------------------------------------------
// Helper: create a new JS ImmutableList instance wrapping a Rust ImmutableList
// ---------------------------------------------------------------------------
//
// This is used by mutation operations (push, set, pop) which produce a new
// ImmutableList. We call `napi_new_instance` with the saved constructor
// reference, passing no arguments to get an empty instance, then replace
// the internal Rust ImmutableList with the actual result.
//
// The trick: we create `new ImmutableList()` to get a valid JS object with
// the right prototype, then overwrite its wrapped data via std::mem::replace.

fn wrap_new_list(env: napi_env, list: ImmutableList) -> napi_value {
    unsafe {
        // Get the constructor from the stored reference.
        let mut constructor: napi_value = ptr::null_mut();
        napi_get_reference_value(env, CONSTRUCTOR_REF, &mut constructor);

        // Create a new instance: `new ImmutableList()` (no args = empty list).
        let mut instance: napi_value = ptr::null_mut();
        napi_new_instance(env, constructor, 0, ptr::null(), &mut instance);

        // Replace the inner ImmutableList with the one we actually want.
        let inner = unwrap_data_mut::<ImmutableList>(env, instance);
        let _ = std::mem::replace(inner, list);

        instance
    }
}

// ---------------------------------------------------------------------------
// Constructor: new ImmutableList() | new ImmutableList(["a", "b", "c"])
// ---------------------------------------------------------------------------
//
// The constructor supports two calling conventions:
//
//   new ImmutableList()              -- creates an empty list
//   new ImmutableList(["a", "b"])    -- creates a list from a JS array
//
// We detect the mode by checking argument count and whether the first
// argument is an array.

unsafe extern "C" fn list_new(env: napi_env, info: napi_callback_info) -> napi_value {
    // Get up to 1 argument.
    let (this, args) = get_cb_info(env, info, 1);

    let list = if args.is_empty() {
        // No arguments: create an empty list.
        ImmutableList::new()
    } else if is_array(env, args[0]) {
        // Array argument: build the list from the array elements.
        let len = array_len(env, args[0]);
        let mut items: Vec<String> = Vec::with_capacity(len as usize);
        for i in 0..len {
            let elem = array_get(env, args[0], i);
            match str_from_js(env, elem) {
                Some(s) => items.push(s),
                None => {
                    // Non-string element: convert to string representation.
                    // For numbers, we use the JS string conversion.
                    // For simplicity, we require string elements.
                    throw_error(env, "ImmutableList elements must be strings");
                    return undefined(env);
                }
            }
        }
        ImmutableList::from_slice(&items)
    } else {
        // Single non-array argument: error.
        throw_error(env, "ImmutableList constructor expects no arguments or an array of strings");
        return undefined(env);
    };

    wrap_data(env, this, list);
    this
}

// ---------------------------------------------------------------------------
// push(value): Append an element, returning a new list
// ---------------------------------------------------------------------------
//
// ImmutableList.push() is the core "add" operation. It returns a *new* JS
// ImmutableList containing all elements of the original plus the new one.
// The original list is unmodified -- this is the immutability guarantee.
//
//   const list1 = new ImmutableList();
//   const list2 = list1.push("hello");  // list1 is still empty
//   const list3 = list2.push("world");  // list2 still has just "hello"

unsafe extern "C" fn list_push(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "push requires a string argument");
        return undefined(env);
    }
    let value = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "push requires a string argument");
            return undefined(env);
        }
    };
    let list = unwrap_data::<ImmutableList>(env, this);
    let new_list = list.push(value);
    wrap_new_list(env, new_list)
}

// ---------------------------------------------------------------------------
// get(index): Retrieve an element by index
// ---------------------------------------------------------------------------
//
// Returns the string at the given index, or undefined if the index is out
// of bounds. This mirrors the Rust `Option<&str>` return type -- None
// becomes JS undefined, Some(s) becomes a JS string.
//
// Time complexity: O(log32 n), which is effectively O(1) for practical
// sizes (a million-element list is only 4 levels deep).

unsafe extern "C" fn list_get(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if args.is_empty() {
        throw_error(env, "get requires a number argument");
        return undefined(env);
    }
    let index = usize_from_js(env, args[0]);
    let list = unwrap_data::<ImmutableList>(env, this);
    match list.get(index) {
        Some(s) => str_to_js(env, s),
        None => undefined(env),
    }
}

// ---------------------------------------------------------------------------
// set(index, value): Replace an element, returning a new list
// ---------------------------------------------------------------------------
//
// Returns a new ImmutableList with the element at `index` replaced by
// `value`. The original list is unmodified. Panics (throws in JS) if the
// index is out of bounds.

unsafe extern "C" fn list_set(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() < 2 {
        throw_error(env, "set requires two arguments: index (number) and value (string)");
        return undefined(env);
    }
    let index = usize_from_js(env, args[0]);
    let value = match str_from_js(env, args[1]) {
        Some(s) => s,
        None => {
            throw_error(env, "set requires a string value");
            return undefined(env);
        }
    };
    let list = unwrap_data::<ImmutableList>(env, this);
    if index >= list.len() {
        throw_error(env, &format!("index {} out of bounds for list of length {}", index, list.len()));
        return undefined(env);
    }
    let new_list = list.set(index, value);
    wrap_new_list(env, new_list)
}

// ---------------------------------------------------------------------------
// pop(): Remove the last element, returning [new_list, removed_value]
// ---------------------------------------------------------------------------
//
// Returns a JS array with two elements:
//   [0] = new ImmutableList (with last element removed)
//   [1] = the removed string value
//
// Throws if the list is empty.

unsafe extern "C" fn list_pop(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let list = unwrap_data::<ImmutableList>(env, this);
    if list.is_empty() {
        throw_error(env, "cannot pop from an empty list");
        return undefined(env);
    }
    let (new_list, removed) = list.pop();
    let result = array_new(env);
    array_set(env, result, 0, wrap_new_list(env, new_list));
    array_set(env, result, 1, str_to_js(env, &removed));
    result
}

// ---------------------------------------------------------------------------
// length(): Return the number of elements
// ---------------------------------------------------------------------------
//
// This is O(1) -- the length is stored as a field, not computed by
// traversing the trie.

unsafe extern "C" fn list_length(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let list = unwrap_data::<ImmutableList>(env, this);
    usize_to_js(env, list.len())
}

// ---------------------------------------------------------------------------
// isEmpty(): Return true if the list has zero elements
// ---------------------------------------------------------------------------

unsafe extern "C" fn list_is_empty(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let list = unwrap_data::<ImmutableList>(env, this);
    bool_to_js(env, list.is_empty())
}

// ---------------------------------------------------------------------------
// toArray(): Collect all elements into a JS array
// ---------------------------------------------------------------------------
//
// Returns a plain JS array of strings. This iterates over every element
// in the list, so it's O(n). Useful for interop with JS code that expects
// regular arrays.

unsafe extern "C" fn list_to_array(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let list = unwrap_data::<ImmutableList>(env, this);
    let arr = array_new(env);
    for (i, elem) in list.iter().enumerate() {
        array_set(env, arr, i as u32, str_to_js(env, elem));
    }
    arr
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We define an ImmutableList class with all its methods and attach it to
// the exports object.

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Define all instance methods using node-bridge's method_property helper.
    let properties = [
        // -- Mutation operations (return new lists) --
        method_property("push", Some(list_push)),
        method_property("get", Some(list_get)),
        method_property("set", Some(list_set)),
        method_property("pop", Some(list_pop)),
        // -- Query operations --
        method_property("length", Some(list_length)),
        method_property("isEmpty", Some(list_is_empty)),
        // -- Conversion --
        method_property("toArray", Some(list_to_array)),
    ];

    // Create the class with constructor and all methods.
    let class = define_class(env, "ImmutableList", Some(list_new), &properties);

    // Store a persistent reference to the constructor so mutation operations
    // can create new instances (see wrap_new_list).
    napi_create_reference(env, class, 1, &raw mut CONSTRUCTOR_REF);

    // Attach the class constructor to exports.
    set_named_property(env, exports, "ImmutableList", class);

    exports
}
