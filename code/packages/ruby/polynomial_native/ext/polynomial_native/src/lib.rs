// lib.rs -- Polynomial Ruby native extension using ruby-bridge
// ============================================================
//
// This is a Ruby C extension written in Rust. It wraps the `polynomial` crate's
// pure functions and exposes them to Ruby as module-level functions under:
//
//   CodingAdventures::PolynomialNative
//
// # Architecture
//
// 1. `Init_polynomial_native()` is called by Ruby when the .so is loaded
// 2. We define the module hierarchy `CodingAdventures::PolynomialNative`
// 3. We define module functions using `rb_define_module_function`
// 4. Each function marshals Ruby Arrays ↔ Vec<f64> and Ruby Float ↔ f64
//
// # Module Functions vs Instance Methods
//
// Unlike `bitset_native` which defines a class with instance methods,
// `polynomial` is a collection of pure functions with no stateful objects.
// We use `define_module_function_raw` which calls Ruby's
// `rb_define_module_function`. This makes each function callable in two ways:
//
//   CodingAdventures::PolynomialNative.normalize([1.0, 0.0])   # module method
//   include CodingAdventures::PolynomialNative
//   normalize([1.0, 0.0])                                       # free function
//
// # Polynomial Representation
//
// Polynomials are represented as Ruby Arrays of Floats, where the array
// index equals the degree of that term's coefficient. For example:
//
//   [3.0, 0.0, 2.0]  =  3 + 0·x + 2·x²  =  3 + 2x²
//   [1.0, 2.0, 3.0]  =  1 + 2x + 3x²
//   []                =  the zero polynomial
//
// # Float Extraction
//
// Ruby floats are heap-allocated objects in modern Ruby. We use `rb_num2dbl`
// from Ruby's C API to reliably extract the f64 value from a Ruby Float or
// any numeric Ruby object.
//
// # Panic Safety
//
// `polynomial::divmod` panics when the divisor is the zero polynomial.
// We catch such panics with `std::panic::catch_unwind` and convert them
// to Ruby `ArgumentError` exceptions, so Ruby callers get a proper error
// rather than a process abort.

use std::ffi::{c_char, c_void, CString};
use std::panic;

use ruby_bridge::VALUE;

// ---------------------------------------------------------------------------
// Additional Ruby C API functions not in ruby-bridge
// ---------------------------------------------------------------------------
//
// `rb_num2dbl` converts any Ruby Numeric (Integer, Float, Rational) to a
// C `double`. This is more robust than casting the VALUE directly.
//
// `rb_float_new` creates a Ruby Float object from a C double.
//
// `rb_path2class` looks up a Ruby class/module by name — used instead of
// extern statics for Windows MinGW + MSVC toolchain compatibility.

extern "C" {
    fn rb_num2dbl(val: VALUE) -> f64;
    fn rb_float_new(v: f64) -> VALUE;
    fn rb_path2class(path: *const c_char) -> VALUE;
}

/// Look up a Ruby class by its fully-qualified name.
///
/// We use `rb_path2class` instead of the extern statics (`rb_cObject`,
/// `rb_eArgumentError`) because the statics have linking issues on Windows
/// when using MinGW Ruby with MSVC's linker. Function-based lookups always work.
fn get_ruby_class(name: &str) -> VALUE {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    unsafe { rb_path2class(c_name.as_ptr()) }
}

/// Raise an ArgumentError with the given message. Does not return.
///
/// Uses `rb_path2class` for the error class to avoid Windows linker issues
/// with the `rb_eArgError` extern static.
fn raise_arg_error(msg: &str) -> ! {
    ruby_bridge::raise_error(get_ruby_class("ArgumentError"), msg)
}

// ---------------------------------------------------------------------------
// Ruby Array ↔ Vec<f64> conversion helpers
// ---------------------------------------------------------------------------
//
// Polynomials cross the Ruby/Rust boundary as Ruby Arrays of Floats.

/// Convert a Ruby Array of Floats to a Rust `Vec<f64>`.
///
/// Each element is extracted using `rb_num2dbl`, which handles Ruby Float,
/// Integer, and Rational objects transparently. If `val` is not an Array
/// (e.g., nil or a non-array), `rb_array_len` may return 0 or raise — the
/// caller is responsible for passing a valid Array.
///
/// # Layout
///
/// The resulting `Vec<f64>` follows the polynomial convention: `vec[i]` is
/// the coefficient of `xⁱ`. So `[3.0, 1.0, 2.0]` is `3 + x + 2x²`.
fn vec_from_rb(val: VALUE) -> Vec<f64> {
    let len = ruby_bridge::array_len(val);
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let elem = ruby_bridge::array_entry(val, i);
        result.push(unsafe { rb_num2dbl(elem) });
    }
    result
}

/// Convert a Rust `Vec<f64>` polynomial to a Ruby Array of Floats.
///
/// Each `f64` is wrapped with `rb_float_new`, which creates a Ruby Float
/// on the heap. The resulting Ruby Array uses the same index-equals-degree
/// convention as the Rust side.
fn vec_to_rb(poly: &[f64]) -> VALUE {
    let ary = ruby_bridge::array_new();
    for &coeff in poly {
        ruby_bridge::array_push(ary, unsafe { rb_float_new(coeff) });
    }
    ary
}

