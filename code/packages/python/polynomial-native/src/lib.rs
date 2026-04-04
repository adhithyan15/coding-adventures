// lib.rs -- Polynomial Python extension using python-bridge
// ==========================================================
//
// Native extension wrapping the Rust `polynomial` crate for Python via our
// zero-dependency python-bridge. Unlike bitset-native (which exposes a class),
// this extension exposes a **module-level API** of free functions, because
// polynomial operations take coefficient arrays as arguments, not object state.
//
// # Design
//
// The polynomial crate represents polynomials as `&[f64]` coefficient arrays
// where index equals degree (little-endian). We map:
//
//   Python list[float]  <-->  Rust &[f64]
//
// Every function:
//   1. Receives Python args tuple
//   2. Extracts list arguments and converts to Vec<f64>
//   3. Calls the Rust polynomial function
//   4. Converts the Vec<f64> result back to a Python list
//   5. Returns the Python list (or tuple for divmod_poly)
//
// # Panic Handling
//
// The `divmod`, `divide`, and `modulo` functions in polynomial panic if the
// divisor is zero. We catch panics via `std::panic::catch_unwind` and convert
// them to Python `ValueError`.
//
// # Module Entry Point
//
// `PyInit_polynomial_native()` registers 12 free functions into a module
// with no type objects. This is simpler than the bitset-native pattern.

use std::ffi::{c_char, c_int, CString};
use std::ptr;
use std::sync::OnceLock;

use python_bridge::*;

// ---------------------------------------------------------------------------
// Additional CPython extern declarations not in python-bridge
// ---------------------------------------------------------------------------
//
// We need a few extra C API functions for float handling and list type
// checking, which are not in the python-bridge re-exports.

#[allow(non_snake_case)]
extern "C" {
    // Float: Python float object functions.
    // PyFloat_AsDouble accepts Python int as well as float (calls __float__ protocol).
    fn PyFloat_AsDouble(obj: PyObjectPtr) -> f64;
    fn PyFloat_FromDouble(v: f64) -> PyObjectPtr;

    // Long (int): extract Python int as C long.
    fn PyLong_AsLong(obj: PyObjectPtr) -> i64;

    // Error checking
    fn PyErr_Occurred() -> PyObjectPtr;

}
// NOTE: PyFloat_Check, PyLong_Check, and PyList_Check are intentionally NOT declared here.
//
// In Python 3.12+, both are `static inline` in the CPython headers and are NOT
// exported symbols in libpython. Using them as extern "C" produces an
// `undefined symbol` LoadError at runtime.
//
// Replacement strategy:
//   - To accept both float and int: call PyFloat_AsDouble() which internally
//     calls __float__ on integers, returning the same value as float(int).
//     It returns -1.0 and sets TypeError if the object is not numeric.
//   - Check PyErr_Occurred() to distinguish "error" from "value is -1.0".
//   - To check for list: call PyList_Size() directly. It validates the type
//     internally (calls PyErr_BadInternalCall and returns -1 for non-lists).
//     Clear the error and set our own ValueError for a clean user message.

// ---------------------------------------------------------------------------
// Helper: convert a Python list of floats to Vec<f64>
// ---------------------------------------------------------------------------
//
// Accepts either a Python `list` of `float` or `int` values. Python users
// often pass integer literals like `[1, 0, 1]` to represent polynomials;
// we accept both for ergonomics.
//
// Returns `None` and sets a Python exception if:
//   - The object is not a list
//   - Any element is neither float nor int
//
// Important: this function MUST NOT be called while a Python exception is
// already active, because we call API functions (like value_error_class())
// that behave incorrectly when an exception is set.

