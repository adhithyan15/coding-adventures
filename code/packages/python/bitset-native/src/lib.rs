// lib.rs -- Bitset Python extension using python-bridge
// =====================================================
//
// Native extension wrapping the Rust `bitset` crate for Python via our
// zero-dependency python-bridge. This follows the same architecture as
// the directed-graph-native extension:
//
// 1. PyInit_bitset_native() creates the module
// 2. A PyTypeObject "Bitset" is created via PyType_FromSpec
// 3. Each instance holds a pointer to a Rust Bitset in its object body
// 4. Method calls extract the Bitset pointer, call Rust, marshal the result
// 5. A custom BitsetError exception is defined for error conditions
//
// # API Surface
//
// The native extension exposes the same interface as the pure Python bitset
// package so they are drop-in replacements for each other:
//
//   - Constructor: Bitset(size=0)
//   - Class methods: from_integer(value), from_binary_str(s)
//   - Single-bit: set(i), clear(i), test(i), toggle(i)
//   - Bulk ops: bitwise_and(other), bitwise_or(other), bitwise_xor(other),
//               bitwise_not(), and_not(other)
//   - Queries: popcount(), capacity(), any(), all(), none()
//   - Iteration: iter_set_bits()
//   - Conversion: to_integer(), to_binary_str()
//   - Protocols: __len__, __contains__, __repr__, __iter__, __eq__, __hash__
//   - Operators: &, |, ^, ~ (via nb_and, nb_or, nb_xor, nb_invert)

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr;

use bitset::Bitset;
use python_bridge::*;

// ---------------------------------------------------------------------------
// Slot numbers from CPython's typeslots.h (stable ABI)
// ---------------------------------------------------------------------------
//
// These numeric constants identify which "slot" a function pointer fills
// in the type object. They come from CPython's Include/typeslots.h.

const PY_TP_DEALLOC: c_int = 52;    // Py_tp_dealloc
const PY_TP_REPR: c_int = 66;       // Py_tp_repr
const PY_TP_HASH: c_int = 59;       // Py_tp_hash
const PY_TP_ITER: c_int = 62;       // Py_tp_iter
const PY_TP_METHODS: c_int = 64;    // Py_tp_methods
const PY_TP_NEW: c_int = 65;        // Py_tp_new
const PY_TP_RICHCOMPARE: c_int = 67; // Py_tp_richcompare
const PY_SQ_CONTAINS: c_int = 41;   // Py_sq_contains
const PY_SQ_LENGTH: c_int = 45;     // Py_sq_length
const PY_NB_AND: c_int = 8;         // Py_nb_and
const PY_NB_INVERT: c_int = 27;     // Py_nb_invert
const PY_NB_OR: c_int = 31;         // Py_nb_or
const PY_NB_XOR: c_int = 38;        // Py_nb_xor

// Rich comparison opcodes (from CPython object.h)
const PY_LT: c_int = 0;
const PY_EQ: c_int = 2;
const PY_NE: c_int = 3;

// METH_CLASS flag — CPython uses 0x0010 for class methods
const METH_CLASS: c_int = 0x0010;

// ---------------------------------------------------------------------------
// Additional CPython extern declarations not in python-bridge
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
extern "C" {
    fn PyType_GenericAlloc(type_obj: PyObjectPtr, nitems: isize) -> PyObjectPtr;
    fn PyObject_Free(ptr: *mut c_void);
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;
    fn PyLong_AsUnsignedLongLong(obj: PyObjectPtr) -> u64;
    fn PyObject_IsInstance(obj: PyObjectPtr, cls: PyObjectPtr) -> c_int;
    fn PyErr_Occurred() -> PyObjectPtr;
    fn PyTuple_Size(tuple: PyObjectPtr) -> isize;
}

