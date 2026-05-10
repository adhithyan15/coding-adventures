//! # python-bridge — Zero-dependency Rust wrapper for Python's C API
//!
//! This crate provides safe Rust wrappers around Python's C extension API
//! using raw `extern "C"` declarations. No pyo3, no pyo3-ffi, no bindgen,
//! no build-time header requirements. Compiles on any platform with just
//! a Rust toolchain.
//!
//! ## How it works
//!
//! Python's C API exports a set of well-known functions from `libpython`.
//! These functions are part of the "Limited API" (PEP 384) and have been
//! ABI-stable since Python 3.2 (2011). We declare them as `extern "C"`
//! and call them directly. When the extension is loaded by Python, the
//! dynamic linker resolves these symbols against the running Python
//! interpreter.
//!
//! ## Why zero dependencies?
//!
//! - **Compiles everywhere** — no Python headers needed at build time
//! - **No bindgen** — no clang/LLVM dependency
//! - **No version conflicts** — works with any Python 3.x
//! - **Fully auditable** — every C function call is visible and grep-able

use std::collections::{BTreeMap, HashSet};
use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// The PyObject type
// ---------------------------------------------------------------------------
//
// Python represents every value as a heap-allocated PyObject. At the C level,
// we only ever deal with pointers to these objects. The actual struct layout
// is an implementation detail — we just need the pointer.

/// Opaque Python object. All Python values are `*mut PyObject` at the C level.
#[repr(C)]
pub struct PyObject {
    _opaque: [u8; 0],
}

/// Convenience alias — every C API function deals in these.
pub type PyObjectPtr = *mut PyObject;

// ---------------------------------------------------------------------------
// Python's C API — extern "C" declarations
// ---------------------------------------------------------------------------
//
// These are the stable, ABI-compatible functions exported by libpython.
// They have been unchanged since Python 3.2+ (Limited API / PEP 384).
// When our .so/.pyd is loaded by Python, the dynamic linker resolves
// these symbols against the running interpreter.

