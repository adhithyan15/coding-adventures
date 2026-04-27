//! # PolynomialNative — Perl XS extension wrapping Rust polynomial arithmetic
//!
//! This file implements a Perl XS extension that exposes polynomial arithmetic
//! to Perl scripts via DynaLoader.
//!
//! ## How Perl XS loading works
//!
//! When Perl encounters `use CodingAdventures::PolynomialNative;`, DynaLoader:
//! 1. Finds the shared library (`PolynomialNative.so` on the `@INC` path).
//! 2. `dlopen`s it and looks for `boot_CodingAdventures__PolynomialNative`.
//! 3. Calls the boot function, which uses `newXS` to register each XSUB.
//!
//! After boot, Perl knows about `CodingAdventures::PolynomialNative::add`,
//! `CodingAdventures::PolynomialNative::multiply`, etc.
//!
//! ## XS calling convention
//!
//! Each XSUB ("XS subroutine") has the signature `fn(*mut CV)`. Arguments
//! arrive on Perl's internal stack. We use `perl-bridge` helpers to read
//! them (`sv_to_f64`, `av_to_f64_vec`) and to create return SVs
//! (`f64_to_sv`, `f64_vec_to_av`).
//!
//! ## Polynomial representation
//!
//! Polynomials cross the Perl boundary as **array references** (arrayrefs):
//!
//! ```perl
//! my $poly = [3.0, 0.0, 1.0];  # 3 + 0·x + 1·x²
//! ```
//!
//! On the Rust side, an arrayref arrives as an SV* holding a reference to
//! an AV*. We dereference it to get the AV*, then use `av_to_f64_vec`.
//!
//! ## Simplified subset
//!
//! The full XS calling convention (dXSARGS, ST(n), XSRETURN) lives in Perl's
//! C headers. `perl-bridge` now exposes those pieces through a tiny shim, so
//! these XSUBs can use the host Perl's own stack macros safely, including
//! threaded builds.
//!
//! ## Note on xs_init! macro
//!
//! The `xs_init!` macro in perl-bridge requires `concat_idents`, which is
//! not in stable Rust. We write the boot function by hand instead.
//!
#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::{
    av_to_f64_vec, die, f64_to_sv, f64_vec_to_av, i64_to_sv, sv_to_f64, sv_to_i64,
    xs_boot_finish, xs_bootstrap, AV, CV, IV, SV, SvREFCNT_dec, newSViv, sv_2iv, sv_2nv,
};
use std::ffi::{c_char, c_int, CString};
use std::panic::catch_unwind;

// ---------------------------------------------------------------------------
// Declare newXS — not in perl-bridge, but exported by Perl's runtime
// ---------------------------------------------------------------------------
//
// `newXS` registers a C function as a Perl subroutine in the symbol table.
// It is the core of every XS boot function.
//
// Signature (simplified — real Perl has ithreads variants):
//   CV* newXS(const char *name, XSUBADDR_t subaddr, const char *filename)
// Returns a CV* (code value); we don't use the return value.

extern "C" {
    #[link_name = "Perl_newXS"]
    fn newXS(name: *const c_char, subaddr: unsafe extern "C" fn(*mut CV), filename: *const c_char)
        -> *mut CV;
}

// ---------------------------------------------------------------------------
// Stack access helpers
// ---------------------------------------------------------------------------
//
// In XS C code, `dSP` / `dXSARGS` / `ST(n)` are macros that reach into
// Perl's internal stack. Since we can't easily replicate those macros in
// Rust without Perl headers, we use a simpler approach:
//
// We call `Perl_call_sv` to invoke helper Perl code, OR we declare the
// XSUBs using a pattern where arguments are read from Perl's public API.
//
// For this implementation, we use `PL_stack_sp` — the Perl global stack
// pointer — which IS exported by libperl. We read arguments relative to it.
//
// PL_stack_sp is a thread-local in ithreads builds; in non-threaded builds
// it is a plain global. For simplicity we target non-threaded Perl here.

extern "C" {
    // The Perl argument stack mark stack — marks mark beginnings of argument lists.
    static mut PL_markstack_ptr: *mut i32;
    // The Perl value stack pointer — top of the argument stack.
    static mut PL_stack_sp: *mut *mut SV;
    // The Perl value stack base.
    static mut PL_stack_base: *mut *mut SV;
}

/// Declare SvRV — not in perl-bridge, needed to dereference array refs.
extern "C" {
    fn SvRV(sv: *mut SV) -> *mut SV;
    /// Returns non-zero if sv is a reference (RV). Must be checked before SvRV.
    fn SvROK(sv: *mut SV) -> c_int;
}