unsafe fn py_list_to_vec_f64(obj: PyObjectPtr) -> Option<Vec<f64>> {
    // Must be a list. We don't accept tuples or other iterables here
    // to keep the interface explicit and well-typed.
    //
    // We cannot use PyList_Check() because in Python 3.12+ it is `static inline`
    // in the CPython headers and is NOT exported by libpython — using it as
    // extern "C" produces an `undefined symbol` LoadError at runtime.
    //
    // Instead, call PyList_Size() directly: it validates that the argument is
    // a list internally (returning -1 + setting SystemError for non-lists).
    // We clear that error and replace it with our own ValueError.
    if obj.is_null() {
        set_error(
            value_error_class(),
            "argument must be a list of floats (e.g. [1.0, 2.0, 3.0])",
        );
        return None;
    }

    PyErr_Clear();
    let len = PyList_Size(obj);
    if len < 0 {
        // PyList_Size returns -1 for non-list objects (sets SystemError).
        // Replace with a user-friendly ValueError.
        PyErr_Clear();
        set_error(
            value_error_class(),
            "argument must be a list of floats (e.g. [1.0, 2.0, 3.0])",
        );
        return None;
    }

    let mut result = Vec::with_capacity(len as usize);

    for i in 0..len {
        // PyList_GetItem returns a borrowed reference (no decref needed).
        let item = PyList_GetItem(obj, i);
        if item.is_null() {
            PyErr_Clear();
            set_error(
                value_error_class(),
                &format!("failed to get list item at index {}", i),
            );
            return None;
        }

        // Accept Python float or int, converting both to f64 via PyFloat_AsDouble.
        // PyFloat_AsDouble calls __float__ internally, so Python ints are accepted
        // without needing PyLong_Check (which is a static inline in Python 3.12+).
        PyErr_Clear();
        let val = PyFloat_AsDouble(item);
        if val == -1.0 && !PyErr_Occurred().is_null() {
            PyErr_Clear();
            set_error(
                value_error_class(),
                &format!(
                    "list element at index {} must be a float or int",
                    i
                ),
            );
            return None;
        }
        let val: f64 = val;

        result.push(val);
    }

    Some(result)
}

// ---------------------------------------------------------------------------
// Helper: convert Vec<f64> to a Python list of floats
// ---------------------------------------------------------------------------
//
// Creates a new Python list. Each Rust f64 becomes a Python float.
// The resulting list has the same length as the input slice.

unsafe fn vec_f64_to_py_list(v: &[f64]) -> PyObjectPtr {
    let list = PyList_New(v.len() as isize);
    if list.is_null() {
        return ptr::null_mut();
    }
    for (i, &val) in v.iter().enumerate() {
        // PyFloat_FromDouble returns a new reference.
        // PyList_SetItem steals the reference — no manual Py_DecRef needed.
        let py_float = PyFloat_FromDouble(val);
        if py_float.is_null() {
            // Release the partially-built list before returning null,
            // so the caller sees null and can propagate the error without
            // a reference leak.
            Py_DecRef(list);
            return ptr::null_mut();
        }
        PyList_SetItem(list, i as isize, py_float);
    }
    list
}

// ---------------------------------------------------------------------------
// Helper: parse two list arguments from the args tuple
// ---------------------------------------------------------------------------
//
// Most polynomial functions take two polynomial lists (a, b). This helper
// extracts both and returns (Vec<f64>, Vec<f64>) or None on error.

unsafe fn parse_two_polys(args: PyObjectPtr) -> Option<(Vec<f64>, Vec<f64>)> {
    let a_obj = PyTuple_GetItem(args, 0);
    if a_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "expected two list arguments");
        return None;
    }
    let b_obj = PyTuple_GetItem(args, 1);
    if b_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "expected two list arguments");
        return None;
    }
    let a = py_list_to_vec_f64(a_obj)?;
    let b = py_list_to_vec_f64(b_obj)?;
    Some((a, b))
}

// ---------------------------------------------------------------------------
// Leaked CString helper (method names need static lifetime)
// ---------------------------------------------------------------------------
//
// We "leak" CStrings so they live for the entire process lifetime, which
// is exactly what the PyMethodDef array requires. Each call to cstr()
// allocates once and leaks intentionally.