// Py_NotImplemented is a singleton; we need to return it with an incref.
// But it's actually accessed via Py_BuildValue or a function. Let's use
// a helper to get Py_NotImplemented safely.
unsafe fn py_not_implemented() -> PyObjectPtr {
    // Use Py_BuildValue to get None, then... actually we need the real
    // Py_NotImplemented. The cleanest way on Windows is to use the
    // _Py_NotImplementedStruct approach, but that has dllimport issues.
    // Instead, we import builtins.NotImplemented.
    let builtins_name = CString::new("builtins").unwrap();
    let builtins = PyImport_ImportModule(builtins_name.as_ptr());
    let attr_name = CString::new("NotImplemented").unwrap();
    let not_impl = PyObject_GetAttrString(builtins, attr_name.as_ptr());
    Py_DecRef(builtins);
    not_impl // already a new reference from GetAttrString
}

// ---------------------------------------------------------------------------
// Instance layout: BitsetObject = PyObject_HEAD + bitset pointer
// ---------------------------------------------------------------------------
//
// Every Python object starts with ob_refcnt and ob_type (the "PyObject head").
// After that comes our custom field: a pointer to a heap-allocated Rust Bitset.

#[repr(C)]
struct BitsetObject {
    ob_refcnt: isize,
    ob_type: PyObjectPtr,
    inner: *mut Bitset,
}

// ---------------------------------------------------------------------------
// Exception class global
// ---------------------------------------------------------------------------

static mut BITSET_ERROR: PyObjectPtr = ptr::null_mut();

// ---------------------------------------------------------------------------
// Type object global (needed for creating new instances from Rust methods)
// ---------------------------------------------------------------------------

static mut BITSET_TYPE: PyObjectPtr = ptr::null_mut();

// ---------------------------------------------------------------------------
// Bitset access helpers
// ---------------------------------------------------------------------------

unsafe fn get_bitset(slf: PyObjectPtr) -> &'static Bitset {
    &*((slf as *mut BitsetObject).read().inner)
}

unsafe fn get_bitset_mut(slf: PyObjectPtr) -> &'static mut Bitset {
    &mut *(*(slf as *mut BitsetObject)).inner
}

// ---------------------------------------------------------------------------
// Helper: create a new Python Bitset object wrapping a Rust Bitset
// ---------------------------------------------------------------------------
//
// This is used by class methods (from_integer, from_binary_str) and
// bulk operations (bitwise_and, etc.) that need to return new Bitset objects.

unsafe fn wrap_bitset(bs: Bitset) -> PyObjectPtr {
    let obj = PyType_GenericAlloc(BITSET_TYPE, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut BitsetObject)).inner = Box::into_raw(Box::new(bs));
    obj
}

// ---------------------------------------------------------------------------
// Helper: parse a single integer argument from Python args tuple
// ---------------------------------------------------------------------------

unsafe fn parse_arg_usize(args: PyObjectPtr, index: isize) -> Option<usize> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        return None;
    }
    let val = PyLong_AsLong(arg);
    if val == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        return None;
    }
    if val < 0 {
        set_error(
            value_error_class(),
            &format!("expected non-negative integer, got {}", val),
        );
        return None;
    }
    Some(val as usize)
}

// ---------------------------------------------------------------------------
// Helper: check if a PyObject is our Bitset type
// ---------------------------------------------------------------------------

unsafe fn is_bitset(obj: PyObjectPtr) -> bool {
    if obj.is_null() || BITSET_TYPE.is_null() {
        return false;
    }
    PyObject_IsInstance(obj, BITSET_TYPE) == 1
}

// ---------------------------------------------------------------------------
// tp_new and tp_dealloc
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_new(
    type_obj: PyObjectPtr,
    args: PyObjectPtr,
    _kwargs: PyObjectPtr,
) -> PyObjectPtr {
    // Bitset(size=0) — optional positional integer argument
    let nargs = PyTuple_Size(args);
    let size = if nargs == 0 {
        0usize
    } else {
        match parse_arg_usize(args, 0) {
            Some(s) => s,
            None => return ptr::null_mut(),
        }
    };

    let obj = PyType_GenericAlloc(type_obj, 0);
    if obj.is_null() {
        return ptr::null_mut();
    }
    (*(obj as *mut BitsetObject)).inner = Box::into_raw(Box::new(Bitset::new(size)));
    obj
}

unsafe extern "C" fn bitset_dealloc(obj: PyObjectPtr) {
    let bsobj = obj as *mut BitsetObject;
    if !(*bsobj).inner.is_null() {
        let _ = Box::from_raw((*bsobj).inner);
        (*bsobj).inner = ptr::null_mut();
    }
    PyObject_Free(obj as *mut c_void);
}