// ---------------------------------------------------------------------------
// Module functions — each has signature fn(self_val: VALUE, ...) -> VALUE
// ---------------------------------------------------------------------------
//
// When registered with `rb_define_module_function`, the first argument is
// the module itself (acting as `self` when called as `Module.method(...)`).
// We name this `_module` since we never use it.

// -- normalize(poly) -> Array<Float> ----------------------------------------
//
// Strip trailing near-zero coefficients from the polynomial.
//
//   CodingAdventures::PolynomialNative.normalize([1.0, 0.0, 0.0])  #=> [1.0]
//   CodingAdventures::PolynomialNative.normalize([0.0])             #=> []
extern "C" fn poly_normalize(_module: VALUE, poly_val: VALUE) -> VALUE {
    let poly = vec_from_rb(poly_val);
    let result = polynomial::normalize(&poly);
    vec_to_rb(&result)
}

// -- degree(poly) -> Integer -------------------------------------------------
//
// Return the degree of the polynomial (index of the highest non-zero term).
// Returns 0 for the zero polynomial (by convention).
//
//   CodingAdventures::PolynomialNative.degree([3.0, 0.0, 2.0])  #=> 2
//   CodingAdventures::PolynomialNative.degree([])                #=> 0
extern "C" fn poly_degree(_module: VALUE, poly_val: VALUE) -> VALUE {
    let poly = vec_from_rb(poly_val);
    let deg = polynomial::degree(&poly);
    ruby_bridge::usize_to_rb(deg)
}

// -- zero -> Array<Float> ---------------------------------------------------
//
// Return the zero polynomial `[0.0]` — the additive identity.
//
//   CodingAdventures::PolynomialNative.zero  #=> [0.0]
extern "C" fn poly_zero(_module: VALUE) -> VALUE {
    vec_to_rb(&polynomial::zero())
}

// -- one -> Array<Float> ----------------------------------------------------
//
// Return the unit polynomial `[1.0]` — the multiplicative identity.
//
//   CodingAdventures::PolynomialNative.one  #=> [1.0]
extern "C" fn poly_one(_module: VALUE) -> VALUE {
    vec_to_rb(&polynomial::one())
}

// -- add(a, b) -> Array<Float> ----------------------------------------------
//
// Add two polynomials term-by-term.
//
//   a = [1.0, 2.0, 3.0]  #  1 + 2x + 3x²
//   b = [4.0, 5.0]       #  4 + 5x
//   CodingAdventures::PolynomialNative.add(a, b)  #=> [5.0, 7.0, 3.0]
extern "C" fn poly_add(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);
    vec_to_rb(&polynomial::add(&a, &b))
}

// -- subtract(a, b) -> Array<Float> -----------------------------------------
//
// Subtract polynomial b from a term-by-term.
//
//   a = [5.0, 7.0, 3.0]
//   b = [1.0, 2.0, 3.0]
//   CodingAdventures::PolynomialNative.subtract(a, b)  #=> [4.0, 5.0]
extern "C" fn poly_subtract(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);
    vec_to_rb(&polynomial::subtract(&a, &b))
}

// -- multiply(a, b) -> Array<Float> -----------------------------------------
//
// Multiply two polynomials via polynomial convolution.
//
//   a = [1.0, 2.0]  #  1 + 2x
//   b = [3.0, 4.0]  #  3 + 4x
//   CodingAdventures::PolynomialNative.multiply(a, b)  #=> [3.0, 10.0, 8.0]
extern "C" fn poly_multiply(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);
    vec_to_rb(&polynomial::multiply(&a, &b))
}

// -- divmod_poly(dividend, divisor) -> [Array<Float>, Array<Float>] ----------
//
// Polynomial long division. Returns a Ruby Array of two Arrays:
// `[quotient, remainder]`, where `dividend = divisor * quotient + remainder`.
//
// Raises `ArgumentError` if the divisor is the zero polynomial.
//
//   dividend = [5.0, 1.0, 3.0, 2.0]   #  5 + x + 3x² + 2x³
//   divisor  = [2.0, 1.0]             #  2 + x
//   q, r = CodingAdventures::PolynomialNative.divmod_poly(dividend, divisor)
//   #=> q: [3.0, -1.0, 2.0],  r: [-1.0]
extern "C" fn poly_divmod(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);

    // Catch the panic from polynomial::divmod when divisor is zero.
    let result = panic::catch_unwind(|| polynomial::divmod(&a, &b));

    match result {
        Ok((quot, rem)) => {
            // Build and return the two-element Ruby Array [[quot...], [rem...]].
            let outer = ruby_bridge::array_new();
            ruby_bridge::array_push(outer, vec_to_rb(&quot));
            ruby_bridge::array_push(outer, vec_to_rb(&rem));
            outer
        }
        Err(_) => raise_arg_error("divmod_poly: divisor is the zero polynomial"),
    }
}