#[allow(non_snake_case)]
extern "C" {
    // -- Reference counting ------------------------------------------------
    pub fn Py_IncRef(o: PyObjectPtr);
    pub fn Py_DecRef(o: PyObjectPtr);

    // -- Module creation ---------------------------------------------------
    pub fn PyModule_Create2(def: *mut PyModuleDef, apiver: c_int) -> PyObjectPtr;
    pub fn PyModule_AddObject(
        module: PyObjectPtr,
        name: *const c_char,
        value: PyObjectPtr,
    ) -> c_int;

    // -- Type/class creation -----------------------------------------------
    pub fn PyType_FromSpec(spec: *mut PyType_Spec) -> PyObjectPtr;

    // -- String conversion -------------------------------------------------
    pub fn PyUnicode_FromStringAndSize(u: *const c_char, size: isize) -> PyObjectPtr;
    pub fn PyUnicode_AsUTF8AndSize(
        unicode: PyObjectPtr,
        size: *mut isize,
    ) -> *const c_char;
    pub fn PyBytes_FromStringAndSize(v: *const c_char, len: isize) -> PyObjectPtr;
    pub fn PyBytes_AsStringAndSize(
        obj: PyObjectPtr,
        buffer: *mut *mut c_char,
        length: *mut isize,
    ) -> c_int;

    // -- List operations ---------------------------------------------------
    pub fn PyList_New(len: isize) -> PyObjectPtr;
    pub fn PyList_SetItem(list: PyObjectPtr, index: isize, item: PyObjectPtr) -> c_int;
    pub fn PyList_GetItem(list: PyObjectPtr, index: isize) -> PyObjectPtr;
    pub fn PyList_Size(list: PyObjectPtr) -> isize;

    // -- Tuple operations --------------------------------------------------
    pub fn PyTuple_New(len: isize) -> PyObjectPtr;
    pub fn PyTuple_SetItem(tuple: PyObjectPtr, pos: isize, item: PyObjectPtr) -> c_int;
    pub fn PyTuple_GetItem(tuple: PyObjectPtr, pos: isize) -> PyObjectPtr;

    // -- Set operations ----------------------------------------------------
    pub fn PySet_New(iterable: PyObjectPtr) -> PyObjectPtr;
    pub fn PySet_Add(set: PyObjectPtr, key: PyObjectPtr) -> c_int;

    // -- Integer / float operations ---------------------------------------
    pub fn PyLong_FromLong(v: c_long) -> PyObjectPtr;
    pub fn PyFloat_FromDouble(v: f64) -> PyObjectPtr;
    pub fn PyFloat_AsDouble(obj: PyObjectPtr) -> f64;
    pub fn PyNumber_Float(obj: PyObjectPtr) -> PyObjectPtr;

    // -- Dict operations ---------------------------------------------------
    pub fn PyDict_New() -> PyObjectPtr;
    pub fn PyDict_SetItem(dict: PyObjectPtr, key: PyObjectPtr, val: PyObjectPtr) -> c_int;

    // -- Iterator ----------------------------------------------------------
    pub fn PyObject_GetIter(o: PyObjectPtr) -> PyObjectPtr;
    pub fn PyIter_Next(iter: PyObjectPtr) -> PyObjectPtr;

    // -- Error handling ----------------------------------------------------
    pub fn PyErr_SetString(type_: PyObjectPtr, message: *const c_char);
    pub fn PyErr_NewException(
        name: *const c_char,
        base: PyObjectPtr,
        dict: PyObjectPtr,
    ) -> PyObjectPtr;
    pub fn PyErr_Clear();

    // -- Object protocol ---------------------------------------------------
    pub fn PyObject_GetAttrString(obj: PyObjectPtr, name: *const c_char) -> PyObjectPtr;

    // -- Bool creation (avoids dllimport issue with _Py_TrueStruct) --------
    pub fn PyBool_FromLong(v: c_long) -> PyObjectPtr;

    // -- Py_None / Py_True / Py_False via Py_BuildValue --------------------
    // On Windows, accessing _Py_NoneStruct and PyExc_* as extern statics
    // requires dllimport which Rust doesn't support portably. Instead we
    // use functions that return the same values.
    pub fn Py_BuildValue(format: *const c_char, ...) -> PyObjectPtr;

    // -- Import ------------------------------------------------------------
    pub fn PyImport_ImportModule(name: *const c_char) -> PyObjectPtr;
}

// ---------------------------------------------------------------------------
// Module definition structures
// ---------------------------------------------------------------------------
//
// These must match Python's C struct layout exactly. The #[repr(C)]
// attribute ensures Rust uses the same memory layout as C.

/// Mirrors Python's PyModuleDef_Base.
///
/// PyModuleDef_Base starts with PyObject (ob_refcnt + ob_type = 2 pointers),
/// then m_init, m_index, m_copy. Total: 5 pointer-sized fields.
#[repr(C)]
pub struct PyModuleDef_Base {
    pub ob_base: [u8; std::mem::size_of::<usize>() * 2], // PyObject: ob_refcnt + ob_type
    pub m_init: Option<unsafe extern "C" fn() -> PyObjectPtr>,
    pub m_index: isize,
    pub m_copy: PyObjectPtr,
}

/// Mirrors Python's PyModuleDef.
#[repr(C)]
pub struct PyModuleDef {
    pub m_base: PyModuleDef_Base,
    pub m_name: *const c_char,
    pub m_doc: *const c_char,
    pub m_size: isize,
    pub m_methods: *mut PyMethodDef,
    pub m_slots: *mut c_void,
    pub m_traverse: *mut c_void,
    pub m_clear: *mut c_void,
    pub m_free: *mut c_void,
}

/// Mirrors Python's PyMethodDef.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PyMethodDef {
    pub ml_name: *const c_char,
    pub ml_meth: Option<unsafe extern "C" fn(PyObjectPtr, PyObjectPtr) -> PyObjectPtr>,
    pub ml_flags: c_int,
    pub ml_doc: *const c_char,
}

/// Mirrors Python's PyType_Spec.
#[repr(C)]
pub struct PyType_Spec {
    pub name: *const c_char,
    pub basicsize: c_int,
    pub itemsize: c_int,
    pub flags: u32,
    pub slots: *mut PyType_Slot,
}