fn cstr(s: &str) -> *const c_char {
    CString::new(s).expect("no NUL bytes").into_raw()
}

// ---------------------------------------------------------------------------
// Module methods: free functions exposed to Python
// ---------------------------------------------------------------------------
//
// Each function follows the standard Python C extension calling convention:
//   - `_module`: the module object (unused for module-level functions)
//   - `args`: a Python tuple of positional arguments
//
// The signature type matches PyMethodDef.ml_meth:
//   unsafe extern "C" fn(PyObjectPtr, PyObjectPtr) -> PyObjectPtr

// -- normalize(poly: list[float]) -> list[float] ----------------------------
//
// Strip trailing near-zero coefficients from a polynomial.
// `normalize([1.0, 0.0, 0.0])` returns `[1.0]`.
// `normalize([0.0])` returns `[]` (the zero polynomial).

unsafe extern "C" fn poly_normalize(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let poly_obj = PyTuple_GetItem(args, 0);
    if poly_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "normalize requires one list argument");
        return ptr::null_mut();
    }
    let poly = match py_list_to_vec_f64(poly_obj) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = polynomial::normalize(&poly);
    vec_f64_to_py_list(&result)
}

// -- degree(poly: list[float]) -> int ----------------------------------------
//
// Return the degree of the polynomial (index of the highest non-zero
// coefficient). Returns 0 for the zero polynomial.

unsafe extern "C" fn poly_degree(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let poly_obj = PyTuple_GetItem(args, 0);
    if poly_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "degree requires one list argument");
        return ptr::null_mut();
    }
    let poly = match py_list_to_vec_f64(poly_obj) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let d = polynomial::degree(&poly);
    usize_to_py(d)
}

// -- zero() -> list[float] ---------------------------------------------------
//
// Return the zero polynomial: `[0.0]`. The additive identity.

unsafe extern "C" fn poly_zero(_module: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let result = polynomial::zero();
    vec_f64_to_py_list(&result)
}

// -- one() -> list[float] ----------------------------------------------------
//
// Return the multiplicative identity polynomial: `[1.0]`.

unsafe extern "C" fn poly_one(_module: PyObjectPtr, _args: PyObjectPtr) -> PyObjectPtr {
    let result = polynomial::one();
    vec_f64_to_py_list(&result)
}

// -- add(a, b) -> list[float] ------------------------------------------------
//
// Add two polynomials coefficient-by-coefficient.
// Missing high-degree terms are treated as 0.

unsafe extern "C" fn poly_add(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = polynomial::add(&a, &b);
    vec_f64_to_py_list(&result)
}

// -- subtract(a, b) -> list[float] -------------------------------------------
//
// Subtract polynomial b from a coefficient-by-coefficient.

unsafe extern "C" fn poly_subtract(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = polynomial::subtract(&a, &b);
    vec_f64_to_py_list(&result)
}

// -- multiply(a, b) -> list[float] -------------------------------------------
//
// Multiply two polynomials via polynomial convolution.
// Result degree = deg(a) + deg(b).

unsafe extern "C" fn poly_multiply(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = polynomial::multiply(&a, &b);
    vec_f64_to_py_list(&result)
}

// -- divmod_poly(dividend, divisor) -> tuple[list[float], list[float]] -------
//
// Polynomial long division: returns (quotient, remainder).
// Raises ValueError if divisor is the zero polynomial.
//
// We catch the Rust panic from polynomial::divmod (which panics on zero
// divisor) using std::panic::catch_unwind. This converts a Rust panic into
// a Python ValueError rather than aborting the Python process.

