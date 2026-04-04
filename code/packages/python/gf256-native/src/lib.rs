// lib.rs -- GF(256) Python extension using python-bridge
// =======================================================
//
// Native extension wrapping the Rust `gf256` crate for Python via our
// zero-dependency python-bridge. Like polynomial-native, this is a
// **module-level API** of free functions (no class), since GF(256) operations
// are on primitive `u8` values, not object instances.
//
// # GF(256) Background
//
// GF(2^8) is a finite field with 256 elements. Elements are bytes (0–255).
// Arithmetic is very different from ordinary integer arithmetic:
//
//   - ADD = XOR (no carries, since 1 + 1 = 0 in GF(2))
//   - SUBTRACT = XOR (same as add, since -1 = 1 in characteristic 2)
//   - MULTIPLY = lookup table (log/antilog tables built at first use)
//   - DIVIDE = multiply by the multiplicative inverse
//
// # Design
//
// All GF(256) functions take Python `int` arguments (0–255) and return
// Python `int` values. We validate the range on entry to give clear error
// messages.
//
// # Panic Handling
//
// The `divide` function panics if `b == 0`. The `inverse` function panics if
// `a == 0`. We catch these via `std::panic::catch_unwind` and raise `ValueError`.
//
// # Module Constants
//
// We expose `ZERO`, `ONE`, and `PRIMITIVE_POLYNOMIAL` as module-level constants
// by adding them to the module after creation.

use std::ffi::{c_char, c_int, c_long, CString};
use std::ptr;
use std::sync::OnceLock;

use python_bridge::*;

// ---------------------------------------------------------------------------
// Additional CPython extern declarations not in python-bridge
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
extern "C" {
    // Integer operations: extract Python int as a C long.
    // Return type is c_long (i64 on Linux/macOS, i32 on Windows) matching the CPython ABI.
    fn PyLong_AsLong(obj: PyObjectPtr) -> c_long;

    // Error checking.
    fn PyErr_Occurred() -> PyObjectPtr;

    // Module attribute setting.
    // value type is c_long to match CPython's `long value` parameter (i32 on Windows).
    fn PyModule_AddIntConstant(
        module: PyObjectPtr,
        name: *const c_char,
        value: c_long,
    ) -> c_int;
}
// NOTE: PyLong_Check is intentionally NOT declared here.
//
// In Python 3.12+, PyLong_Check is a `static inline` function in cpython/longobject.h,
// NOT an exported symbol in libpython. Declaring it as extern "C" compiles fine but
// produces an `undefined symbol: PyLong_Check` LoadError at runtime on Linux/macOS.
//
// Instead we use PyLong_AsLong and check PyErr_Occurred() to detect type errors:
// if PyLong_AsLong returns -1 and an error is set, the object is not an integer.
// PyFloat_Check has the same issue and is replaced with PyFloat_AsDouble + error check.

// ---------------------------------------------------------------------------
// Helper: extract a GF(256) element (u8) from a Python int argument
// ---------------------------------------------------------------------------
//
// GF(256) elements are bytes: integers in the range [0, 255].
// We extract the value from a Python int and validate the range.
//
// Returns `None` and sets a Python exception on:
//   - null pointer (missing argument)
//   - non-integer type
//   - value out of range [0, 255]

unsafe fn extract_u8(obj: PyObjectPtr, arg_name: &str) -> Option<u8> {
    if obj.is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            &format!("argument '{}' is required", arg_name),
        );
        return None;
    }

    // Try integer extraction. PyLong_AsLong returns -1 and sets a TypeError
    // if obj is not an integer (or any subclass). Clearing errors first
    // ensures PyErr_Occurred() reflects only this call.
    PyErr_Clear();
    let val = PyLong_AsLong(obj);
    if val == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        set_error(
            value_error_class(),
            &format!("argument '{}' must be an integer (0-255)", arg_name),
        );
        return None;
    }

    if val < 0 || val > 255 {
        set_error(
            value_error_class(),
            &format!(
                "argument '{}' must be in range 0-255, got {}",
                arg_name, val
            ),
        );
        return None;
    }

    Some(val as u8)
}

// ---------------------------------------------------------------------------
// Leaked CString helper
// ---------------------------------------------------------------------------

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL bytes").into_raw()
}

// ---------------------------------------------------------------------------
// Module methods: free functions exposed to Python
// ---------------------------------------------------------------------------

// -- add(a: int, b: int) -> int -----------------------------------------------
//
// Add two GF(256) elements. In GF(256), addition is XOR (no carries).
//
// Example: add(0x53, 0xCA) = 0x53 ^ 0xCA = 0x99

unsafe extern "C" fn gf_add(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let a_obj = PyTuple_GetItem(args, 0);
    let b_obj = PyTuple_GetItem(args, 1);
    let a = match extract_u8(a_obj, "a") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let b = match extract_u8(b_obj, "b") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = gf256::add(a, b);
    PyLong_FromLong(result as c_long)
}

// -- subtract(a: int, b: int) -> int ------------------------------------------
//
// Subtract two GF(256) elements. In GF(256) characteristic 2, subtraction
// equals addition: a - b = a XOR b.