/// Mirrors Python's PyType_Slot.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PyType_Slot {
    pub slot: c_int,
    pub pfunc: *mut c_void,
}

// Method flags
pub const METH_VARARGS: c_int = 0x0001;
pub const METH_NOARGS: c_int = 0x0004;

// Type flags — Py_TPFLAGS_DEFAULT includes Py_TPFLAGS_HAVE_VERSION_TAG
// which is required for type slots to work properly with PyType_FromSpec.
pub const PY_TPFLAGS_DEFAULT: u32 = 1 << 18;

// Module API version (Python 3)
pub const PYTHON_API_VERSION: c_int = 1013;

// ---------------------------------------------------------------------------
// Safe wrappers — String conversion
// ---------------------------------------------------------------------------

/// Convert a Rust `&str` to a Python str (new reference).
pub unsafe fn str_to_py(s: &str) -> PyObjectPtr {
    PyUnicode_FromStringAndSize(s.as_ptr() as *const c_char, s.len() as isize)
}

/// Convert a Python str to a Rust `String`.
pub unsafe fn str_from_py(obj: PyObjectPtr) -> Option<String> {
    if obj.is_null() {
        return None;
    }
    let mut size: isize = 0;
    let ptr = PyUnicode_AsUTF8AndSize(obj, &mut size);
    if ptr.is_null() {
        PyErr_Clear();
        return None;
    }
    let slice = std::slice::from_raw_parts(ptr as *const u8, size as usize);
    String::from_utf8(slice.to_vec()).ok()
}

/// Convert a Rust byte slice to a Python `bytes` object (new reference).
pub unsafe fn bytes_to_py(bytes: &[u8]) -> PyObjectPtr {
    unsafe { PyBytes_FromStringAndSize(bytes.as_ptr() as *const c_char, bytes.len() as isize) }
}

/// Convert a Python `bytes` object to an owned Rust byte buffer.
pub unsafe fn bytes_from_py(obj: PyObjectPtr) -> Option<Vec<u8>> {
    if obj.is_null() {
        return None;
    }

    let mut ptr: *mut c_char = ptr::null_mut();
    let mut len: isize = 0;
    if unsafe { PyBytes_AsStringAndSize(obj, &mut ptr, &mut len) } != 0 {
        unsafe { PyErr_Clear() };
        return None;
    }
    if ptr.is_null() || len < 0 {
        return None;
    }

    Some(unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }.to_vec())
}

// ---------------------------------------------------------------------------
// Safe wrappers — List conversion
// ---------------------------------------------------------------------------

/// Convert a `&[String]` to a Python list of str.
pub unsafe fn vec_str_to_py(items: &[String]) -> PyObjectPtr {
    let list = PyList_New(items.len() as isize);
    for (i, item) in items.iter().enumerate() {
        PyList_SetItem(list, i as isize, str_to_py(item));
    }
    list
}

/// Convert a Python list of str to a `Vec<String>`.
pub unsafe fn vec_str_from_py(obj: PyObjectPtr) -> Option<Vec<String>> {
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
        let item = PyList_GetItem(obj, i);
        if let Some(s) = str_from_py(item) {
            result.push(s);
        }
    }
    Some(result)
}

/// Convert a `&[Vec<String>]` to a Python list of lists of str.
pub unsafe fn vec_vec_str_to_py(items: &[Vec<String>]) -> PyObjectPtr {
    let list = PyList_New(items.len() as isize);
    for (i, group) in items.iter().enumerate() {
        PyList_SetItem(list, i as isize, vec_str_to_py(group));
    }
    list
}

/// Convert `&[(String, String)]` to a Python list of (str, str) tuples.
pub unsafe fn vec_tuple2_str_to_py(items: &[(String, String)]) -> PyObjectPtr {
    let list = PyList_New(items.len() as isize);
    for (i, (a, b)) in items.iter().enumerate() {
        let tuple = PyTuple_New(2);
        PyTuple_SetItem(tuple, 0, str_to_py(a));
        PyTuple_SetItem(tuple, 1, str_to_py(b));
        PyList_SetItem(list, i as isize, tuple);
    }
    list
}