unsafe extern "C" fn poly_divmod(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (dividend, divisor) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // Clone for the closure (which needs ownership, not borrows).
    let dividend_clone = dividend.clone();
    let divisor_clone = divisor.clone();

    // catch_unwind prevents the Rust panic from crossing the FFI boundary and
    // crashing the Python interpreter. A panic here means division by zero.
    let result = std::panic::catch_unwind(move || {
        polynomial::divmod(&dividend_clone, &divisor_clone)
    });

    match result {
        Ok((quot, rem)) => {
            // Build a Python 2-tuple: (quotient_list, remainder_list)
            let quot_list = vec_f64_to_py_list(&quot);
            let rem_list = vec_f64_to_py_list(&rem);

            if quot_list.is_null() || rem_list.is_null() {
                // Cleanup on allocation failure
                if !quot_list.is_null() {
                    Py_DecRef(quot_list);
                }
                if !rem_list.is_null() {
                    Py_DecRef(rem_list);
                }
                return ptr::null_mut();
            }

            let tuple = PyTuple_New(2);
            if tuple.is_null() {
                Py_DecRef(quot_list);
                Py_DecRef(rem_list);
                return ptr::null_mut();
            }

            // PyTuple_SetItem steals the reference to quot_list and rem_list.
            PyTuple_SetItem(tuple, 0, quot_list);
            PyTuple_SetItem(tuple, 1, rem_list);
            tuple
        }
        Err(_) => {
            // The Rust polynomial crate panicked — divisor was zero.
            set_error(
                value_error_class(),
                "polynomial division by zero: divisor is the zero polynomial",
            );
            ptr::null_mut()
        }
    }
}

// -- divide(a, b) -> list[float] ---------------------------------------------
//
// Return the quotient of a / b. Raises ValueError if b is zero.

unsafe extern "C" fn poly_divide(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    let a_clone = a.clone();
    let b_clone = b.clone();

    let result = std::panic::catch_unwind(move || polynomial::divide(&a_clone, &b_clone));

    match result {
        Ok(quot) => vec_f64_to_py_list(&quot),
        Err(_) => {
            set_error(
                value_error_class(),
                "polynomial division by zero: divisor is the zero polynomial",
            );
            ptr::null_mut()
        }
    }
}

// -- modulo(a, b) -> list[float] ---------------------------------------------
//
// Return the remainder of a / b. Raises ValueError if b is zero.

unsafe extern "C" fn poly_modulo(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    let a_clone = a.clone();
    let b_clone = b.clone();

    let result = std::panic::catch_unwind(move || polynomial::modulo(&a_clone, &b_clone));

    match result {
        Ok(rem) => vec_f64_to_py_list(&rem),
        Err(_) => {
            set_error(
                value_error_class(),
                "polynomial division by zero: divisor is the zero polynomial",
            );
            ptr::null_mut()
        }
    }
}

// -- evaluate(poly, x) -> float ----------------------------------------------
//
// Evaluate the polynomial at x using Horner's method. Returns a Python float.

unsafe extern "C" fn poly_evaluate(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let poly_obj = PyTuple_GetItem(args, 0);
    if poly_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "evaluate requires two arguments: poly, x");
        return ptr::null_mut();
    }
    let x_obj = PyTuple_GetItem(args, 1);
    if x_obj.is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "evaluate requires two arguments: poly, x");
        return ptr::null_mut();
    }

    let poly = match py_list_to_vec_f64(poly_obj) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    // x can be a float or an int — use PyFloat_AsDouble which calls __float__
    // for integer objects too. Avoids PyFloat_Check/PyLong_Check which are
    // static inline in Python 3.12+ and not exported as symbols.
    PyErr_Clear();
    let x_raw = PyFloat_AsDouble(x_obj);
    if x_raw == -1.0 && !PyErr_Occurred().is_null() {
        PyErr_Clear();
        set_error(value_error_class(), "x argument must be a float or int");
        return ptr::null_mut();
    }
    let x: f64 = x_raw;

    let val = polynomial::evaluate(&poly, x);
    // PyFloat_FromDouble can return null if the interpreter is out of memory.
    // A null return without an active exception is itself an indication of
    // allocation failure; treat it as a ValueError.
    let py_result = PyFloat_FromDouble(val);
    if py_result.is_null() {
        set_error(
            value_error_class(),
            "failed to create Python float (out of memory?)",
        );
        return ptr::null_mut();
    }
    py_result
}