// ---------------------------------------------------------------------------
// sq_length (__len__) and sq_contains (__contains__)
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_sq_length(slf: PyObjectPtr) -> isize {
    get_bitset(slf).len() as isize
}

unsafe extern "C" fn bitset_sq_contains(slf: PyObjectPtr, key: PyObjectPtr) -> c_int {
    // __contains__ checks if a given bit index is set.
    // Non-integer keys return 0 (not contained) rather than raising.
    let val = PyLong_AsLong(key);
    if val == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        return 0; // non-integer -> not contained
    }
    if val < 0 {
        return 0; // negative index -> not contained
    }
    if get_bitset(slf).test(val as usize) {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// tp_repr
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_repr(slf: PyObjectPtr) -> PyObjectPtr {
    let bs = get_bitset(slf);
    str_to_py(&format!("Bitset('{}')", bs.to_binary_str()))
}

// ---------------------------------------------------------------------------
// tp_richcompare (__eq__, __ne__)
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_richcompare(
    slf: PyObjectPtr,
    other: PyObjectPtr,
    op: c_int,
) -> PyObjectPtr {
    // Only support == and !=. For everything else, return NotImplemented.
    if op != PY_EQ && op != PY_NE {
        return py_not_implemented();
    }

    // If other is not a Bitset, return NotImplemented (lets Python try
    // the reflected operation or fall back to identity comparison).
    if !is_bitset(other) {
        return py_not_implemented();
    }

    let a = get_bitset(slf);
    let b = get_bitset(other);

    // Two bitsets are equal if they have the same len and the same bits.
    let equal = a.len() == b.len()
        && a.to_binary_str() == b.to_binary_str();

    match op {
        PY_EQ => bool_to_py(equal),
        PY_NE => bool_to_py(!equal),
        _ => py_not_implemented(),
    }
}

// ---------------------------------------------------------------------------
// tp_hash
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_hash(slf: PyObjectPtr) -> isize {
    // Hash based on len and binary string representation.
    // This is simple and correct: equal bitsets produce equal hashes.
    let bs = get_bitset(slf);
    let s = bs.to_binary_str();
    // Simple FNV-1a style hash
    let mut h: u64 = 14695981039346656037;
    for byte in s.as_bytes() {
        h ^= *byte as u64;
        h = h.wrapping_mul(1099511628211);
    }
    // Mix in the length
    h ^= bs.len() as u64;
    h = h.wrapping_mul(1099511628211);
    // Python expects -1 to mean "error", so avoid it
    let result = h as isize;
    if result == -1 { -2 } else { result }
}

// ---------------------------------------------------------------------------
// tp_iter (__iter__) — delegates to iter_set_bits which returns a list
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_iter(slf: PyObjectPtr) -> PyObjectPtr {
    // Return an iterator over set bit indices.
    // We build a Python list of ints, then return its iterator.
    let bs = get_bitset(slf);
    let bits: Vec<usize> = bs.iter_set_bits().collect();
    let list = PyList_New(bits.len() as isize);
    for (i, bit) in bits.iter().enumerate() {
        PyList_SetItem(list, i as isize, PyLong_FromLong(*bit as c_long));
    }
    let iter = PyObject_GetIter(list);
    Py_DecRef(list);
    iter
}

// ---------------------------------------------------------------------------
// Number protocol: nb_and, nb_or, nb_xor, nb_invert (for &, |, ^, ~)
// ---------------------------------------------------------------------------

unsafe extern "C" fn bitset_nb_and(a: PyObjectPtr, b: PyObjectPtr) -> PyObjectPtr {
    if !is_bitset(a) || !is_bitset(b) {
        return py_not_implemented();
    }
    wrap_bitset(get_bitset(a).and(get_bitset(b)))
}

unsafe extern "C" fn bitset_nb_or(a: PyObjectPtr, b: PyObjectPtr) -> PyObjectPtr {
    if !is_bitset(a) || !is_bitset(b) {
        return py_not_implemented();
    }
    wrap_bitset(get_bitset(a).or(get_bitset(b)))
}

unsafe extern "C" fn bitset_nb_xor(a: PyObjectPtr, b: PyObjectPtr) -> PyObjectPtr {
    if !is_bitset(a) || !is_bitset(b) {
        return py_not_implemented();
    }
    wrap_bitset(get_bitset(a).xor(get_bitset(b)))
}

unsafe extern "C" fn bitset_nb_invert(a: PyObjectPtr) -> PyObjectPtr {
    // nb_invert is a unary function: fn(self) -> result
    // But the slot signature for binary ops is fn(a, b). For unary ops
    // like invert, CPython passes (self, NULL) in some contexts, or
    // just (self) depending on the slot. nb_invert is actually a unary
    // slot with signature fn(self) -> PyObject*.
    wrap_bitset(get_bitset(a).not())
}

// ---------------------------------------------------------------------------
// Method implementations
// ---------------------------------------------------------------------------

// -- Single-bit operations --

unsafe extern "C" fn bitset_set(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let i = match parse_arg_usize(args, 0) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    get_bitset_mut(slf).set(i);
    py_none()
}

unsafe extern "C" fn bitset_clear(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let i = match parse_arg_usize(args, 0) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    get_bitset_mut(slf).clear(i);
    py_none()
}

unsafe extern "C" fn bitset_test(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let i = match parse_arg_usize(args, 0) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    bool_to_py(get_bitset(slf).test(i))
}

unsafe extern "C" fn bitset_toggle(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let i = match parse_arg_usize(args, 0) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    get_bitset_mut(slf).toggle(i);
    py_none()
}

// -- Bulk bitwise operations (return new Bitset) --

unsafe extern "C" fn bitset_bitwise_and(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if !is_bitset(other) {
        set_error(value_error_class(), "argument must be a Bitset");
        return ptr::null_mut();
    }
    wrap_bitset(get_bitset(slf).and(get_bitset(other)))
}

unsafe extern "C" fn bitset_bitwise_or(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if !is_bitset(other) {
        set_error(value_error_class(), "argument must be a Bitset");
        return ptr::null_mut();
    }
    wrap_bitset(get_bitset(slf).or(get_bitset(other)))
}

unsafe extern "C" fn bitset_bitwise_xor(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if !is_bitset(other) {
        set_error(value_error_class(), "argument must be a Bitset");
        return ptr::null_mut();
    }
    wrap_bitset(get_bitset(slf).xor(get_bitset(other)))
}

unsafe extern "C" fn bitset_bitwise_not(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    wrap_bitset(get_bitset(slf).not())
}

unsafe extern "C" fn bitset_and_not(slf: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let other = PyTuple_GetItem(args, 0);
    if !is_bitset(other) {
        set_error(value_error_class(), "argument must be a Bitset");
        return ptr::null_mut();
    }
    wrap_bitset(get_bitset(slf).and_not(get_bitset(other)))
}

// -- Counting and query --

unsafe extern "C" fn bitset_popcount(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    usize_to_py(get_bitset(slf).popcount())
}

unsafe extern "C" fn bitset_capacity(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    usize_to_py(get_bitset(slf).capacity())
}

unsafe extern "C" fn bitset_any(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_bitset(slf).any())
}