/// Convert `&[(String, f64)]` to a Python list of (str, float) tuples.
pub unsafe fn vec_tuple2_str_f64_to_py(items: &[(String, f64)]) -> PyObjectPtr {
    let list = PyList_New(items.len() as isize);
    for (i, (a, value)) in items.iter().enumerate() {
        let tuple = PyTuple_New(2);
        PyTuple_SetItem(tuple, 0, str_to_py(a));
        PyTuple_SetItem(tuple, 1, f64_to_py(*value));
        PyList_SetItem(list, i as isize, tuple);
    }
    list
}

/// Convert `&[(String, String, f64)]` to a Python list of (str, str, float) tuples.
pub unsafe fn vec_tuple3_str_f64_to_py(items: &[(String, String, f64)]) -> PyObjectPtr {
    let list = PyList_New(items.len() as isize);
    for (i, (a, b, value)) in items.iter().enumerate() {
        let tuple = PyTuple_New(3);
        PyTuple_SetItem(tuple, 0, str_to_py(a));
        PyTuple_SetItem(tuple, 1, str_to_py(b));
        PyTuple_SetItem(tuple, 2, f64_to_py(*value));
        PyList_SetItem(list, i as isize, tuple);
    }
    list
}

// ---------------------------------------------------------------------------
// Safe wrappers — Set conversion
// ---------------------------------------------------------------------------

/// Convert a `HashSet<String>` to a Python set of str.
pub unsafe fn set_str_to_py(items: &HashSet<String>) -> PyObjectPtr {
    let set = PySet_New(ptr::null_mut());
    for item in items {
        let s = str_to_py(item);
        PySet_Add(set, s);
        Py_DecRef(s);
    }
    set
}

/// Convert a Python set/iterable of str to a `HashSet<String>`.
pub unsafe fn set_str_from_py(obj: PyObjectPtr) -> Option<HashSet<String>> {
    if obj.is_null() {
        return None;
    }
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
    PyErr_Clear();
    Some(result)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Boolean, Integer, None
// ---------------------------------------------------------------------------

/// Python `True` (new reference via PyBool_FromLong).
pub unsafe fn py_true() -> PyObjectPtr {
    PyBool_FromLong(1)
}

/// Python `False` (new reference via PyBool_FromLong).
pub unsafe fn py_false() -> PyObjectPtr {
    PyBool_FromLong(0)
}

/// Convert a Rust `bool` to Python bool.
pub unsafe fn bool_to_py(b: bool) -> PyObjectPtr {
    if b { py_true() } else { py_false() }
}

/// Convert a Rust `usize` to Python int.
pub unsafe fn usize_to_py(n: usize) -> PyObjectPtr {
    PyLong_FromLong(n as c_long)
}

/// Convert a Rust `f64` to Python float.
pub unsafe fn f64_to_py(value: f64) -> PyObjectPtr {
    PyFloat_FromDouble(value)
}

/// Convert a Python number to Rust `f64`.
pub unsafe fn f64_from_py(obj: PyObjectPtr) -> Option<f64> {
    if obj.is_null() {
        return None;
    }
    let float_obj = PyNumber_Float(obj);
    if float_obj.is_null() {
        PyErr_Clear();
        return None;
    }
    let value = PyFloat_AsDouble(float_obj);
    Py_DecRef(float_obj);
    Some(value)
}

/// Python `None` (new reference via Py_BuildValue with empty format).
pub unsafe fn py_none() -> PyObjectPtr {
    // Py_BuildValue("") returns a new reference to Py_None
    let fmt = b"\0".as_ptr() as *const c_char;
    Py_BuildValue(fmt)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Argument parsing
// ---------------------------------------------------------------------------

/// Extract one string argument from a Python args tuple.
///
/// Returns `None` (with no active exception) if the index is out of range
/// or if the item is not a str. This is important: callers that return an
/// error on `None` typically call API functions like `value_error_class()`
/// which must NOT be called while a Python exception is already active.
///
/// `PyTuple_GetItem` sets an `IndexError` on out-of-range access. We clear
/// it here so that the caller can safely set its own exception.
pub unsafe fn parse_arg_str(args: PyObjectPtr, index: isize) -> Option<String> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        // PyTuple_GetItem set an IndexError — clear it so the caller can
        // safely call any Python API function to set a different error.
        PyErr_Clear();
        return None;
    }
    str_from_py(arg)
}