// -- gcd(a, b) -> list[float] ------------------------------------------------
//
// Compute the GCD of two polynomials using the Euclidean algorithm.
// The result is the monic GCD (normalized).

unsafe extern "C" fn poly_gcd(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let (a, b) = match parse_two_polys(args) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };
    let result = polynomial::gcd(&a, &b);
    vec_f64_to_py_list(&result)
}

// ---------------------------------------------------------------------------
// Module initialization: PyInit_polynomial_native
// ---------------------------------------------------------------------------
//
// This is the entry point that Python calls when it does:
//   `import polynomial_native`
//
// It creates a module object, registers all 12 free functions into it, and
// returns the module. There is no type object (no class), just functions.
//
// The module name must match the shared library filename:
//   `libpolynomial_native.so` / `polynomial_native.so` / `polynomial_native.pyd`
//
// Method table layout:
//   0: normalize  (VARARGS)
//   1: degree     (VARARGS)
//   2: zero       (NOARGS)
//   3: one        (NOARGS)
//   4: add        (VARARGS)
//   5: subtract   (VARARGS)
//   6: multiply   (VARARGS)
//   7: divmod_poly(VARARGS)
//   8: divide     (VARARGS)
//   9: modulo     (VARARGS)
//  10: evaluate   (VARARGS)
//  11: gcd        (VARARGS)
//  12: sentinel   (null terminator)

// ---------------------------------------------------------------------------
// OnceLock-guarded method table and module definition
// ---------------------------------------------------------------------------
//
// The CPython C API requires that `PyMethodDef` arrays and `PyModuleDef`
// structs outlive the module object — effectively for the whole process
// lifetime. Using `static mut` and re-initializing on every `PyInit_*` call
// is unsound: if two threads import the module simultaneously the writes race.
//
// The fix: use `std::sync::OnceLock` so the table is built exactly once in a
// thread-safe manner.
//
// # Why the `SendSync` wrapper?
//
// `PyMethodDef` and `PyModuleDef` contain raw pointers (`*const c_char`,
// `*mut c_void`, etc.), which Rust conservatively marks as `!Send + !Sync`.
// However, our usage is safe:
//   - All `*const c_char` fields point to leaked `CString` allocations that
//     are never mutated or freed after init — so sharing them across threads
//     is fine.
//   - `ml_meth` function pointers are inherently thread-safe.
//   - `*mut c_void` slots are all null (we use `m_slots: null()`).
//
// We wrap the Vec/struct in a `SendSync` newtype that asserts these invariants.

struct SendSync<T>(T);
// SAFETY: See explanation above. The pointed-to data is immutable after init.
unsafe impl<T> Send for SendSync<T> {}
unsafe impl<T> Sync for SendSync<T> {}

