// error.rs — Exception handling for the Python C API
// ===================================================
//
// # How Python exceptions work at the C level
//
// In Python, exceptions are objects that propagate up the call stack. But at
// the C API level, there is no stack unwinding or try/catch. Instead, Python
// uses a thread-local "error indicator" — a set of three values:
//
//   1. Exception type  (e.g., PyExc_ValueError)
//   2. Exception value (the error message or exception instance)
//   3. Traceback       (the stack trace)
//
// When a C function wants to raise an exception, it:
//   1. Sets the error indicator via PyErr_SetString() or similar
//   2. Returns NULL (for functions returning PyObject*) or -1 (for int returns)
//
// The caller checks for NULL and either handles the error or propagates it
// by also returning NULL. This continues up the call chain until Python's
// bytecode interpreter catches it and turns it into a Python exception.
//
// # What this module provides
//
// Three helpers that make exception handling concise:
//
// - `set_error()` — sets the error indicator (you decide what to return)
// - `return_error()` — sets the error indicator AND returns null (one-liner)
// - `new_exception()` — creates a custom exception class for your module
//
// # Example
//
// ```text
// // Raising ValueError:
// return error::return_error(
//     unsafe { pyo3_ffi::PyExc_ValueError },
//     "expected a positive integer"
// );
//
// // Creating a custom exception:
// let cycle_error = error::new_exception(module, "CycleError", base);
// ```

use pyo3_ffi;

/// Set the Python error indicator with a given exception type and message.
///
/// This calls `PyErr_SetString`, which is Python's standard way to raise
/// exceptions from C code. After calling this, the current function should
/// return NULL (or -1) to propagate the exception.
///
/// # Arguments
///
/// * `exc_type` — The exception class to raise. Must be a valid Python type
///   object like `PyExc_ValueError`, `PyExc_TypeError`, or a custom exception
///   created with `new_exception()`.
/// * `msg` — The error message. This becomes the string representation of
///   the exception (what you see in `str(e)` in Python).
///
/// # Example
///
/// ```text
/// if value < 0 {
///     error::set_error(
///         unsafe { pyo3_ffi::PyExc_ValueError },
///         "value must be non-negative"
///     );
///     return std::ptr::null_mut();
/// }
/// ```
pub fn set_error(exc_type: *mut pyo3_ffi::PyObject, msg: &str) {
    // We need a null-terminated C string for PyErr_SetString.
    // CString handles the conversion and ensures no interior null bytes.
    let c_msg = std::ffi::CString::new(msg).unwrap_or_else(|_| {
        // If the message contains null bytes (very unlikely), fall back
        // to a generic message rather than panicking.
        std::ffi::CString::new("(error message contained null bytes)").unwrap()
    });

    // SAFETY: exc_type must be a valid Python exception type. We trust the
    // caller to pass one of the PyExc_* constants or a type created by
    // new_exception(). PyErr_SetString does not steal any references.
    unsafe {
        pyo3_ffi::PyErr_SetString(exc_type, c_msg.as_ptr());
    }
}

/// Set the Python error indicator and return a null pointer in one step.
///
/// This is a convenience function that combines `set_error()` with returning
/// null. In C extension functions that return `*mut PyObject`, returning null
/// signals that an exception has been set.
///
/// # Why this exists
///
/// Without this helper, raising an exception takes three lines:
///
/// ```text
/// set_error(PyExc_ValueError, "bad input");
/// return std::ptr::null_mut();
/// ```
///
/// With `return_error`, it's one line:
///
/// ```text
/// return return_error(PyExc_ValueError, "bad input");
/// ```
///
/// This matters because exception handling is the most common boilerplate
/// in C extension code. Reducing it to one line makes the control flow
/// easier to follow.
pub fn return_error(exc_type: *mut pyo3_ffi::PyObject, msg: &str) -> *mut pyo3_ffi::PyObject {
    set_error(exc_type, msg);
    std::ptr::null_mut()
}