unsafe extern "C" fn gf_subtract(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let a_obj = PyTuple_GetItem(args, 0);
    let b_obj = PyTuple_GetItem(args, 1);
    let a = match extract_u8(a_obj, "a") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let b = match extract_u8(b_obj, "b") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = gf256::subtract(a, b);
    PyLong_FromLong(result as c_long)
}

// -- multiply(a: int, b: int) -> int ------------------------------------------
//
// Multiply two GF(256) elements using log/antilog tables.
// 0 * anything = 0.

unsafe extern "C" fn gf_multiply(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let a_obj = PyTuple_GetItem(args, 0);
    let b_obj = PyTuple_GetItem(args, 1);
    let a = match extract_u8(a_obj, "a") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let b = match extract_u8(b_obj, "b") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = gf256::multiply(a, b);
    PyLong_FromLong(result as c_long)
}

// -- divide(a: int, b: int) -> int --------------------------------------------
//
// Divide a by b in GF(256). Raises ValueError if b == 0.
// 0 / b = 0 for any non-zero b.

unsafe extern "C" fn gf_divide(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let a_obj = PyTuple_GetItem(args, 0);
    let b_obj = PyTuple_GetItem(args, 1);
    let a = match extract_u8(a_obj, "a") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let b = match extract_u8(b_obj, "b") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // Catch the Rust panic for division by zero.
    let result = std::panic::catch_unwind(move || gf256::divide(a, b));
    match result {
        Ok(val) => PyLong_FromLong(val as c_long),
        Err(_) => {
            set_error(value_error_class(), "GF(256): division by zero");
            ptr::null_mut()
        }
    }
}

// -- power(base: int, exp: int) -> int ----------------------------------------
//
// Raise a GF(256) element to a non-negative integer power.
// Uses log tables: base^exp = ALOG[(LOG[base] * exp) % 255].
//
// Special cases:
//   - 0^0 = 1 (by convention)
//   - 0^n = 0 for n > 0
//   - x^0 = 1 for any x

unsafe extern "C" fn gf_power(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let base_obj = PyTuple_GetItem(args, 0);
    let exp_obj = PyTuple_GetItem(args, 1);

    let base = match extract_u8(base_obj, "base") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // exp is a u32 (non-negative integer). We accept Python ints and clamp
    // to u32::MAX to avoid overflow in the Rust function.
    if exp_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "power requires two arguments: base, exp");
        return ptr::null_mut();
    }
    // PyLong_Check is `static inline` in CPython 3.12+ — NOT an exported symbol.
    // Use try-extract: PyLong_AsLong sets TypeError if obj is not an integer.
    // PyErr_Clear() first so PyErr_Occurred() reflects only this call.
    PyErr_Clear();
    let exp_val = PyLong_AsLong(exp_obj);
    if exp_val == -1 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "argument 'exp' must be a non-negative integer");
        return ptr::null_mut();
    }
    if exp_val < 0 {
        set_error(
            value_error_class(),
            &format!("argument 'exp' must be non-negative, got {}", exp_val),
        );
        return ptr::null_mut();
    }

    // Clamp to u32::MAX for very large exponents (edge case; not expected in practice).
    // Cast to i64 first so the comparison is the same type on all platforms
    // (c_long is i32 on Windows, i64 on Linux/macOS).
    let exp = (exp_val as i64).min(u32::MAX as i64) as u32;

    let result = gf256::power(base, exp);
    PyLong_FromLong(result as c_long)
}

// -- inverse(a: int) -> int ---------------------------------------------------
//
// Compute the multiplicative inverse of a in GF(256).
// Raises ValueError if a == 0 (zero has no multiplicative inverse).
// inverse(a) satisfies: multiply(a, inverse(a)) == 1.