/// Read the number of arguments passed to the current XSUB.
///
/// The XS calling convention uses a "mark" on the argument stack. The mark
/// tells us where the current argument list begins. `ax` = mark index,
/// `items` = number of args = sp - (mark + 1).
///
/// This function uses saturating arithmetic and pointer comparison to guard
/// against pathological mark values that could cause pointer arithmetic overflow.
///
/// ## Guard against stack corruption
///
/// If `mark = i32::MAX`, then `ax = i32::MAX` (saturated), and
/// `base.add(i32::MAX as usize)` is pointer arithmetic that exceeds the valid
/// allocation range — undefined behaviour in Rust.
///
/// We therefore clamp `ax` to a sane maximum. Perl's maximum stack depth is
/// far below 64k arguments; 4096 is generous.
unsafe fn xsub_args() -> (*mut *mut SV, i32, i32) {
    let mark = *PL_markstack_ptr;
    // Guard against pathological mark values with saturating add.
    let ax = mark.saturating_add(1);
    let base = PL_stack_base;
    let sp = PL_stack_sp;

    // Guard: if ax is unreasonably large, Perl's stack is corrupted.
    // Perl's maximum stack depth is far below 64k arguments; 4096 is generous.
    const MAX_SANE_AX: i32 = 4096;
    if ax > MAX_SANE_AX || ax < 0 {
        // Stack is corrupted; return 0 items so each XSUB's arity check fires.
        return (base, 0, 0);
    }

    // Compute items with overflow protection: only subtract if sp >= base_ax.
    let base_ax = base.add(ax as usize);
    let items = if sp >= base_ax {
        ((sp as usize - base_ax as usize) / std::mem::size_of::<*mut SV>()) as i32 + 1
    } else {
        0
    };
    (base, ax, items)
}

/// Return n SV* results from an XSUB.
///
/// Adjusts the stack pointer to point to the return values, which must
/// already be in place starting at PL_stack_base[ax].
unsafe fn xsub_return(n: i32, ax: i32) {
    // Set sp to point to the last return value.
    PL_stack_sp = PL_stack_base.add((ax + n - 1) as usize);
    // Consume the mark.
    PL_markstack_ptr = PL_markstack_ptr.sub(1);
}

/// Read a polynomial (arrayref) from argument n.
///
/// Dereferences an SV* holding an array reference to get the AV*,
/// then converts the AV to Vec<f64>.
///
/// Returns `None` if the SV is null or not a reference. Callers must
/// call `die()` with a helpful message when `None` is returned.
unsafe fn arg_poly(base: *mut *mut SV, ax: i32, n: i32) -> Option<Vec<f64>> {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        return None;
    }
    // SvROK checks that the SV is actually a reference before calling SvRV.
    // Calling SvRV on a non-reference SV is undefined behaviour and can
    // segfault or corrupt memory.
    if SvROK(sv) == 0 {
        return None; // not a reference — caller handles via die()
    }
    let av = SvRV(sv);
    av_to_f64_vec(av as *mut AV)
}

/// Place an SV* return value at stack position n.
unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

// ---------------------------------------------------------------------------
// Helper: return a polynomial as an SV* (reference to an AV)
// ---------------------------------------------------------------------------

unsafe fn poly_to_sv(values: &[f64]) -> *mut SV {
    let av = f64_vec_to_av(values);
    // newRV_noinc wraps the AV in a reference SV without incrementing AV's refcount.
    // The reference "owns" the AV; when the RV is freed, the AV is freed too.
    newRV_noinc(av as *mut SV)
}

// ---------------------------------------------------------------------------
// XSUB implementations
// ---------------------------------------------------------------------------
//
// Each XSUB reads its arguments off Perl's stack, calls Rust, and puts the
// result(s) back. The pattern:
//
//   1. Wrap everything in catch_unwind to prevent Rust panics from unwinding
//      across the FFI boundary into Perl (undefined behaviour).
//   2. Validate argument count before reading from the stack.
//   3. Validate argument types (SvROK check via arg_poly).
//   4. Compute result.
//   5. Place result SVs starting at PL_stack_base[ax].
//   6. Call xsub_return(n_results, ax).
//
// Note: `die()` calls Perl's `croak()` which uses C `longjmp` — it never
// returns. The catch_unwind result only matters to distinguish "returned
// normally" vs "panicked".

