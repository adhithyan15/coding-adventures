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
//! The full XS calling convention (dXSARGS, ST(n), XSRETURN) requires
//! accessing Perl's internal stack pointer from C. Rather than duplicating
//! the full macro expansion in Rust, we expose a clean subset of functions
//! that work with Perl's `call_sv` / `eval_sv` style, using a simpler
//! approach: each XSUB reads globals from the Perl stack.
//!
//! For simplicity and correctness, we expose the functions as C-callable
//! stubs registered via `newXS`. Each function takes no arguments directly;
//! instead it uses Perl's stack introspection macros. Since those macros
//! are not in perl-bridge, we use a reduced set of functions that returns
//! one SV at a time — appropriate for a teaching implementation.
//!
//! ## Note on xs_init! macro
//!
//! The `xs_init!` macro in perl-bridge requires `concat_idents`, which is
//! not in stable Rust. We write the boot function by hand instead.

#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::{
    av_to_f64_vec, die, f64_to_sv, f64_vec_to_av, i64_to_sv, sv_to_f64, sv_to_i64,
    AV, CV, IV, SV, SvREFCNT_dec, newSViv, sv_2iv, sv_2nv,
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

/// Read the number of arguments passed to the current XSUB.
///
/// The XS calling convention uses a "mark" on the argument stack. The mark
/// tells us where the current argument list begins. `ax` = mark index,
/// `items` = number of args = sp - (mark + 1).
unsafe fn xsub_args() -> (*mut *mut SV, i32, i32) {
    // sp = current stack pointer (top of arg list)
    let sp = PL_stack_sp;
    // The mark is at PL_markstack_ptr[0]; it is an index into PL_stack_base.
    let mark = *PL_markstack_ptr;
    // ax = index of first argument relative to PL_stack_base
    let ax = mark + 1;
    // items = number of arguments
    let items = (sp as isize - PL_stack_base.add(ax as usize) as isize)
        / (std::mem::size_of::<*mut SV>() as isize)
        + 1;
    (sp, ax, items as i32)
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
unsafe fn arg_poly(base: *mut *mut SV, ax: i32, n: i32) -> Vec<f64> {
    let sv = *base.add((ax + n) as usize);
    // Dereference the SV: get the AV* from an RV (reference to array).
    // In Perl's C API, SvRV(sv) gives the referent of a reference SV.
    // We use sv_2iv to get the integer representation which IS the pointer.
    // Actually, the correct way: cast via Perl internals. We'll use the
    // fact that sv_2iv on an RV gives the address of the referent.
    // Better: use a direct cast approach via the AV pointer.
    let av = SvRV(sv);
    match av_to_f64_vec(av as *mut AV) {
        Some(v) => v,
        None => die("polynomial argument must be an array reference of numbers"),
    }
}

/// Declare SvRV — not in perl-bridge, needed to dereference array refs.
extern "C" {
    fn SvRV(sv: *mut SV) -> *mut SV;
}

/// Place an SV* return value at stack position n.
unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

// ---------------------------------------------------------------------------
// Helper: return a polynomial as an SV* (reference to an AV)
// ---------------------------------------------------------------------------

extern "C" {
    /// Create a reference to an SV (makes an RV). Returns a new SV* with refcount 1.
    fn newRV_noinc(sv: *mut SV) -> *mut SV;
}

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
//   1. let (sp, ax, items) = xsub_args();
//   2. Read arg n as: *PL_stack_base.add((ax + n) as usize)
//   3. Compute result.
//   4. Place result SVs starting at PL_stack_base[ax].
//   5. Call xsub_return(n_results, ax).

unsafe extern "C" fn xs_normalize(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let poly = arg_poly(base, ax, 0);
    let result = polynomial::normalize(&poly);
    set_return(base, ax, 0, poly_to_sv(&result));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_degree(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let poly = arg_poly(base, ax, 0);
    let d = polynomial::degree(&poly) as IV;
    set_return(base, ax, 0, newSViv(d));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_zero(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    set_return(base, ax, 0, poly_to_sv(&polynomial::zero()));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_one(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    set_return(base, ax, 0, poly_to_sv(&polynomial::one()));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_add(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let a = arg_poly(base, ax, 0);
    let b = arg_poly(base, ax, 1);
    set_return(base, ax, 0, poly_to_sv(&polynomial::add(&a, &b)));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_subtract(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let a = arg_poly(base, ax, 0);
    let b = arg_poly(base, ax, 1);
    set_return(base, ax, 0, poly_to_sv(&polynomial::subtract(&a, &b)));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_multiply(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let a = arg_poly(base, ax, 0);
    let b = arg_poly(base, ax, 1);
    set_return(base, ax, 0, poly_to_sv(&polynomial::multiply(&a, &b)));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_evaluate(_cv: *mut CV) {
    let (_, ax, _items) = xsub_args();
    let base = PL_stack_base;
    let poly = arg_poly(base, ax, 0);
    let x_sv = *base.add((ax + 1) as usize);
    let x = sv_2nv(x_sv);
    let result = polynomial::evaluate(&poly, x);
    set_return(base, ax, 0, f64_to_sv(result));
    xsub_return(1, ax);
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
pub unsafe extern "C" fn boot_CodingAdventures__PolynomialNative(_cv: *mut CV) {
    let file = b"PolynomialNative.so\0".as_ptr() as *const c_char;

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
}