unsafe extern "C" fn bitset_all(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_bitset(slf).all())
}

unsafe extern "C" fn bitset_none_method(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    bool_to_py(get_bitset(slf).none())
}

// -- Iteration --

unsafe extern "C" fn bitset_iter_set_bits(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let bs = get_bitset(slf);
    let bits: Vec<usize> = bs.iter_set_bits().collect();
    let list = PyList_New(bits.len() as isize);
    for (i, bit) in bits.iter().enumerate() {
        PyList_SetItem(list, i as isize, PyLong_FromLong(*bit as c_long));
    }
    list
}

// -- Conversion --

unsafe extern "C" fn bitset_to_integer(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    // The pure Python version returns a Python int of arbitrary size.
    // The Rust Bitset::to_integer returns Option<u64> — None if more than
    // 64 bits are needed. But the Python API returns the full value.
    //
    // We need to reconstruct the integer from the binary string representation
    // for bitsets that don't fit in u64, to match the pure Python behavior.
    let bs = get_bitset(slf);
    let bin_str = bs.to_binary_str();
    if bin_str.is_empty() {
        return PyLong_FromLong(0);
    }

    // Use Python's int(binary_string, 2) to handle arbitrarily large values.
    // This is the simplest correct approach.
    let py_str = str_to_py(&bin_str);
    let builtins_name = CString::new("builtins").unwrap();
    let builtins = PyImport_ImportModule(builtins_name.as_ptr());
    let int_name = CString::new("int").unwrap();
    let int_func = PyObject_GetAttrString(builtins, int_name.as_ptr());
    Py_DecRef(builtins);

    // Call int(string, 2)
    let base = PyLong_FromLong(2);
    let call_args = PyTuple_New(2);
    PyTuple_SetItem(call_args, 0, py_str);
    PyTuple_SetItem(call_args, 1, base);

    // Use PyObject_Call
    extern "C" {
        fn PyObject_Call(
            callable: PyObjectPtr,
            args: PyObjectPtr,
            kwargs: PyObjectPtr,
        ) -> PyObjectPtr;
    }
    let result = PyObject_Call(int_func, call_args, ptr::null_mut());
    Py_DecRef(call_args);
    Py_DecRef(int_func);
    result
}

