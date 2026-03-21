//! # python-bridge — Thin safe wrapper over Python's C API
//!
//! This crate replaces PyO3 (~50,000 lines) with ~400 lines of explicit,
//! debuggable code. It wraps only the raw C API functions from `pyo3-ffi`
//! needed to build Python native extensions.
//!
//! ## How Python's C API works
//!
//! Python represents every value as a `*mut PyObject` — a reference-counted
//! heap-allocated structure. When you receive a "new reference" from a C API
//! call, you own it and must eventually call `Py_DECREF`. When you receive
//! a "borrowed reference", you must NOT decref it.
//!
//! ## Modules in this crate
//!
//! - `types` — PyObj RAII wrapper for automatic reference counting
//! - `error` — Exception handling
//! - This file — string, list, set, tuple, bool, int marshaling + module/class creation

pub mod error;
pub mod types;

use std::collections::HashSet;
use std::ffi::{c_char, CString};
use std::ptr;

use pyo3_ffi::*;

pub use types::PyObj;

// ---------------------------------------------------------------------------
// String conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` to a Python str (new reference).
pub unsafe fn str_to_py(s: &str) -> *mut PyObject {
    PyUnicode_FromStringAndSize(s.as_ptr() as *const c_char, s.len() as isize)
}

/// Convert a Python str to a Rust `String`.
///
/// Returns `None` if the object is not a string or contains invalid UTF-8.
pub unsafe fn str_from_py(obj: *mut PyObject) -> Option<String> {
    if obj.is_null() {
        return None;
    }
    let mut size: isize = 0;
    let ptr = PyUnicode_AsUTF8AndSize(obj, &mut size);
    if ptr.is_null() {
        // Clear any error that PyUnicode_AsUTF8AndSize may have set.
        PyErr_Clear();
        return None;
    }
    let slice = std::slice::from_raw_parts(ptr as *const u8, size as usize);
    String::from_utf8(slice.to_vec()).ok()
}

// ---------------------------------------------------------------------------
// List conversion
// ---------------------------------------------------------------------------

/// Convert a `Vec<String>` to a Python list of str.
pub unsafe fn vec_str_to_py(items: &[String]) -> *mut PyObject {
    let list = PyList_New(items.len() as isize);
    for (i, item) in items.iter().enumerate() {
        // PyList_SET_ITEM steals the reference, so str_to_py's new ref is consumed.
        PyList_SetItem(list, i as isize, str_to_py(item));
    }
    list
}

/// Convert a Python list of str to a `Vec<String>`.
pub unsafe fn vec_str_from_py(obj: *mut PyObject) -> Option<Vec<String>> {
    if obj.is_null() {
        return None;
    }
    let len = PyList_Size(obj);
    if len < 0 {
        PyErr_Clear();
        return None;
    }
    let mut result = Vec::with_capacity(len as usize);
    for i in 0..len {
        let item = PyList_GetItem(obj, i); // borrowed reference
        if let Some(s) = str_from_py(item) {
            result.push(s);
        }
    }
    Some(result)
}

/// Convert a `Vec<Vec<String>>` to a Python list of lists of str.
pub unsafe fn vec_vec_str_to_py(items: &[Vec<String>]) -> *mut PyObject {
    let list = PyList_New(items.len() as isize);
    for (i, group) in items.iter().enumerate() {
        PyList_SetItem(list, i as isize, vec_str_to_py(group));
    }
    list
}

/// Convert a `Vec<(String, String)>` to a Python list of (str, str) tuples.
pub unsafe fn vec_tuple2_str_to_py(items: &[(String, String)]) -> *mut PyObject {
    let list = PyList_New(items.len() as isize);
    for (i, (a, b)) in items.iter().enumerate() {
        let tuple = PyTuple_New(2);
        PyTuple_SetItem(tuple, 0, str_to_py(a));
        PyTuple_SetItem(tuple, 1, str_to_py(b));
        PyList_SetItem(list, i as isize, tuple);
    }
    list
}

// ---------------------------------------------------------------------------
// Set conversion
// ---------------------------------------------------------------------------

/// Convert a `HashSet<String>` to a Python set of str.
pub unsafe fn set_str_to_py(items: &HashSet<String>) -> *mut PyObject {
    let set = PySet_New(ptr::null_mut());
    for item in items {
        let s = str_to_py(item);
        PySet_Add(set, s);
        Py_DecRef(s);
    }
    set
}

/// Convert a Python set (or any iterable) of str to a `HashSet<String>`.
pub unsafe fn set_str_from_py(obj: *mut PyObject) -> Option<HashSet<String>> {
    if obj.is_null() {
        return None;
    }
    // Get an iterator over the set/iterable.
    let iter = PyObject_GetIter(obj);
    if iter.is_null() {
        PyErr_Clear();
        return None;
    }
    let mut result = HashSet::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            break;
        }
        if let Some(s) = str_from_py(item) {
            result.insert(s);
        }
        Py_DecRef(item);
    }
    Py_DecRef(iter);
    PyErr_Clear(); // Clear StopIteration
    Some(result)
}

// ---------------------------------------------------------------------------
// Boolean conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `bool` to a Python bool.
pub unsafe fn bool_to_py(b: bool) -> *mut PyObject {
    if b {
        let obj = Py_True();
        Py_IncRef(obj);
        obj
    } else {
        let obj = Py_False();
        Py_IncRef(obj);
        obj
    }
}

// ---------------------------------------------------------------------------
// Integer conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `usize` to a Python int.
pub unsafe fn usize_to_py(n: usize) -> *mut PyObject {
    PyLong_FromSize_t(n)
}

// ---------------------------------------------------------------------------
// None
// ---------------------------------------------------------------------------

/// Return Python `None` (with incremented reference count).
pub unsafe fn none() -> *mut PyObject {
    let n = Py_None();
    Py_IncRef(n);
    n
}

// ---------------------------------------------------------------------------
// Argument parsing helpers
// ---------------------------------------------------------------------------

/// Extract one string argument from a Python args tuple.
pub unsafe fn parse_arg_str(args: *mut PyObject, index: isize) -> Option<String> {
    let arg = PyTuple_GetItem(args, index); // borrowed ref
    str_from_py(arg)
}

/// Extract two string arguments from a Python args tuple.
pub unsafe fn parse_args_2str(args: *mut PyObject) -> Option<(String, String)> {
    let a = parse_arg_str(args, 0)?;
    let b = parse_arg_str(args, 1)?;
    Some((a, b))
}

// ---------------------------------------------------------------------------
// Module creation
// ---------------------------------------------------------------------------

/// Add an object to a Python module (e.g., a class or exception type).
pub unsafe fn module_add_object(module: *mut PyObject, name: &str, obj: *mut PyObject) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    PyModule_AddObject(module, c_name.as_ptr(), obj);
}

// ---------------------------------------------------------------------------
// Exception creation
// ---------------------------------------------------------------------------

/// Create a new exception class and add it to the module.
///
/// Returns the exception type object (new reference).
pub unsafe fn new_exception(
    module_name: &str,
    exc_name: &str,
    base: *mut PyObject,
) -> *mut PyObject {
    let full_name = format!("{}.{}", module_name, exc_name);
    let c_name = CString::new(full_name).expect("name must not contain NUL");
    PyErr_NewException(c_name.as_ptr(), base, ptr::null_mut())
}