/// Extract two string arguments from a Python args tuple.
pub unsafe fn parse_args_2str(args: PyObjectPtr) -> Option<(String, String)> {
    let a = parse_arg_str(args, 0)?;
    let b = parse_arg_str(args, 1)?;
    Some((a, b))
}

/// Extract one float argument from a Python args tuple.
pub unsafe fn parse_arg_f64(args: PyObjectPtr, index: isize) -> Option<f64> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    f64_from_py(arg)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Module and exception creation
// ---------------------------------------------------------------------------

/// Add an object to a Python module.
pub unsafe fn module_add_object(module: PyObjectPtr, name: &str, obj: PyObjectPtr) {
    let c_name = CString::new(name).expect("name must not contain NUL");
    PyModule_AddObject(module, c_name.as_ptr(), obj);
}

/// Convert a `BTreeMap<String, f64>` to a Python dict[str, float].
pub unsafe fn map_str_f64_to_py(items: &BTreeMap<String, f64>) -> PyObjectPtr {
    let dict = PyDict_New();
    for (key, value) in items {
        let py_key = str_to_py(key);
        let py_value = f64_to_py(*value);
        PyDict_SetItem(dict, py_key, py_value);
        Py_DecRef(py_key);
        Py_DecRef(py_value);
    }
    dict
}

/// Create a new exception class.
pub unsafe fn new_exception(
    module_name: &str,
    exc_name: &str,
    base: PyObjectPtr,
) -> PyObjectPtr {
    let full_name = format!("{}.{}", module_name, exc_name);
    let c_name = CString::new(full_name).expect("name must not contain NUL");
    PyErr_NewException(c_name.as_ptr(), base, ptr::null_mut())
}

/// Set a Python exception with a message.
pub unsafe fn set_error(exc_type: PyObjectPtr, msg: &str) {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error)").unwrap());
    PyErr_SetString(exc_type, c_msg.as_ptr());
}

/// Get a built-in exception class by name from the builtins module.
unsafe fn get_builtin_exception(name: &str) -> PyObjectPtr {
    let builtins_name = CString::new("builtins").unwrap();
    let builtins = PyImport_ImportModule(builtins_name.as_ptr());
    let exc_name = CString::new(name).unwrap();
    let exc = PyObject_GetAttrString(builtins, exc_name.as_ptr());
    Py_DecRef(builtins);
    exc
}

/// Get the built-in Exception class.
pub unsafe fn exception_class() -> PyObjectPtr {
    get_builtin_exception("Exception")
}

/// Get the built-in TypeError class.
///
/// Use for: wrong number of arguments, wrong argument type.
/// Python convention: `TypeError` is raised for signature violations
/// (e.g. "takes 1 argument (0 given)", "argument must be str not int").
pub unsafe fn type_error_class() -> PyObjectPtr {
    get_builtin_exception("TypeError")
}

/// Get the built-in ValueError class.
pub unsafe fn value_error_class() -> PyObjectPtr {
    get_builtin_exception("ValueError")
}

/// Get the built-in RuntimeError class.
pub unsafe fn runtime_error_class() -> PyObjectPtr {
    get_builtin_exception("RuntimeError")
}

/// Get the built-in IndexError class.
pub unsafe fn index_error_class() -> PyObjectPtr {
    get_builtin_exception("IndexError")
}

// ---------------------------------------------------------------------------
// Sentinel value for method/slot arrays
// ---------------------------------------------------------------------------

/// A null PyMethodDef — used as the sentinel at the end of method arrays.
pub const fn method_def_sentinel() -> PyMethodDef {
    PyMethodDef {
        ml_name: ptr::null(),
        ml_meth: None,
        ml_flags: 0,
        ml_doc: ptr::null(),
    }
}

/// A null PyType_Slot — used as the sentinel at the end of slot arrays.
pub const fn type_slot_sentinel() -> PyType_Slot {
    PyType_Slot {
        slot: 0,
        pfunc: ptr::null_mut(),
    }
}
