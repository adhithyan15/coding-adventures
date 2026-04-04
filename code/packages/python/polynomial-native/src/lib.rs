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

use python_bridge::*;

// ---------------------------------------------------------------------------
// Additional CPython extern declarations not in python-bridge
// ---------------------------------------------------------------------------
//
// We need a few extra C API functions for float handling and list type
// checking, which are not in the python-bridge re-exports.

#[allow(non_snake_case)]
extern "C" {
    // Float: Python float object functions
    fn PyFloat_AsDouble(obj: PyObjectPtr) -> f64;
    fn PyFloat_FromDouble(v: f64) -> PyObjectPtr;

    // Long (int): needed for fallback when list items are Python ints
    fn PyLong_AsLong(obj: PyObjectPtr) -> i64;

    // Error checking
    fn PyErr_Occurred() -> PyObjectPtr;

    // Type checking
    fn PyList_Check(obj: PyObjectPtr) -> c_int;
    fn PyFloat_Check(obj: PyObjectPtr) -> c_int;
    fn PyLong_Check(obj: PyObjectPtr) -> c_int;
}

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
    if obj.is_null() || PyList_Check(obj) == 0 {
        set_error(
            value_error_class(),
            "argument must be a list of floats (e.g. [1.0, 2.0, 3.0])",
        );
        return None;
    }

    let len = PyList_Size(obj);
    if len < 0 {
        PyErr_Clear();
        set_error(value_error_class(), "failed to get list size");
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

        // Accept Python float or int, converting both to f64.
        let val: f64 = if PyFloat_Check(item) != 0 {
            // Python float -> f64 directly
            let v = PyFloat_AsDouble(item);
            if v == -1.0 && !PyErr_Occurred().is_null() {
                PyErr_Clear();
                set_error(
                    value_error_class(),
                    &format!("failed to convert float at index {}", i),
                );
                return None;
            }
            v
        } else if PyLong_Check(item) != 0 {
            // Python int -> i64 -> f64 (for coefficients like 0, 1, -1)
            let v = PyLong_AsLong(item);
            if v == -1 && !PyErr_Occurred().is_null() {
                PyErr_Clear();
                set_error(
                    value_error_class(),
                    &format!("integer at index {} is out of range", i),
                );
                return None;
            }
            v as f64
        } else {
            set_error(
                value_error_class(),
                &format!(
                    "list element at index {} must be a float or int, not {}",
                    i, "another type"
                ),
            );
            return None;
        };

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

    // x can be a float or an int
    let x: f64 = if PyFloat_Check(x_obj) != 0 {
        let v = PyFloat_AsDouble(x_obj);
        if v == -1.0 && !PyErr_Occurred().is_null() {
            PyErr_Clear();
            set_error(value_error_class(), "x argument must be a float or int");
            return ptr::null_mut();
        }
        v
    } else if PyLong_Check(x_obj) != 0 {
        let v = PyLong_AsLong(x_obj);
        if v == -1 && !PyErr_Occurred().is_null() {
            PyErr_Clear();
            set_error(value_error_class(), "x argument is out of range");
            return ptr::null_mut();
        }
        v as f64
    } else {
        set_error(value_error_class(), "x argument must be a float or int");
        return ptr::null_mut();
    };

    let val = polynomial::evaluate(&poly, x);
    PyFloat_FromDouble(val)
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

#[no_mangle]
pub unsafe extern "C" fn PyInit_polynomial_native() -> PyObjectPtr {
    // -- Method table ---------------------------------------------------------
    //
    // CPython expects a null-terminated array of PyMethodDef. We use a static
    // mut because the array must outlive this function call (the module holds
    // a pointer to it). The array is initialized once and never changed.

    static mut METHODS: [PyMethodDef; 13] = [
        PyMethodDef {
            ml_name: ptr::null(),
            ml_meth: None,
            ml_flags: 0,
            ml_doc: ptr::null(),
        };
        13
    ];

    METHODS[0] = PyMethodDef {
        ml_name: cstr("normalize"),
        ml_meth: Some(poly_normalize),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("normalize(poly) -> list[float]\n\nStrip trailing near-zero coefficients."),
    };
    METHODS[1] = PyMethodDef {
        ml_name: cstr("degree"),
        ml_meth: Some(poly_degree),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("degree(poly) -> int\n\nReturn the degree of a polynomial (index of highest non-zero term)."),
    };
    METHODS[2] = PyMethodDef {
        ml_name: cstr("zero"),
        ml_meth: Some(poly_zero),
        ml_flags: METH_NOARGS,
        ml_doc: cstr("zero() -> list[float]\n\nReturn the zero polynomial [0.0]."),
    };
    METHODS[3] = PyMethodDef {
        ml_name: cstr("one"),
        ml_meth: Some(poly_one),
        ml_flags: METH_NOARGS,
        ml_doc: cstr("one() -> list[float]\n\nReturn the multiplicative identity polynomial [1.0]."),
    };
    METHODS[4] = PyMethodDef {
        ml_name: cstr("add"),
        ml_meth: Some(poly_add),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("add(a, b) -> list[float]\n\nAdd two polynomials."),
    };
    METHODS[5] = PyMethodDef {
        ml_name: cstr("subtract"),
        ml_meth: Some(poly_subtract),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("subtract(a, b) -> list[float]\n\nSubtract polynomial b from a."),
    };
    METHODS[6] = PyMethodDef {
        ml_name: cstr("multiply"),
        ml_meth: Some(poly_multiply),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("multiply(a, b) -> list[float]\n\nMultiply two polynomials via convolution."),
    };
    METHODS[7] = PyMethodDef {
        ml_name: cstr("divmod_poly"),
        ml_meth: Some(poly_divmod),
        ml_flags: METH_VARARGS,
        ml_doc: cstr(
            "divmod_poly(dividend, divisor) -> tuple[list[float], list[float]]\n\n\
             Polynomial long division. Returns (quotient, remainder).\n\
             Raises ValueError if divisor is the zero polynomial.",
        ),
    };
    METHODS[8] = PyMethodDef {
        ml_name: cstr("divide"),
        ml_meth: Some(poly_divide),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("divide(a, b) -> list[float]\n\nReturn quotient of a / b. Raises ValueError if b is zero."),
    };
    METHODS[9] = PyMethodDef {
        ml_name: cstr("modulo"),
        ml_meth: Some(poly_modulo),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("modulo(a, b) -> list[float]\n\nReturn remainder of a / b. Raises ValueError if b is zero."),
    };
    METHODS[10] = PyMethodDef {
        ml_name: cstr("evaluate"),
        ml_meth: Some(poly_evaluate),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("evaluate(poly, x) -> float\n\nEvaluate the polynomial at x using Horner's method."),
    };
    METHODS[11] = PyMethodDef {
        ml_name: cstr("gcd"),
        ml_meth: Some(poly_gcd),
        ml_flags: METH_VARARGS,
        ml_doc: cstr("gcd(a, b) -> list[float]\n\nGreatest common divisor of two polynomials (Euclidean algorithm)."),
    };
    METHODS[12] = method_def_sentinel();

    // -- Module definition ----------------------------------------------------
    //
    // The module definition must also be static (the module object holds a
    // pointer to it for its entire lifetime).

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

    MODULE_DEF.m_name = cstr("polynomial_native");
    MODULE_DEF.m_doc = cstr(
        "polynomial_native -- Rust-backed polynomial arithmetic for Python.\n\
         \n\
         Polynomials are represented as list[float] where index == degree:\n\
         [3.0, 0.0, 2.0] means 3 + 0x + 2x^2\n\
         \n\
         All operations return normalized polynomials (trailing zeros stripped).\n\
         Division functions raise ValueError if the divisor is zero.",
    );
    MODULE_DEF.m_methods = (&raw mut METHODS) as *mut PyMethodDef;

    PyModule_Create2(&raw mut MODULE_DEF, PYTHON_API_VERSION)
}