unsafe extern "C" fn bitset_to_binary_str(slf: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    str_to_py(&get_bitset(slf).to_binary_str())
}

// -- Class methods: from_integer, from_binary_str --

unsafe extern "C" fn bitset_from_integer(_cls: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    // from_integer(value) — accepts a non-negative Python int.
    // The Rust API accepts u128, but Python ints can be arbitrarily large.
    // For values that fit in u128, we use from_integer directly.
    // For larger values, we parse the binary string representation.
    let arg = PyTuple_GetItem(args, 0);
    if arg.is_null() {
        set_error(value_error_class(), "from_integer requires one argument");
        return ptr::null_mut();
    }

    // First check if it's negative by comparing with 0
    // Use Python's comparison: call arg < 0
    let zero = PyLong_FromLong(0);

    extern "C" {
        fn PyObject_RichCompareBool(a: PyObjectPtr, b: PyObjectPtr, op: c_int) -> c_int;
    }
    let is_negative = PyObject_RichCompareBool(arg, zero, PY_LT);
    Py_DecRef(zero);

    if is_negative == 1 {
        set_error(BITSET_ERROR, "from_integer requires a non-negative integer");
        return ptr::null_mut();
    }

    // Try to get the value. For values that fit in u64, use PyLong_AsUnsignedLongLong.
    // For larger values, convert through Python's bin() function.
    let val = PyLong_AsUnsignedLongLong(arg);
    if !PyErr_Occurred().is_null() {
        // OverflowError — value doesn't fit in u64. Use bin() approach.
        PyErr_Clear();

        // Call bin(arg) to get "0b..." string, then parse it.
        let builtins_name = CString::new("builtins").unwrap();
        let builtins = PyImport_ImportModule(builtins_name.as_ptr());
        let bin_name = CString::new("bin").unwrap();
        let bin_func = PyObject_GetAttrString(builtins, bin_name.as_ptr());
        Py_DecRef(builtins);

        let call_args = PyTuple_New(1);
        Py_IncRef(arg);
        PyTuple_SetItem(call_args, 0, arg);

        extern "C" {
            fn PyObject_Call(
                callable: PyObjectPtr,
                args: PyObjectPtr,
                kwargs: PyObjectPtr,
            ) -> PyObjectPtr;
        }
        let bin_result = PyObject_Call(bin_func, call_args, ptr::null_mut());
        Py_DecRef(call_args);
        Py_DecRef(bin_func);

        if bin_result.is_null() {
            return ptr::null_mut();
        }

        let bin_string = match str_from_py(bin_result) {
            Some(s) => s,
            None => {
                Py_DecRef(bin_result);
                return ptr::null_mut();
            }
        };
        Py_DecRef(bin_result);

        // bin() returns "0b..." — strip the prefix
        let binary_digits = &bin_string[2..];
        match Bitset::from_binary_str(binary_digits) {
            Ok(bs) => return wrap_bitset(bs),
            Err(e) => {
                set_error(BITSET_ERROR, &e.to_string());
                return ptr::null_mut();
            }
        }
    }

    // Value fits in u64. Use from_integer with u128 to match Rust API.
    wrap_bitset(Bitset::from_integer(val as u128))
}