// -- divide(dividend, divisor) -> Array<Float> -------------------------------
//
// Return the quotient of polynomial long division.
// Raises `ArgumentError` if the divisor is the zero polynomial.
//
//   CodingAdventures::PolynomialNative.divide([3.0, -1.0], [1.0])  #=> [3.0, -1.0]
extern "C" fn poly_divide(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);

    let result = panic::catch_unwind(|| polynomial::divide(&a, &b));
    match result {
        Ok(quot) => vec_to_rb(&quot),
        Err(_) => raise_arg_error("divide: divisor is the zero polynomial"),
    }
}

// -- modulo(dividend, divisor) -> Array<Float> -------------------------------
//
// Return the remainder of polynomial long division.
// Raises `ArgumentError` if the divisor is the zero polynomial.
//
//   dividend = [1.0, -3.0, 2.0]  #  (x-1)(x-2)
//   divisor  = [1.0, -1.0]       #  (x-1)
//   CodingAdventures::PolynomialNative.modulo(dividend, divisor)  #=> []  (exact division)
extern "C" fn poly_modulo(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);

    let result = panic::catch_unwind(|| polynomial::modulo(&a, &b));
    match result {
        Ok(rem) => vec_to_rb(&rem),
        Err(_) => raise_arg_error("modulo: divisor is the zero polynomial"),
    }
}

// -- evaluate(poly, x) -> Float ----------------------------------------------
//
// Evaluate a polynomial at the given x value using Horner's method.
//
//   CodingAdventures::PolynomialNative.evaluate([3.0, 0.0, 1.0], 2.0)  #=> 7.0
//   # 3 + 0*2 + 1*4 = 7
extern "C" fn poly_evaluate(_module: VALUE, poly_val: VALUE, x_val: VALUE) -> VALUE {
    let poly = vec_from_rb(poly_val);
    let x = unsafe { rb_num2dbl(x_val) };
    let result = polynomial::evaluate(&poly, x);
    unsafe { rb_float_new(result) }
}

// -- gcd(a, b) -> Array<Float> -----------------------------------------------
//
// Compute the greatest common divisor of two polynomials via the Euclidean
// algorithm. The GCD is the highest-degree polynomial that divides both inputs.
//
//   a = [1.0, -3.0, 2.0]  # (x-1)(x-2)
//   b = [1.0, -1.0]       # (x-1)
//   CodingAdventures::PolynomialNative.gcd(a, b)  #=> [1.0, -1.0]  (x-1)
extern "C" fn poly_gcd(_module: VALUE, a_val: VALUE, b_val: VALUE) -> VALUE {
    let a = vec_from_rb(a_val);
    let b = vec_from_rb(b_val);
    vec_to_rb(&polynomial::gcd(&a, &b))
}

// ---------------------------------------------------------------------------
// Init_polynomial_native -- Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_polynomial_native` because Ruby derives
// the init function name from the .so filename. When Ruby loads
// `polynomial_native.so`, it calls `Init_polynomial_native()`.
//
// We set up the module hierarchy and bind all module functions here:
//
//   module CodingAdventures
//     module PolynomialNative
//       def self.normalize(poly)  ... end
//       def self.degree(poly)     ... end
//       def self.zero             ... end
//       def self.one              ... end
//       def self.add(a, b)        ... end
//       def self.subtract(a, b)   ... end
//       def self.multiply(a, b)   ... end
//       def self.divmod_poly(a,b) ... end
//       def self.divide(a, b)     ... end
//       def self.modulo(a, b)     ... end
//       def self.evaluate(p, x)   ... end
//       def self.gcd(a, b)        ... end
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_polynomial_native() {
    // -- Module hierarchy ---------------------------------------------------
    //
    // `CodingAdventures` is the top-level namespace used by all packages in
    // this monorepo. `PolynomialNative` is the specific module for this gem.
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let polynomial_native =
        ruby_bridge::define_module_under(coding_adventures, "PolynomialNative");

    // -- Fundamentals -------------------------------------------------------
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "normalize",
        poly_normalize as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "degree",
        poly_degree as *const c_void,
        1,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "zero",
        poly_zero as *const c_void,
        0,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "one",
        poly_one as *const c_void,
        0,
    );

    // -- Arithmetic operations ----------------------------------------------
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "add",
        poly_add as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "subtract",
        poly_subtract as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "multiply",
        poly_multiply as *const c_void,
        2,
    );

    // -- Division operations ------------------------------------------------
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "divmod_poly",
        poly_divmod as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "divide",
        poly_divide as *const c_void,
        2,
    );
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "modulo",
        poly_modulo as *const c_void,
        2,
    );

    // -- Evaluation ---------------------------------------------------------
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "evaluate",
        poly_evaluate as *const c_void,
        2,
    );

    // -- GCD ----------------------------------------------------------------
    ruby_bridge::define_module_function_raw(
        polynomial_native,
        "gcd",
        poly_gcd as *const c_void,
        2,
    );
}
