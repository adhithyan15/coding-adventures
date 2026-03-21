// types.rs — PyObj: a safe wrapper around Python's PyObject pointer
// ================================================================
//
// # The Problem
//
// Python's C API deals in raw `*mut PyObject` pointers everywhere. These
// pointers are reference-counted: every time you get a "new reference" from
// a C API call, you must eventually call `Py_DECREF` to release it. Forget
// to decref and you leak memory. Decref too many times and you crash.
//
// # The Solution: RAII
//
// Rust's ownership system is the perfect fit for reference counting. We wrap
// the raw pointer in a `PyObj` struct that:
//
// 1. **Owns** one reference — creating a PyObj from a "new reference" pointer
//    takes ownership of that reference.
// 2. **Drops** the reference — when the PyObj goes out of scope, `Drop` calls
//    `Py_DECREF` automatically.
// 3. **Transfers** ownership — `into_ptr()` hands the raw pointer back without
//    decrementing, for when you're giving the reference to Python.
//
// This is exactly the same pattern as `Box<T>` or `Arc<T>` — Rust's type
// system enforces correct reference counting at compile time.
//
// # Analogy
//
// Think of PyObj like a library book checkout card:
// - Creating a PyObj = checking out a book (incrementing the borrow count)
// - Dropping a PyObj = returning the book (decrementing the count)
// - `into_ptr()` = transferring your checkout to someone else
// - When the count reaches zero, the library discards the book (Python frees
//   the object)

use pyo3_ffi;

/// A safe wrapper around a raw Python object pointer (`*mut PyObject`).
///
/// PyObj owns exactly one reference to the Python object. When it is dropped,
/// `Py_DECREF` is called to release that reference. This prevents memory leaks
/// without requiring manual reference counting.
///
/// # Safety invariant
///
/// The wrapped pointer must be either:
/// - A valid "new reference" from a Python C API call, or
/// - Null (representing an error / missing value)
///
/// PyObj handles the null case gracefully — dropping a null PyObj is a no-op.
pub struct PyObj {
    /// The raw Python object pointer. May be null.
    ptr: *mut pyo3_ffi::PyObject,
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

impl PyObj {
    /// Wrap a raw pointer that represents a NEW reference.
    ///
    /// # When to use this
    ///
    /// Most Python C API functions return a "new reference" — a pointer with
    /// an incremented reference count. Wrap those with `PyObj::from_owned()`:
    ///
    /// ```text
    /// let list = PyObj::from_owned(PyList_New(5));
    /// // PyObj now owns the reference; Py_DECREF is called on drop
    /// ```
    ///
    /// # When NOT to use this
    ///
    /// Some C API functions return "borrowed references" (like `PyTuple_GetItem`).
    /// Those must NOT be wrapped directly — you'd need to `Py_INCREF` first.
    /// However, in this bridge we avoid borrowed references entirely by using
    /// the safer `PySequence_GetItem` variants that return new references.
    pub fn from_owned(ptr: *mut pyo3_ffi::PyObject) -> Self {
        PyObj { ptr }
    }