unsafe extern "C" fn bitset_from_binary_str(
    _cls: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let s = match parse_arg_str(args, 0) {
        Some(s) => s,
        None => {
            set_error(
                value_error_class(),
                "from_binary_str requires one string argument",
            );
            return ptr::null_mut();
        }
    };
    match Bitset::from_binary_str(&s) {
        Ok(bs) => wrap_bitset(bs),
        Err(e) => {
            set_error(BITSET_ERROR, &e.to_string());
            ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Leaked CString helper (method tables need static lifetime)
// ---------------------------------------------------------------------------

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL").into_raw()
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn PyInit_bitset_native() -> PyObjectPtr {
    // -- Method table -------------------------------------------------------
    //
    // 20 methods + 1 sentinel = 21 entries
    //
    // Methods:
    //  0: set           (VARARGS)
    //  1: clear         (VARARGS)
    //  2: test          (VARARGS)
    //  3: toggle        (VARARGS)
    //  4: bitwise_and   (VARARGS)
    //  5: bitwise_or    (VARARGS)
    //  6: bitwise_xor   (VARARGS)
    //  7: bitwise_not   (NOARGS)
    //  8: and_not       (VARARGS)
    //  9: popcount      (NOARGS)
    // 10: capacity      (NOARGS)
    // 11: any           (NOARGS)
    // 12: all           (NOARGS)
    // 13: none          (NOARGS)
    // 14: iter_set_bits (NOARGS)
    // 15: to_integer    (NOARGS)
    // 16: to_binary_str (NOARGS)
    // 17: from_integer  (VARARGS | CLASS)
    // 18: from_binary_str (VARARGS | CLASS)
    // 19: sentinel

    static mut METHODS: [PyMethodDef; 20] = [
        PyMethodDef {
            ml_name: ptr::null(),
            ml_meth: None,
            ml_flags: 0,
            ml_doc: ptr::null(),
        }; 20
    ];

    METHODS[0] = PyMethodDef {
        ml_name: cstr("set"),
        ml_meth: Some(bitset_set),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("clear"),
        ml_meth: Some(bitset_clear),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("test"),
        ml_meth: Some(bitset_test),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("toggle"),
        ml_meth: Some(bitset_toggle),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("bitwise_and"),
        ml_meth: Some(bitset_bitwise_and),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("bitwise_or"),
        ml_meth: Some(bitset_bitwise_or),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("bitwise_xor"),
        ml_meth: Some(bitset_bitwise_xor),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("bitwise_not"),
        ml_meth: Some(bitset_bitwise_not),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[8] = PyMethodDef {
        ml_name: cstr("and_not"),
        ml_meth: Some(bitset_and_not),
        ml_flags: METH_VARARGS,
        ml_doc: ptr::null(),
    };
    METHODS[9] = PyMethodDef {
        ml_name: cstr("popcount"),
        ml_meth: Some(bitset_popcount),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[10] = PyMethodDef {
        ml_name: cstr("capacity"),
        ml_meth: Some(bitset_capacity),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[11] = PyMethodDef {
        ml_name: cstr("any"),
        ml_meth: Some(bitset_any),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[12] = PyMethodDef {
        ml_name: cstr("all"),
        ml_meth: Some(bitset_all),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[13] = PyMethodDef {
        ml_name: cstr("none"),
        ml_meth: Some(bitset_none_method),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[14] = PyMethodDef {
        ml_name: cstr("iter_set_bits"),
        ml_meth: Some(bitset_iter_set_bits),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[15] = PyMethodDef {
        ml_name: cstr("to_integer"),
        ml_meth: Some(bitset_to_integer),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[16] = PyMethodDef {
        ml_name: cstr("to_binary_str"),
        ml_meth: Some(bitset_to_binary_str),
        ml_flags: METH_NOARGS,
        ml_doc: ptr::null(),
    };
    METHODS[17] = PyMethodDef {
        ml_name: cstr("from_integer"),
        ml_meth: Some(bitset_from_integer),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    METHODS[18] = PyMethodDef {
        ml_name: cstr("from_binary_str"),
        ml_meth: Some(bitset_from_binary_str),
        ml_flags: METH_VARARGS | METH_CLASS,
        ml_doc: ptr::null(),
    };
    METHODS[19] = method_def_sentinel();

    // -- Type slots ----------------------------------------------------------
    //
    // Basic slots matching the directed-graph-native pattern, plus
    // richcompare, hash, iter, and number protocol slots.

    static mut SLOTS: [PyType_Slot; 14] = [
        PyType_Slot {
            slot: 0,
            pfunc: ptr::null_mut(),
        }; 14
    ];

    SLOTS[0] = PyType_Slot { slot: PY_TP_NEW, pfunc: bitset_new as *mut c_void };
    SLOTS[1] = PyType_Slot { slot: PY_TP_DEALLOC, pfunc: bitset_dealloc as *mut c_void };
    SLOTS[2] = PyType_Slot { slot: PY_TP_METHODS, pfunc: (&raw mut METHODS) as *mut c_void };
    SLOTS[3] = PyType_Slot { slot: PY_TP_REPR, pfunc: bitset_repr as *mut c_void };
    SLOTS[4] = PyType_Slot { slot: PY_TP_RICHCOMPARE, pfunc: bitset_richcompare as *mut c_void };
    SLOTS[5] = PyType_Slot { slot: PY_TP_HASH, pfunc: bitset_hash as *mut c_void };
    SLOTS[6] = PyType_Slot { slot: PY_TP_ITER, pfunc: bitset_iter as *mut c_void };
    SLOTS[7] = PyType_Slot { slot: PY_SQ_LENGTH, pfunc: bitset_sq_length as *mut c_void };
    SLOTS[8] = PyType_Slot { slot: PY_SQ_CONTAINS, pfunc: bitset_sq_contains as *mut c_void };
    SLOTS[9] = PyType_Slot { slot: PY_NB_AND, pfunc: bitset_nb_and as *mut c_void };
    SLOTS[10] = PyType_Slot { slot: PY_NB_OR, pfunc: bitset_nb_or as *mut c_void };
    SLOTS[11] = PyType_Slot { slot: PY_NB_XOR, pfunc: bitset_nb_xor as *mut c_void };
    SLOTS[12] = PyType_Slot { slot: PY_NB_INVERT, pfunc: bitset_nb_invert as *mut c_void };
    SLOTS[13] = type_slot_sentinel();

    // -- Type spec -----------------------------------------------------------
    static mut SPEC: PyType_Spec = PyType_Spec {
        name: ptr::null(),
        basicsize: 0,
        itemsize: 0,
        flags: 0,
        slots: ptr::null_mut(),
    };

    SPEC.name = cstr("bitset_native.Bitset");
    SPEC.basicsize = std::mem::size_of::<BitsetObject>() as c_int;
    SPEC.flags = PY_TPFLAGS_DEFAULT;
    SPEC.slots = (&raw mut SLOTS) as *mut PyType_Slot;

    let type_obj = PyType_FromSpec(&raw mut SPEC);
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    // Store the type object globally so wrap_bitset() and is_bitset() can use it
    BITSET_TYPE = type_obj;

    // -- Module definition ---------------------------------------------------
    static mut MODULE_DEF: PyModuleDef = PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: ptr::null(),
        m_doc: ptr::null(),
        m_size: -1,
        m_methods: ptr::null_mut(),
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    };
    MODULE_DEF.m_name = cstr("bitset_native");

    let module = PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION);
    if module.is_null() {
        return ptr::null_mut();
    }

    // -- Add class to module -------------------------------------------------
    Py_IncRef(type_obj);
    module_add_object(module, "Bitset", type_obj);

    // -- Create exception class ----------------------------------------------
    BITSET_ERROR = new_exception("bitset_native", "BitsetError", exception_class());
    Py_IncRef(BITSET_ERROR);
    module_add_object(module, "BitsetError", BITSET_ERROR);

    module
}