fn get_methods() -> &'static [PyMethodDef] {
    static METHODS: OnceLock<SendSync<Vec<PyMethodDef>>> = OnceLock::new();
    &METHODS.get_or_init(|| SendSync(vec![
            PyMethodDef {
                ml_name: cstr("normalize"),
                ml_meth: Some(poly_normalize),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("normalize(poly) -> list[float]\n\nStrip trailing near-zero coefficients."),
            },
            PyMethodDef {
                ml_name: cstr("degree"),
                ml_meth: Some(poly_degree),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("degree(poly) -> int\n\nReturn the degree of a polynomial (index of highest non-zero term)."),
            },
            PyMethodDef {
                ml_name: cstr("zero"),
                ml_meth: Some(poly_zero),
                ml_flags: METH_NOARGS,
                ml_doc: cstr("zero() -> list[float]\n\nReturn the zero polynomial [0.0]."),
            },
            PyMethodDef {
                ml_name: cstr("one"),
                ml_meth: Some(poly_one),
                ml_flags: METH_NOARGS,
                ml_doc: cstr("one() -> list[float]\n\nReturn the multiplicative identity polynomial [1.0]."),
            },
            PyMethodDef {
                ml_name: cstr("add"),
                ml_meth: Some(poly_add),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("add(a, b) -> list[float]\n\nAdd two polynomials."),
            },
            PyMethodDef {
                ml_name: cstr("subtract"),
                ml_meth: Some(poly_subtract),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("subtract(a, b) -> list[float]\n\nSubtract polynomial b from a."),
            },
            PyMethodDef {
                ml_name: cstr("multiply"),
                ml_meth: Some(poly_multiply),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("multiply(a, b) -> list[float]\n\nMultiply two polynomials via convolution."),
            },
            PyMethodDef {
                ml_name: cstr("divmod_poly"),
                ml_meth: Some(poly_divmod),
                ml_flags: METH_VARARGS,
                ml_doc: cstr(
                    "divmod_poly(dividend, divisor) -> tuple[list[float], list[float]]\n\n\
                     Polynomial long division. Returns (quotient, remainder).\n\
                     Raises ValueError if divisor is the zero polynomial.",
                ),
            },
            PyMethodDef {
                ml_name: cstr("divide"),
                ml_meth: Some(poly_divide),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("divide(a, b) -> list[float]\n\nReturn quotient of a / b. Raises ValueError if b is zero."),
            },
            PyMethodDef {
                ml_name: cstr("modulo"),
                ml_meth: Some(poly_modulo),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("modulo(a, b) -> list[float]\n\nReturn remainder of a / b. Raises ValueError if b is zero."),
            },
            PyMethodDef {
                ml_name: cstr("evaluate"),
                ml_meth: Some(poly_evaluate),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("evaluate(poly, x) -> float\n\nEvaluate the polynomial at x using Horner's method."),
            },
            PyMethodDef {
                ml_name: cstr("gcd"),
                ml_meth: Some(poly_gcd),
                ml_flags: METH_VARARGS,
                ml_doc: cstr("gcd(a, b) -> list[float]\n\nGreatest common divisor of two polynomials (Euclidean algorithm)."),
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
        .get_or_init(|| {
            SendSync(PyModuleDef {
                m_base: PyModuleDef_Base {
                    ob_base: [0; std::mem::size_of::<usize>() * 2],
                    m_init: None,
                    m_index: 0,
                    m_copy: ptr::null_mut(),
                },
                m_name: cstr("polynomial_native"),
                m_doc: cstr(
                    "polynomial_native -- Rust-backed polynomial arithmetic for Python.\n\
                     \n\
                     Polynomials are represented as list[float] where index == degree:\n\
                     [3.0, 0.0, 2.0] means 3 + 0x + 2x^2\n\
                     \n\
                     All operations return normalized polynomials (trailing zeros stripped).\n\
                     Division functions raise ValueError if the divisor is zero.",
                ),
                m_size: -1,
                // `get_methods()` has already ensured the Vec is allocated and pinned
                // in the OnceLock; as_ptr() into it is stable for the process lifetime.
                m_methods: get_methods().as_ptr() as *mut PyMethodDef,
                m_slots: ptr::null_mut(),
                m_traverse: ptr::null_mut(),
                m_clear: ptr::null_mut(),
                m_free: ptr::null_mut(),
            })
        })
        .0
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_polynomial_native() -> PyObjectPtr {
    // Both the method table and the module definition are now initialized at
    // most once, regardless of how many threads race to import this module.
    PyModule_Create2(
        get_module_def() as *const PyModuleDef as *mut PyModuleDef,
        PYTHON_API_VERSION,
    )
}