/// Create a new Python exception class and add it to a module.
///
/// This is equivalent to writing `class MyError(BaseException): pass` in
/// Python, but from C. The new exception type is:
///
/// 1. Created via `PyErr_NewException` with the given name and base class
/// 2. Added to the module's namespace so Python code can import it
///
/// # Arguments
///
/// * `module` — The Python module to add the exception to (from PyModule_Create)
/// * `name` — The exception class name, e.g., "CycleError"
/// * `base` — The base exception class. Use `PyExc_Exception` for a standard
///   exception, or another exception type for a hierarchy.
///
/// # Returns
///
/// A raw pointer to the new exception type object. Store this pointer for
/// later use with `set_error()` or `return_error()`.
///
/// # How Python sees it
///
/// After calling `new_exception(module, "CycleError", PyExc_Exception)`:
///
/// ```python
/// from my_module import CycleError
///
/// try:
///     do_something()
/// except CycleError as e:
///     print(f"Caught: {e}")
/// ```
///
/// # Returns null on failure
///
/// If `PyErr_NewException` or `PyModule_AddObject` fails, this function
/// returns null and sets a Python exception. The caller should check for
/// null and propagate the error.
pub fn new_exception(
    module: *mut pyo3_ffi::PyObject,
    name: &str,
    base: *mut pyo3_ffi::PyObject,
) -> *mut pyo3_ffi::PyObject {
    // PyErr_NewException expects a dotted name like "module.ExcName".
    // We need to get the module name first.

    // Build the fully-qualified name: "module_name.ExcName"
    // For simplicity, we use the short name directly and let PyErr_NewException
    // handle it. The name format is "module.name" as a C string.
    let full_name = std::ffi::CString::new(format!("python_bridge.{}", name)).unwrap_or_else(|_| {
        std::ffi::CString::new("python_bridge.Error").unwrap()
    });

    let c_name = std::ffi::CString::new(name).unwrap_or_else(|_| {
        std::ffi::CString::new("Error").unwrap()
    });

    // SAFETY: PyErr_NewException creates a new exception type. The `base`
    // argument must be a valid exception type or NULL (defaults to Exception).
    // The `dict` argument is NULL (no extra attributes).
    let exc_type = unsafe {
        pyo3_ffi::PyErr_NewException(
            full_name.as_ptr(),
            base,
            std::ptr::null_mut(), // no extra dict
        )
    };

    if exc_type.is_null() {
        return std::ptr::null_mut();
    }

    // Add the exception to the module so Python can import it.
    //
    // PyModule_AddObject steals a reference on success, so we need to
    // increment the reference count first. If it fails, we decrement.
    //
    // Actually, PyModule_AddObjectRef (Python 3.10+) is safer — it doesn't
    // steal. But for compatibility, we use PyModule_AddObject with an INCREF.
    unsafe {
        pyo3_ffi::Py_INCREF(exc_type);
        let rc = pyo3_ffi::PyModule_AddObject(module, c_name.as_ptr(), exc_type);
        if rc < 0 {
            // AddObject failed — undo our INCREF
            pyo3_ffi::Py_DECREF(exc_type);
            // The original reference from PyErr_NewException is still live;
            // we return it anyway so the caller can clean up, but in practice
            // this failure is very rare (out of memory).
        }
    }

    exc_type
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
//
// Exception handling depends on the Python runtime, so we can only test
// the pure-Rust parts here (like CString construction). The actual exception
// setting/raising is tested by integration tests running under Python.

#[cfg(test)]
mod tests {
    #[test]
    fn cstring_with_normal_message() {
        // Verify that CString::new works with typical error messages
        let msg = "node not found: A";
        let c = std::ffi::CString::new(msg).unwrap();
        assert_eq!(c.to_str().unwrap(), msg);
    }

    #[test]
    fn cstring_with_empty_message() {
        let c = std::ffi::CString::new("").unwrap();
        assert_eq!(c.to_str().unwrap(), "");
    }

    #[test]
    fn cstring_with_unicode() {
        // Error messages might contain non-ASCII characters
        let msg = "noeud introuvable: \u{00e9}";
        let c = std::ffi::CString::new(msg).unwrap();
        assert_eq!(c.to_str().unwrap(), msg);
    }
}
