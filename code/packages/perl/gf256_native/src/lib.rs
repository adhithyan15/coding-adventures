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
//! registers XSUBs via `newXS`. Each XSUB reads from/writes to `PL_stack_*`.

#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::{
    die, CV, IV, SV,
    newSViv, sv_2iv,
};
use std::ffi::c_char;
use std::panic::catch_unwind;

// ---------------------------------------------------------------------------
// Declare runtime functions not in perl-bridge
// ---------------------------------------------------------------------------

extern "C" {
    fn newXS(name: *const c_char, subaddr: unsafe extern "C" fn(*mut CV), filename: *const c_char)
        -> *mut CV;
}

extern "C" {
    static mut PL_markstack_ptr: *mut i32;
    static mut PL_stack_sp: *mut *mut SV;
    static mut PL_stack_base: *mut *mut SV;
}

// ---------------------------------------------------------------------------
// Stack helpers (same pattern as polynomial_native)
// ---------------------------------------------------------------------------

unsafe fn xsub_args() -> (*mut *mut SV, i32, i32) {
    let sp = PL_stack_sp;
    let mark = *PL_markstack_ptr;
    let ax = mark + 1;
    let items = (sp as isize - PL_stack_base.add(ax as usize) as isize)
        / (std::mem::size_of::<*mut SV>() as isize)
        + 1;
    (sp, ax, items as i32)
}

unsafe fn xsub_return(n: i32, ax: i32) {
    PL_stack_sp = PL_stack_base.add((ax + n - 1) as usize);
    PL_markstack_ptr = PL_markstack_ptr.sub(1);
}

unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

/// Read a GF(256) element (integer 0–255) from argument n.
unsafe fn arg_u8(base: *mut *mut SV, ax: i32, n: i32) -> u8 {
    let sv = *base.add((ax + n) as usize);
    let v = sv_2iv(sv);
    if v < 0 || v > 255 {
        die("GF256 argument out of range [0, 255]");
    }
    v as u8
}

/// Read a non-negative integer exponent from argument n.
unsafe fn arg_u32(base: *mut *mut SV, ax: i32, n: i32) -> u32 {
    let sv = *base.add((ax + n) as usize);
    let v = sv_2iv(sv);
    if v < 0 {
        die("GF256 exponent must be non-negative");
    }
    v as u32
}

// ---------------------------------------------------------------------------
// XSUB implementations
// ---------------------------------------------------------------------------

unsafe extern "C" fn xs_add(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let a = arg_u8(base, ax, 0);
    let b = arg_u8(base, ax, 1);
    set_return(base, ax, 0, newSViv(gf256::add(a, b) as IV));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_subtract(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let a = arg_u8(base, ax, 0);
    let b = arg_u8(base, ax, 1);
    set_return(base, ax, 0, newSViv(gf256::subtract(a, b) as IV));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_multiply(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let a = arg_u8(base, ax, 0);
    let b = arg_u8(base, ax, 1);
    set_return(base, ax, 0, newSViv(gf256::multiply(a, b) as IV));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_divide(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let a = arg_u8(base, ax, 0);
    let b = arg_u8(base, ax, 1);
    match catch_unwind(|| gf256::divide(a, b)) {
        Ok(result) => {
            set_return(base, ax, 0, newSViv(result as IV));
            xsub_return(1, ax);
        }
        Err(_) => die("GF256: division by zero"),
    }
}

unsafe extern "C" fn xs_power(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let b = arg_u8(base, ax, 0);
    let e = arg_u32(base, ax, 1);
    set_return(base, ax, 0, newSViv(gf256::power(b, e) as IV));
    xsub_return(1, ax);
}

unsafe extern "C" fn xs_inverse(_cv: *mut CV) {
    let (_, ax, _) = xsub_args();
    let base = PL_stack_base;
    let a = arg_u8(base, ax, 0);
    match catch_unwind(|| gf256::inverse(a)) {
        Ok(result) => {
            set_return(base, ax, 0, newSViv(result as IV));
            xsub_return(1, ax);
        }
        Err(_) => die("GF256: zero has no multiplicative inverse"),
    }
}

// ---------------------------------------------------------------------------
// Boot function
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__GF256Native(_cv: *mut CV) {
    let file = b"GF256Native.so\0".as_ptr() as *const c_char;

    newXS(b"CodingAdventures::GF256Native::add\0".as_ptr() as *const c_char,
          xs_add, file);
    newXS(b"CodingAdventures::GF256Native::subtract\0".as_ptr() as *const c_char,
          xs_subtract, file);
    newXS(b"CodingAdventures::GF256Native::multiply\0".as_ptr() as *const c_char,
          xs_multiply, file);
    newXS(b"CodingAdventures::GF256Native::divide\0".as_ptr() as *const c_char,
          xs_divide, file);
    newXS(b"CodingAdventures::GF256Native::power\0".as_ptr() as *const c_char,
          xs_power, file);
    newXS(b"CodingAdventures::GF256Native::inverse\0".as_ptr() as *const c_char,
          xs_inverse, file);
}