extern "C" fn xs_normalize(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 1 {
            die("xs_normalize: expected 1 argument");
            return;
        }
        let poly = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_normalize: argument must be an array reference");
                return;
            }
        };
        let result = polynomial::normalize(&poly);
        set_return(base, ax, 0, poly_to_sv(&result));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_degree(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 1 {
            die("xs_degree: expected 1 argument");
            return;
        }
        let poly = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_degree: argument must be an array reference");
                return;
            }
        };
        let d = polynomial::degree(&poly) as IV;
        set_return(base, ax, 0, newSViv(d));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_zero(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| {
        unsafe {
            let frame = xsub_frame();
            let base = frame.base;
            let ax = frame.ax;
            // xs_zero takes no arguments; items may be 0 or more (extras ignored).
            set_return(base, ax, 0, poly_to_sv(&polynomial::zero()));
            xsub_return(1, ax);
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_one(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| {
        unsafe {
            let frame = xsub_frame();
            let base = frame.base;
            let ax = frame.ax;
            // xs_one takes no arguments; items may be 0 or more (extras ignored).
            set_return(base, ax, 0, poly_to_sv(&polynomial::one()));
            xsub_return(1, ax);
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_add(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_add: expected 2 arguments");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_add: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_add: argument 2 must be an array reference");
                return;
            }
        };
        set_return(base, ax, 0, poly_to_sv(&polynomial::add(&a, &b)));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_subtract(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_subtract: expected 2 arguments");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_subtract: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_subtract: argument 2 must be an array reference");
                return;
            }
        };
        set_return(base, ax, 0, poly_to_sv(&polynomial::subtract(&a, &b)));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_multiply(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_multiply: expected 2 arguments");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_multiply: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_multiply: argument 2 must be an array reference");
                return;
            }
        };
        set_return(base, ax, 0, poly_to_sv(&polynomial::multiply(&a, &b)));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

extern "C" fn xs_evaluate(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_evaluate: expected 2 arguments (poly, x)");
            return;
        }
        let poly = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_evaluate: argument 1 must be an array reference");
                return;
            }
        };
        let x_sv = *base.add((ax + 1) as usize);
        let x = sv_2nv(x_sv);
        let result = polynomial::evaluate(&poly, x);
        set_return(base, ax, 0, f64_to_sv(result));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

// ---------------------------------------------------------------------------
// Additional XSUBs: divmod_poly, divide, modulo, gcd
// ---------------------------------------------------------------------------