unsafe extern "C" fn gf_inverse(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let a_obj = PyTuple_GetItem(args, 0);
    let a = match extract_u8(a_obj, "a") {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // Catch the Rust panic for inverse(0).
    let result = std::panic::catch_unwind(move || gf256::inverse(a));
    match result {
        Ok(val) => PyLong_FromLong(val as c_long),
        Err(_) => {
            set_error(
                value_error_class(),
                "GF(256): zero has no multiplicative inverse",
            );
            ptr::null_mut()
        }
    }
}

// ---------------------------------------------------------------------------
// Module initialization: PyInit_gf256_native
// ---------------------------------------------------------------------------
//
// Creates the `gf256_native` module, registers all 6 arithmetic functions,
// and adds 3 module-level constants:
//
//   ZERO                = 0      (additive identity)
//   ONE                 = 1      (multiplicative identity)
//   PRIMITIVE_POLYNOMIAL = 0x11D  (irreducible polynomial used for modular reduction)
//
// Method table:
//   0: add        (VARARGS)
//   1: subtract   (VARARGS)
//   2: multiply   (VARARGS)
//   3: divide     (VARARGS)
//   4: power      (VARARGS)
//   5: inverse    (VARARGS)
//   6: sentinel

// ---------------------------------------------------------------------------
// OnceLock-guarded method table and module definition
// ---------------------------------------------------------------------------
//
// Same rationale as polynomial-native: `static mut` re-initialized on every
// call is a data race if two threads import simultaneously. `OnceLock`
// guarantees the table is built exactly once in a thread-safe way.
//
// # Why the `SendSync` wrapper?
//
// `PyMethodDef` and `PyModuleDef` contain raw pointers, which Rust marks as
// `!Send + !Sync`. Our usage is safe: all pointed-to data are leaked CStrings
// (immutable for the process lifetime) and function pointers. We assert
// Send + Sync with a newtype wrapper.

struct SendSync<T>(T);
// SAFETY: pointed-to data is immutable after initialization.
unsafe impl<T> Send for SendSync<T> {}
unsafe impl<T> Sync for SendSync<T> {}

fn get_methods() -> &'static [PyMethodDef] {
    static METHODS: OnceLock<SendSync<Vec<PyMethodDef>>> = OnceLock::new();
    &METHODS.get_or_init(|| SendSync(vec![
            PyMethodDef {
                ml_name: cstr("add"),
                ml_meth: Some(gf_add),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "add(a, b) -> int\n\n\
                     Add two GF(256) elements. In characteristic 2, this is XOR:\n\
                     add(0x53, 0xCA) == 0x53 ^ 0xCA == 0x99",
                ),
            },
            PyMethodDef {
                ml_name: cstr("subtract"),
                ml_meth: Some(gf_subtract),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "subtract(a, b) -> int\n\n\
                     Subtract b from a in GF(256). Equals XOR (same as add in char 2).",
                ),
            },
            PyMethodDef {
                ml_name: cstr("multiply"),
                ml_meth: Some(gf_multiply),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "multiply(a, b) -> int\n\n\
                     Multiply two GF(256) elements using log/antilog tables.\n\
                     multiply(0, x) == multiply(x, 0) == 0.",
                ),
            },
            PyMethodDef {
                ml_name: cstr("divide"),
                ml_meth: Some(gf_divide),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "divide(a, b) -> int\n\n\
                     Divide a by b in GF(256). Raises ValueError if b == 0.",
                ),
            },
            PyMethodDef {
                ml_name: cstr("power"),
                ml_meth: Some(gf_power),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "power(base, exp) -> int\n\n\
                     Raise base to a non-negative integer power in GF(256).\n\
                     power(2, 255) == 1 (by Fermat's little theorem for finite fields).",
                ),
            },
            PyMethodDef {
                ml_name: cstr("inverse"),
                ml_meth: Some(gf_inverse),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "inverse(a) -> int\n\n\
                     Multiplicative inverse of a in GF(256): multiply(a, inverse(a)) == 1.\n\
                     Raises ValueError if a == 0.",
                ),
            },
            // Null sentinel: CPython uses this to know where the table ends.
            PyMethodDef {
                ml_name: ptr::null(),
                ml_meth: None,
                ml_flags: 0,
                ml_doc: ptr::null(),
            },
        ]))
    .0
}

fn get_module_def() -> &'static PyModuleDef {
    static MODULE_DEF: OnceLock<SendSync<PyModuleDef>> = OnceLock::new();
    &MODULE_DEF
        .get_or_init(|| SendSync(PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: cstr("gf256_native"),
        m_doc: cstr(
            "gf256_native -- Rust-backed GF(256) arithmetic for Python.\n\
             \n\
             GF(2^8) is a finite field with 256 elements (bytes 0-255).\n\
             Addition is XOR. Multiplication uses log/antilog tables.\n\
             \n\
             Constants:\n\
               ZERO                 = 0      (additive identity)\n\
               ONE                  = 1      (multiplicative identity)\n\
               PRIMITIVE_POLYNOMIAL = 0x11D  (irreducible reduction polynomial)\n\
             \n\
             All arguments must be integers in range [0, 255].",
        ),
        m_size: -1,
        m_methods: get_methods().as_ptr() as *mut PyMethodDef,
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    }))
    .0
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_gf256_native() -> PyObjectPtr {
    let module = PyModule_Create2(
        get_module_def() as *const PyModuleDef as *mut PyModuleDef,
        PYTHON_API_VERSION,
    );
    if module.is_null() {
        return ptr::null_mut();
    }

    // -- Add module-level constants -------------------------------------------
    //
    // These correspond to the Rust constants:
    //   gf256::ZERO                 = 0u8
    //   gf256::ONE                  = 1u8
    //   gf256::PRIMITIVE_POLYNOMIAL = 0x11Du16 = 285
    //
    // We use `cstr()` (which intentionally leaks the allocation) rather than
    // creating a temporary `CString` that would be dropped before
    // `PyModule_AddIntConstant` finishes copying the name into the module dict.
    // Using a dropped `CString` here was the prior bug: the pointer passed to
    // CPython pointed into freed memory, which is undefined behaviour.

    PyModule_AddIntConstant(module, cstr("ZERO"), gf256::ZERO as c_long);
    PyModule_AddIntConstant(module, cstr("ONE"), gf256::ONE as c_long);
    PyModule_AddIntConstant(module, cstr("PRIMITIVE_POLYNOMIAL"), gf256::PRIMITIVE_POLYNOMIAL as c_long);

    module
}
