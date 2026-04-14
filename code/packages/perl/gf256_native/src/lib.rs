//! # GF256Native — Perl XS extension wrapping Rust GF(256) arithmetic
//!
//! This Perl XS module exposes GF(256) Galois Field arithmetic to Perl scripts.
//!
//! ## GF(256) in a nutshell
//!
//! GF(256) is the finite field with exactly 256 elements (the bytes 0–255).
//! It is used in Reed-Solomon error correction, QR codes, and AES. Addition
//! is XOR; multiplication uses precomputed log/antilog tables.
//!
//! ## Element representation
//!
//! Elements are Perl scalars holding integers 0–255.
//!
//! ## XS calling convention
//!
//! Same pattern as PolynomialNative: `boot_CodingAdventures__GF256Native`
//! registers XSUBs via `newXS`. Each XSUB reads from/writes to Perl's stack
//! through `perl-bridge`, which routes stack access through the host Perl's
//! own XS macros so threaded builds are safe.

#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::{
    die, newSViv, newXS, sv_2iv, xs_boot_finish, xs_bootstrap, xsub_frame, xsub_return, CV, IV, SV,
};
use std::ffi::c_char;
use std::panic::catch_unwind;

unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

/// Read a GF(256) element (integer 0–255) from argument n.
///
/// ## Null guard
///
/// `sv_2iv` is undefined when called on a null pointer. Perl can produce a
/// null SV* on the stack if the caller passes an uninitialized variable in
/// certain edge cases. We check for null before dereferencing.
///
/// `die()` calls Perl's `croak()` which uses C `longjmp` — it never returns.
/// The `return 0` after each `die()` call is dead code that exists solely to
/// satisfy the Rust type-checker (which cannot know that `die` is `!`).
unsafe fn arg_u8(base: *mut *mut SV, ax: i32, n: i32) -> u8 {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("GF256 argument is null");
        // die never returns (longjmp), but return 0 to satisfy the compiler.
        return 0;
    }
    let v = sv_2iv(sv);
    if v < 0 || v > 255 {
        die("GF256 argument out of range [0, 255]");
    }
    v as u8
}

/// Read a non-negative integer exponent from argument n.
///
/// ## Null guard
///
/// Same null-safety rationale as `arg_u8` above.
unsafe fn arg_u32(base: *mut *mut SV, ax: i32, n: i32) -> u32 {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("GF256 exponent argument is null");
        // die never returns (longjmp), but return 0 to satisfy the compiler.
        return 0;
    }
    let v = sv_2iv(sv);
    if v < 0 {
        die("GF256 exponent must be non-negative");
    }
    v as u32
}

// ---------------------------------------------------------------------------
// XSUB implementations
// ---------------------------------------------------------------------------
//
// Each XSUB:
//   1. Wraps everything in catch_unwind to prevent Rust panics from unwinding
//      across the FFI boundary into Perl (undefined behaviour).
//   2. Validates argument count before reading from the stack.
//   3. Computes result.
//   4. Places result SVs starting at PL_stack_base[ax].
//   5. Calls xsub_return(n_results, ax).
//
// Note: `die()` calls Perl's `croak()` which uses C `longjmp` — it never
// returns. The catch_unwind result only matters to distinguish "returned
// normally" vs "panicked".

extern "C" fn xs_add(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_add: expected 2 arguments");
            return;
        }
        let a = arg_u8(base, ax, 0);
        let b = arg_u8(base, ax, 1);
        set_return(base, ax, 0, newSViv(gf256::add(a, b) as IV));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

extern "C" fn xs_subtract(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_subtract: expected 2 arguments");
            return;
        }
        let a = arg_u8(base, ax, 0);
        let b = arg_u8(base, ax, 1);
        set_return(base, ax, 0, newSViv(gf256::subtract(a, b) as IV));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

extern "C" fn xs_multiply(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_multiply: expected 2 arguments");
            return;
        }
        let a = arg_u8(base, ax, 0);
        let b = arg_u8(base, ax, 1);
        set_return(base, ax, 0, newSViv(gf256::multiply(a, b) as IV));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

extern "C" fn xs_divide(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_divide: expected 2 arguments");
            return;
        }
        let a = arg_u8(base, ax, 0);
        let b = arg_u8(base, ax, 1);
        match catch_unwind(|| gf256::divide(a, b)) {
            Ok(result) => {
                set_return(base, ax, 0, newSViv(result as IV));
                xsub_return(1, ax);
            }
            Err(_) => die("GF256: division by zero"),
        }
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

extern "C" fn xs_power(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 2 {
            die("xs_power: expected 2 arguments (base, exponent)");
            return;
        }
        let b = arg_u8(base, ax, 0);
        let e = arg_u32(base, ax, 1);
        set_return(base, ax, 0, newSViv(gf256::power(b, e) as IV));
        xsub_return(1, ax);
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

extern "C" fn xs_inverse(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 1 {
            die("xs_inverse: expected 1 argument");
            return;
        }
        let a = arg_u8(base, ax, 0);
        match catch_unwind(|| gf256::inverse(a)) {
            Ok(result) => {
                set_return(base, ax, 0, newSViv(result as IV));
                xsub_return(1, ax);
            }
            Err(_) => die("GF256: zero has no multiplicative inverse"),
        }
    });
    if result.is_err() {
        unsafe { die("GF256 operation panicked unexpectedly") };
    }
}

// ---------------------------------------------------------------------------
// Boot function
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__GF256Native(cv: *mut CV) {
    let file = b"GF256Native.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);

    newXS(
        b"CodingAdventures::GF256Native::add\0".as_ptr() as *const c_char,
        xs_add,
        file,
    );
    newXS(
        b"CodingAdventures::GF256Native::subtract\0".as_ptr() as *const c_char,
        xs_subtract,
        file,
    );
    newXS(
        b"CodingAdventures::GF256Native::multiply\0".as_ptr() as *const c_char,
        xs_multiply,
        file,
    );
    newXS(
        b"CodingAdventures::GF256Native::divide\0".as_ptr() as *const c_char,
        xs_divide,
        file,
    );
    newXS(
        b"CodingAdventures::GF256Native::power\0".as_ptr() as *const c_char,
        xs_power,
        file,
    );
    newXS(
        b"CodingAdventures::GF256Native::inverse\0".as_ptr() as *const c_char,
        xs_inverse,
        file,
    );
    xs_boot_finish(ax);
}