/// xs_divmod_poly(dividend_ref, divisor_ref) → arrayref [ quot_ref, rem_ref ]
///
/// Returns a reference to a 2-element Perl array whose first element is a
/// reference to the quotient polynomial and whose second element is a
/// reference to the remainder polynomial.
///
/// Returning a single arrayref (rather than two separate return values)
/// is the safest approach for XSUBs: Perl pre-allocates stack space for
/// the declared return count, and writing two values when the caller expects
/// one can overwrite adjacent stack slots.
extern "C" fn xs_divmod_poly(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| {
        unsafe {
            let frame = xsub_frame();
            let base = frame.base;
            let ax = frame.ax;
            let items = frame.items;
            if items < 2 {
                die("xs_divmod_poly: expected 2 arguments (dividend, divisor)");
                return;
            }
            let a = match arg_poly(base, ax, 0) {
                Some(p) => p,
                None => {
                    die("xs_divmod_poly: argument 1 must be an array reference");
                    return;
                }
            };
            let b = match arg_poly(base, ax, 1) {
                Some(p) => p,
                None => {
                    die("xs_divmod_poly: argument 2 must be an array reference");
                    return;
                }
            };
            // Catch division-by-zero panics from the Rust library.
            match std::panic::catch_unwind(|| polynomial::divmod(&a, &b)) {
                Ok((q, r)) => {
                    let quot_sv = poly_to_sv(&q);
                    let rem_sv = poly_to_sv(&r);
                    // Build an AV of [quot_sv, rem_sv] then wrap in an RV.
                    // av_push transfers ownership of the SVs to the AV.
                    let outer_av = newAV();
                    av_push(outer_av, quot_sv);
                    av_push(outer_av, rem_sv);
                    let outer_sv = newRV_noinc(outer_av as *mut SV);
                    set_return(base, ax, 0, outer_sv);
                    xsub_return(1, ax);
                }
                Err(_) => die("xs_divmod_poly: divisor is the zero polynomial"),
            }
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

/// xs_divide(dividend_ref, divisor_ref) → quotient_ref
///
/// Performs polynomial long division and returns only the quotient.
extern "C" fn xs_divide(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_divide: expected 2 arguments (dividend, divisor)");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_divide: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_divide: argument 2 must be an array reference");
                return;
            }
        };
        match std::panic::catch_unwind(|| polynomial::divide(&a, &b)) {
            Ok(result) => {
                set_return(base, ax, 0, poly_to_sv(&result));
                xsub_return(1, ax);
            }
            Err(_) => die("xs_divide: divisor is the zero polynomial"),
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

/// xs_modulo(dividend_ref, divisor_ref) → remainder_ref
///
/// Performs polynomial long division and returns only the remainder.
extern "C" fn xs_modulo(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_modulo: expected 2 arguments (dividend, divisor)");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_modulo: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_modulo: argument 2 must be an array reference");
                return;
            }
        };
        match std::panic::catch_unwind(|| polynomial::modulo(&a, &b)) {
            Ok(result) => {
                set_return(base, ax, 0, poly_to_sv(&result));
                xsub_return(1, ax);
            }
            Err(_) => die("xs_modulo: divisor is the zero polynomial"),
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

/// xs_gcd(a_ref, b_ref) → gcd_ref
///
/// Computes the greatest common divisor of two polynomials using the
/// Euclidean algorithm over polynomial rings.
extern "C" fn xs_gcd(_cv: *mut CV) {
    let result = std::panic::catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_gcd: expected 2 arguments");
            return;
        }
        let a = match arg_poly(base, ax, 0) {
            Some(p) => p,
            None => {
                die("xs_gcd: argument 1 must be an array reference");
                return;
            }
        };
        let b = match arg_poly(base, ax, 1) {
            Some(p) => p,
            None => {
                die("xs_gcd: argument 2 must be an array reference");
                return;
            }
        };
        match std::panic::catch_unwind(|| polynomial::gcd(&a, &b)) {
            Ok(result) => {
                set_return(base, ax, 0, poly_to_sv(&result));
                xsub_return(1, ax);
            }
            Err(_) => die("xs_gcd: computation panicked unexpectedly"),
        }
    });
    if result.is_err() {
        unsafe { die("polynomial operation panicked unexpectedly") };
    }
}

// ---------------------------------------------------------------------------
// Boot function — registers all XSUBs
// ---------------------------------------------------------------------------
//
// `boot_CodingAdventures__PolynomialNative` is called by DynaLoader when
// `use CodingAdventures::PolynomialNative;` is executed.
//
// Double underscore `__` is Perl's package-separator in C symbol names:
//   CodingAdventures::PolynomialNative → CodingAdventures__PolynomialNative
//
// We write this by hand because xs_init! requires concat_idents (not stable).

/// Register all polynomial XSUBs with Perl's symbol table.
///
/// Called by DynaLoader when the module is first loaded.
#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__PolynomialNative(cv: *mut CV) {
    let file = b"PolynomialNative.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);

    newXS(b"CodingAdventures::PolynomialNative::normalize\0".as_ptr() as *const c_char,
          xs_normalize, file);
    newXS(b"CodingAdventures::PolynomialNative::degree\0".as_ptr() as *const c_char,
          xs_degree, file);
    newXS(b"CodingAdventures::PolynomialNative::zero\0".as_ptr() as *const c_char,
          xs_zero, file);
    newXS(b"CodingAdventures::PolynomialNative::one\0".as_ptr() as *const c_char,
          xs_one, file);
    newXS(b"CodingAdventures::PolynomialNative::add\0".as_ptr() as *const c_char,
          xs_add, file);
    newXS(b"CodingAdventures::PolynomialNative::subtract\0".as_ptr() as *const c_char,
          xs_subtract, file);
    newXS(b"CodingAdventures::PolynomialNative::multiply\0".as_ptr() as *const c_char,
          xs_multiply, file);
    newXS(b"CodingAdventures::PolynomialNative::evaluate\0".as_ptr() as *const c_char,
          xs_evaluate, file);
    newXS(b"CodingAdventures::PolynomialNative::divmod_poly\0".as_ptr() as *const c_char,
          xs_divmod_poly, file);
    newXS(b"CodingAdventures::PolynomialNative::divide\0".as_ptr() as *const c_char,
          xs_divide, file);
    newXS(b"CodingAdventures::PolynomialNative::modulo\0".as_ptr() as *const c_char,
          xs_modulo, file);
    newXS(b"CodingAdventures::PolynomialNative::gcd\0".as_ptr() as *const c_char,
          xs_gcd, file);
    xs_boot_finish(ax);
}