    /// Create a null PyObj.
    ///
    /// This is useful as a sentinel or error value. Dropping a null PyObj
    /// is safe — the Drop implementation checks for null before decrementing.
    ///
    /// ```text
    /// let empty = PyObj::null();
    /// assert!(empty.is_null());
    /// ```
    pub fn null() -> Self {
        PyObj {
            ptr: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

impl PyObj {
    /// Check whether this PyObj holds a null pointer.
    ///
    /// A null PyObj typically means a Python C API call failed and set an
    /// exception. After getting a null result, you should propagate the error
    /// back to Python (by returning null from your own function).
    ///
    /// ```text
    /// let result = PyObj::from_owned(some_c_api_call());
    /// if result.is_null() {
    ///     // An exception was set — propagate it
    ///     return std::ptr::null_mut();
    /// }
    /// ```
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// Get the raw pointer without consuming the PyObj.
    ///
    /// The PyObj retains ownership — when it is dropped, `Py_DECREF` will
    /// still be called. Use this when you need to pass the pointer to a C API
    /// function that does NOT steal a reference.
    ///
    /// ```text
    /// let list = PyObj::from_owned(PyList_New(3));
    /// // PyList_SetItem steals a reference to the *item*, but the *list*
    /// // pointer is just read, not consumed:
    /// PyList_SetItem(list.as_ptr(), 0, some_item);
    /// ```
    pub fn as_ptr(&self) -> *mut pyo3_ffi::PyObject {
        self.ptr
    }

    /// Consume the PyObj and return the raw pointer WITHOUT calling Py_DECREF.
    ///
    /// This transfers ownership of the reference to the caller. Use this when
    /// returning a Python object from a C extension function — Python will own
    /// the reference and manage its lifetime.
    ///
    /// ```text
    /// // In a method that returns *mut PyObject to Python:
    /// let result = PyObj::from_owned(PyLong_FromLong(42));
    /// result.into_ptr()  // Python now owns the reference
    /// ```
    ///
    /// After calling `into_ptr()`, the PyObj is consumed (moved) so the
    /// compiler prevents double-free at compile time.
    pub fn into_ptr(self) -> *mut pyo3_ffi::PyObject {
        let ptr = self.ptr;
        // Prevent the Drop implementation from calling Py_DECREF.
        // std::mem::forget consumes self without running its destructor.
        std::mem::forget(self);
        ptr
    }
}

// ---------------------------------------------------------------------------
// Automatic reference counting via Drop
// ---------------------------------------------------------------------------
//
// This is the key insight: by implementing Drop, we get automatic reference
// counting. Every PyObj that goes out of scope decrements the Python object's
// reference count. No manual Py_DECREF calls needed — and no forgetting them.
//
// Compare with C:
//   PyObject* obj = PyList_New(0);
//   // ... use obj ...
//   Py_DECREF(obj);  // Easy to forget! Memory leak if you do.
//
// With PyObj:
//   let obj = PyObj::from_owned(PyList_New(0));
//   // ... use obj ...
//   // Py_DECREF happens automatically when obj goes out of scope

impl Drop for PyObj {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: We only wrap pointers that came from Python C API calls
            // as "new references." Each PyObj owns exactly one reference, so
            // calling Py_DECREF once is correct. The null check above prevents
            // decrementing a null pointer (which would be undefined behavior).
            unsafe {
                pyo3_ffi::Py_DECREF(self.ptr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Why no Clone?
// ---------------------------------------------------------------------------
//
// We deliberately do NOT implement Clone for PyObj. Cloning would require
// calling Py_INCREF to add a new reference, which we could do, but it would
// make ownership less explicit. If you need a second reference to the same
// Python object, use Py_INCREF manually and wrap the result in a new PyObj.
// This keeps the ownership model simple and auditable.
//
// ---------------------------------------------------------------------------
// Why no Send/Sync?
// ---------------------------------------------------------------------------
//
// Python objects are not thread-safe — they can only be accessed while holding
// the GIL (Global Interpreter Lock). Since PyObj wraps a Python object, it
// should not be sent across threads. Rust's auto-trait rules already handle
// this: *mut PyObject is !Send and !Sync, so PyObj inherits those constraints
// automatically. No extra work needed.

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------
//
// Note: We can't test Py_DECREF without an actual Python interpreter, so
// these tests only verify the null/non-null logic and into_ptr ownership
// transfer. The actual reference counting is tested by integration tests
// that load the extension into Python.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_pyobj_is_null() {
        let obj = PyObj::null();
        assert!(obj.is_null());
    }

    #[test]
    fn null_pyobj_has_null_ptr() {
        let obj = PyObj::null();
        assert!(obj.as_ptr().is_null());
    }

    #[test]
    fn from_owned_with_null_is_null() {
        let obj = PyObj::from_owned(std::ptr::null_mut());
        assert!(obj.is_null());
    }

    #[test]
    fn into_ptr_returns_the_pointer() {
        // We can't create a real PyObject without Python, but we can verify
        // that into_ptr returns the same pointer and prevents Drop.
        let fake_ptr = 0xDEAD_BEEF as *mut pyo3_ffi::PyObject;
        let obj = PyObj::from_owned(fake_ptr);
        let returned = obj.into_ptr();
        assert_eq!(returned, fake_ptr);
        // obj is consumed — Drop is NOT called (which is correct, since
        // fake_ptr isn't a real Python object)
    }

    #[test]
    fn as_ptr_does_not_consume() {
        let obj = PyObj::from_owned(std::ptr::null_mut());
        let _p1 = obj.as_ptr();
        let _p2 = obj.as_ptr(); // Can call multiple times
        assert!(obj.is_null());
        // Drop is called here, but it's a no-op for null
    }
}
