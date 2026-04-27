// Perl's C API uses non-standard naming conventions inherited from its C headers:
// mixed-case constants (SVs_TEMP, SVf_IOK), non-snake-case functions (newSViv,
// SvREFCNT_inc), and upper-case type names (SV, AV, HV). Allow these throughout.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

//! # perl-bridge — Rust wrapper for Perl's C API
//!
//! This crate provides safe Rust wrappers around Perl's C extension API.
//! A tiny C shim, compiled as part of the crate, handles the header-only
//! macros and thread-context-sensitive stack helpers that threaded Perl
//! exposes through `perl.h`/`XSUB.h`.
//!
//! ## How it works
//!
//! Perl's native extension system (XS/DynaLoader) loads shared libraries at
//! runtime when `use MyModule;` is called. The loader looks for a
//! `boot_MyModule` function in the `.so`/`.dll` and calls it. Our `extern "C"`
//! declarations are resolved against the running Perl interpreter at load time.
//!
//! ## The SV model
//!
//! Every Perl value is an **SV** (Scalar Value). Arrays are **AV** (Array
//! Value), hashes are **HV** (Hash Value), and code refs are **CV** (Code
//! Value). All are heap-allocated and reference-counted:
//!
//! ```text
//! SV* sv = newSViv(42);   // integer SV
//! SvREFCNT_inc(sv);       // bump refcount (keep alive)
//! SvREFCNT_dec(sv);       // decrement refcount (maybe free)
//! ```
//!
//! ## The XS calling convention
//!
//! Perl's native functions ("XSUBs") have a different calling convention from
//! normal C callbacks. They are called as:
//!
//! ```c
//! void xsub_fn(pTHX_ CV* cv)
//! ```
//!
//! Where `pTHX_` expands to a pointer to the Perl interpreter (thread context)
//! in thread-safe Perl, or nothing in non-threaded builds. Arguments arrive
//! on Perl's argument stack and are accessed via the `ST(n)` macro.
//!
//! ## Return value convention
//!
//! XSUBs push return values onto the stack and call `XSRETURN(n)`. The
//! convenience macros `XSRETURN_IV`, `XSRETURN_NV`, etc. are declared here
//! as extern "C".
//!
//! ## Why zero dependencies?
//!
//! - **No external Rust dependencies** — no XS crate wrappers or bindgen
//! - **Threaded-Perl safe** — stack access goes through Perl's own macros
//! - **No version conflicts** — works with the host Perl installation
//! - **Fully auditable** — every Perl API touchpoint is explicit

use std::ffi::{c_char, c_int, c_void, CString};

// ---------------------------------------------------------------------------
// Perl's fundamental value types
// ---------------------------------------------------------------------------
//
// Every Perl value is an SV*. AV*, HV*, and CV* are specializations that
// add array/hash/code-specific fields to the base SV structure. They are
// always passed as pointers; you never hold them by value.

/// A Perl Scalar Value — the universal Perl data type.
///
/// SV can hold an integer (IV), a float (NV), a string (PV), a reference,
/// or a combination. The `Sv*` flags tell you which representation is valid:
/// - `SvIOK(sv)` → IV field is valid
/// - `SvNOK(sv)` → NV field is valid
/// - `SvPOK(sv)` → PV (string) field is valid
#[repr(C)]
pub struct SV {
    _opaque: [u8; 0],
}

/// A Perl Array Value. Accessed via `av_*` functions.
#[repr(C)]
pub struct AV {
    _opaque: [u8; 0],
}

/// A Perl Hash Value. Accessed via `hv_*` functions.
#[repr(C)]
pub struct HV {
    _opaque: [u8; 0],
}

/// A Perl Code Value (subroutine reference). Accessed via `cv_*` functions.
#[repr(C)]
pub struct CV {
    _opaque: [u8; 0],
}

/// The Perl interpreter struct. In thread-safe Perl (`ithreads`), this is
/// passed as the first implicit argument to almost every API call.
#[repr(C)]
pub struct PerlInterpreter {
    _opaque: [u8; 0],
}

/// `pTHX` is the Perl thread context (interpreter pointer).
///
/// In single-threaded builds, this pointer is implicit. In threaded builds,
/// it is passed explicitly. We declare it for the threaded signatures.
pub type pTHX = *mut PerlInterpreter;

// ---------------------------------------------------------------------------
// Perl's integer and float typedefs
// ---------------------------------------------------------------------------
//
// Perl's C API uses these instead of `int` / `double` to ensure portability.
// On 64-bit systems, IV and UV are 64-bit; NV is double.

/// Perl's signed integer type (matches Perl's `Int`).
pub type IV = isize;

/// Perl's unsigned integer type (matches Perl's `UInt`).
pub type UV = usize;

/// Perl's floating-point type (always `double`).
pub type NV = f64;

/// Perl's internal stack offset type used by XS boot helpers.
pub type StackOffset = isize;

// ---------------------------------------------------------------------------
// SV type tag constants
// ---------------------------------------------------------------------------
//
// These are the internal SV type codes stored in `SvTYPE(sv)`. You rarely
// need these directly; use the `SvIOK`, `SvNOK`, `SvPOK` predicates instead.

/// SV holds an integer (IV).
pub const SVt_IV: u32 = 1;
/// SV holds a float (NV).
pub const SVt_NV: u32 = 2;
/// SV holds a string (PV).
pub const SVt_PV: u32 = 4;

/// Integer 1 (for `c_int` return values meaning "true").
pub const TRUE: c_int = 1;
/// Integer 0 (for `c_int` return values meaning "false").
pub const FALSE: c_int = 0;

// ---------------------------------------------------------------------------
// SV flag constants
// ---------------------------------------------------------------------------
//
// These flags live in `SvFLAGS(sv)` and indicate which representation fields
// are currently valid in the SV. An SV can have multiple valid representations
// simultaneously (e.g. a number that has been stringified has both NOK and POK).

/// The SV's IV slot is valid (integer representation is current).
pub const SVf_IOK: u32 = 0x00000100;
/// The SV's NV slot is valid (float representation is current).
pub const SVf_NOK: u32 = 0x00000200;
/// The SV's PV slot is valid (string representation is current).
pub const SVf_POK: u32 = 0x00000400;
/// This SV is a temporary (will be freed at end of expression).
pub const SVs_TEMP: u32 = 0x00000800;

// ---------------------------------------------------------------------------
// Perl C API — extern "C" declarations
// ---------------------------------------------------------------------------
//
// These symbols are exported by the Perl interpreter (`libperl`). The
// dynamic linker resolves them when our XS module is loaded.
//
// Note: Many Perl API functions are actually C macros (Sv*). We declare the
// underlying function-equivalents here. Where Perl only provides a macro, we
// implement it manually below.

#[allow(non_snake_case)]
extern "C" {
    #[link_name = "perl_bridge_newSViv"]
    fn raw_newSViv(i: IV) -> *mut SV;
    #[link_name = "perl_bridge_newSVnv"]
    fn raw_newSVnv(n: NV) -> *mut SV;
    #[link_name = "perl_bridge_newSVpv"]
    fn raw_newSVpv(s: *const c_char, len: usize) -> *mut SV;
    #[link_name = "perl_bridge_newSVpvn"]
    fn raw_newSVpvn(s: *const c_char, len: usize) -> *mut SV;
    #[link_name = "perl_bridge_newSVuv"]
    fn raw_newSVuv(u: UV) -> *mut SV;
    #[link_name = "perl_bridge_sv_2iv"]
    fn raw_sv_2iv(sv: *mut SV) -> IV;
    #[link_name = "perl_bridge_sv_2nv"]
    fn raw_sv_2nv(sv: *mut SV) -> NV;
    #[link_name = "perl_bridge_sv_2pv_flags"]
    fn raw_sv_2pv_flags(sv: *mut SV, lp: *mut usize, flags: u32) -> *mut c_char;
    #[link_name = "perl_bridge_sv_true"]
    fn raw_sv_true(sv: *mut SV) -> c_int;
    #[link_name = "perl_bridge_sv_iok"]
    fn raw_sv_iok(sv: *mut SV) -> c_int;
    #[link_name = "perl_bridge_sv_nok"]
    fn raw_sv_nok(sv: *mut SV) -> c_int;
    #[link_name = "perl_bridge_sv_pok"]
    fn raw_sv_pok(sv: *mut SV) -> c_int;
    #[link_name = "perl_bridge_sv_refcnt_inc"]
    fn raw_SvREFCNT_inc(sv: *mut SV) -> *mut SV;
    #[link_name = "perl_bridge_sv_refcnt_dec"]
    fn raw_SvREFCNT_dec(sv: *mut SV);
    #[link_name = "perl_bridge_newAV"]
    fn raw_newAV() -> *mut AV;
    #[link_name = "perl_bridge_av_push"]
    fn raw_av_push(av: *mut AV, val: *mut SV);
    #[link_name = "perl_bridge_av_pop"]
    fn raw_av_pop(av: *mut AV) -> *mut SV;
    #[link_name = "perl_bridge_av_fetch"]
    fn raw_av_fetch(av: *mut AV, key: isize, lval: c_int) -> *mut *mut SV;
    #[link_name = "perl_bridge_av_len"]
    fn raw_av_len(av: *mut AV) -> isize;
    #[link_name = "perl_bridge_newRV_noinc"]
    fn raw_newRV_noinc(sv: *mut SV) -> *mut SV;
    #[link_name = "perl_bridge_sv_rv"]
    fn raw_SvRV(sv: *mut SV) -> *mut SV;
    #[link_name = "perl_bridge_sv_rok"]
    fn raw_SvROK(sv: *mut SV) -> c_int;
    #[link_name = "perl_bridge_croak_message"]
    fn raw_croak_message(msg: *const c_char) -> !;
    #[link_name = "perl_bridge_warn_message"]
    fn raw_warn_message(msg: *const c_char);
    #[link_name = "perl_bridge_xsub_frame"]
    fn raw_xsub_frame(frame: *mut XsubFrame);
    #[link_name = "perl_bridge_xsub_return"]
    fn raw_xsub_return(ax: i32, count: i32);
    #[link_name = "perl_bridge_newXS"]
    fn raw_newXS(
        name: *const c_char,
        subaddr: unsafe extern "C" fn(*mut CV),
        filename: *const c_char,
    ) -> *mut CV;
    #[link_name = "perl_bridge_xs_boot_finish"]
    fn raw_xs_boot_finish(ax: StackOffset);

    /// Create a new SV holding an integer (IV). Returns a mortal SV with
    /// refcount 1. Equivalent to Perl's `my $x = 42`.
    #[link_name = "Perl_newSViv"]
    pub fn newSViv(i: IV) -> *mut SV;

    /// Create a new SV holding a float (NV). Equivalent to Perl's `my $x = 3.14`.
    #[link_name = "Perl_newSVnv"]
    pub fn newSVnv(n: NV) -> *mut SV;

    /// Create a new SV holding a string copy of `s[0..len]`.
    /// If `len == 0`, the string is determined by `strlen(s)`.
    #[link_name = "Perl_newSVpv"]
    pub fn newSVpv(s: *const c_char, len: usize) -> *mut SV;

    /// Like `newSVpv` but explicitly uses the provided length.
    #[link_name = "Perl_newSVpvn"]
    pub fn newSVpvn(s: *const c_char, len: usize) -> *mut SV;

    /// Create a new SV holding an unsigned integer (UV).
    #[link_name = "Perl_newSVuv"]
    pub fn newSVuv(u: UV) -> *mut SV;

    // -- SV conversion (macro-equivalents) ---------------------------------
    //
    // These are the underlying functions that the `Sv2*` macros call.
    // They coerce the SV to the requested type if needed.

    /// Convert (coerce) an SV to an IV. E.g. string `"42"` → 42.
    #[link_name = "Perl_sv_2iv"]
    pub fn sv_2iv(sv: *mut SV) -> IV;

    /// Convert (coerce) an SV to an NV (float). E.g. string `"3.14"` → 3.14.
    #[link_name = "Perl_sv_2nv"]
    pub fn sv_2nv(sv: *mut SV) -> NV;

    /// Convert (coerce) an SV to a PV (string pointer).
    /// `flags` controls stringification behavior (0 = default).
    /// The returned pointer is owned by the SV; do not free it.
    #[link_name = "Perl_sv_2pv_flags"]
    pub fn sv_2pv_flags(sv: *mut SV, lp: *mut usize, flags: u32) -> *mut c_char;

    /// Return the boolean truth value of an SV (like Perl's `if ($sv)`).
    pub fn SvTRUE(sv: *mut SV) -> bool;

    /// Return true if the SV's IV (integer) representation is valid.
    pub fn SvIOK(sv: *mut SV) -> bool;

    /// Return true if the SV's NV (float) representation is valid.
    pub fn SvNOK(sv: *mut SV) -> bool;

    /// Return true if the SV's PV (string) representation is valid.
    pub fn SvPOK(sv: *mut SV) -> bool;

    // -- Reference counting ------------------------------------------------
    //
    // Perl uses reference counting for memory management. XS code must
    // maintain counts carefully: each `newSV*` or `av_pop` returns an SV
    // with refcount 1. Store it somewhere to own it; decrement when done.

    /// Increment the SV's reference count. Returns `sv` for chaining.
    /// Equivalent to the `SvREFCNT_inc` macro.
    pub fn SvREFCNT_inc(sv: *mut SV) -> *mut SV;

    /// Decrement the SV's reference count. If it reaches 0, the SV is freed.
    /// Equivalent to the `SvREFCNT_dec` macro.
    pub fn SvREFCNT_dec(sv: *mut SV);

    // -- Array operations --------------------------------------------------

    /// Create a new empty AV (Perl array). Refcount starts at 1.
    #[link_name = "Perl_newAV"]
    pub fn newAV() -> *mut AV;

    /// Push `val` onto the end of `av`. Takes ownership of `val`'s reference.
    #[link_name = "Perl_av_push"]
    pub fn av_push(av: *mut AV, val: *mut SV);

    /// Pop the last element from `av` and return it.
    /// The caller owns the returned SV and must decrement its refcount.
    #[link_name = "Perl_av_pop"]
    pub fn av_pop(av: *mut AV) -> *mut SV;

    /// Fetch the SV at index `key` from `av`.
    /// If `lval` is non-zero, creates the slot if absent.
    /// Returns a pointer-to-SV-pointer (double indirection), or null if absent.
    #[link_name = "Perl_av_fetch"]
    pub fn av_fetch(av: *mut AV, key: isize, lval: c_int) -> *mut *mut SV;

    /// Return the highest valid index in `av` (i.e. `length - 1`).
    /// Returns -1 for an empty array.
    #[link_name = "Perl_av_len"]
    pub fn av_len(av: *mut AV) -> isize;

    /// Create a new AV from an array of `size` SV pointers.
    #[link_name = "Perl_av_make"]
    pub fn av_make(size: isize, strp: *mut *mut SV) -> *mut AV;

    // -- Stack management --------------------------------------------------
    //
    // Perl's argument stack is how XSUBs receive arguments and return values.
    // The stack is accessed via macros like `ST(n)` in C. Here we declare the
    // internal growth function; the actual stack pointer arithmetic is done in
    // our `xs_arg_sv` helper below.

    /// Grow the Perl argument stack if needed. Called internally by XS macros.
    pub fn Perl_stack_grow(
        my_perl: pTHX,
        sp: *mut *mut SV,
        p: *mut *mut SV,
        n: c_int,
    ) -> *mut *mut SV;

    // -- Error handling ----------------------------------------------------

    /// Perl's `die()` equivalent. Raises a Perl exception. Never returns.
    /// `pat` is a printf-style format string.
    #[link_name = "Perl_croak"]
    pub fn croak(pat: *const c_char, ...) -> !;

    /// Complete XS bootstrapping by restoring the Perl stack for DynaLoader.
    pub fn Perl_xs_boot_epilog(ax: StackOffset);

    /// Perform Perl's XS bootstrap handshake and return the boot stack offset.
    pub fn Perl_xs_handshake(
        key: u32,
        v_my_perl: *mut c_void,
        file: *const c_char,
        ...,
    ) -> StackOffset;

    /// Perl's `warn()` equivalent. Prints a warning to STDERR. Returns normally.
    #[link_name = "Perl_warn"]
    pub fn warn(pat: *const c_char, ...);

    // -- XS return helpers -------------------------------------------------
    //
    // These push a single return value onto the stack and set up the return
    // count. They are typically called as the last thing in an XSUB.

    /// Return an IV (integer) value from an XSUB.
    pub fn XSRETURN_IV(i: IV);

    /// Return an NV (float) value from an XSUB.
    pub fn XSRETURN_NV(n: NV);

    /// Return a PV (string) value from an XSUB.
    pub fn XSRETURN_PV(s: *const c_char);

    /// Return `undef` from an XSUB.
    pub fn XSRETURN_UNDEF();
}

// ---------------------------------------------------------------------------
// Safe helper functions — the "bridge" layer
// ---------------------------------------------------------------------------

pub unsafe fn newSViv(i: IV) -> *mut SV {
    raw_newSViv(i)
}

pub unsafe fn newSVnv(n: NV) -> *mut SV {
    raw_newSVnv(n)
}

pub unsafe fn newSVpv(s: *const c_char, len: usize) -> *mut SV {
    raw_newSVpv(s, len)
}

pub unsafe fn newSVpvn(s: *const c_char, len: usize) -> *mut SV {
    raw_newSVpvn(s, len)
}

pub unsafe fn newSVuv(u: UV) -> *mut SV {
    raw_newSVuv(u)
}

pub unsafe fn sv_2iv(sv: *mut SV) -> IV {
    raw_sv_2iv(sv)
}

pub unsafe fn sv_2nv(sv: *mut SV) -> NV {
    raw_sv_2nv(sv)
}

pub unsafe fn sv_2pv_flags(sv: *mut SV, lp: *mut usize, flags: u32) -> *mut c_char {
    raw_sv_2pv_flags(sv, lp, flags)
}

pub unsafe fn SvTRUE(sv: *mut SV) -> bool {
    raw_sv_true(sv) != 0
}

pub unsafe fn SvIOK(sv: *mut SV) -> bool {
    raw_sv_iok(sv) != 0
}

pub unsafe fn SvNOK(sv: *mut SV) -> bool {
    raw_sv_nok(sv) != 0
}

pub unsafe fn SvPOK(sv: *mut SV) -> bool {
    raw_sv_pok(sv) != 0
}

pub unsafe fn SvREFCNT_inc(sv: *mut SV) -> *mut SV {
    raw_SvREFCNT_inc(sv)
}

pub unsafe fn SvREFCNT_dec(sv: *mut SV) {
    raw_SvREFCNT_dec(sv)
}

pub unsafe fn newAV() -> *mut AV {
    raw_newAV()
}

pub unsafe fn av_push(av: *mut AV, val: *mut SV) {
    raw_av_push(av, val)
}

pub unsafe fn av_pop(av: *mut AV) -> *mut SV {
    raw_av_pop(av)
}

pub unsafe fn av_fetch(av: *mut AV, key: isize, lval: c_int) -> *mut *mut SV {
    raw_av_fetch(av, key, lval)
}

pub unsafe fn av_len(av: *mut AV) -> isize {
    raw_av_len(av)
}

pub unsafe fn newRV_noinc(sv: *mut SV) -> *mut SV {
    raw_newRV_noinc(sv)
}

pub unsafe fn SvRV(sv: *mut SV) -> *mut SV {
    raw_SvRV(sv)
}

pub unsafe fn SvROK(sv: *mut SV) -> c_int {
    raw_SvROK(sv)
}

pub unsafe fn xsub_frame() -> XsubFrame {
    let mut frame = XsubFrame {
        base: std::ptr::null_mut(),
        ax: 0,
        items: 0,
    };
    raw_xsub_frame(&mut frame);
    frame
}

pub unsafe fn xsub_return(n: i32, ax: i32) {
    raw_xsub_return(ax, n)
}

pub unsafe fn newXS(
    name: *const c_char,
    subaddr: unsafe extern "C" fn(*mut CV),
    filename: *const c_char,
) -> *mut CV {
    raw_newXS(name, subaddr, filename)
}

/// Extract the n-th argument from an XSUB stack frame.
pub unsafe fn xs_arg_sv(base: *const *mut SV, ax: i32, n: i32) -> *mut SV {
    *base.add((ax + n) as usize)
}

/// Extract an `i64` from a Perl SV argument.
///
/// Calls `sv_2iv` which coerces any scalar (string, float) to an integer.
pub unsafe fn sv_to_i64(sv: *mut SV) -> i64 {
    sv_2iv(sv) as i64
}

/// Extract an `f64` from a Perl SV argument.
///
/// Calls `sv_2nv` which coerces any scalar to a float.
pub unsafe fn sv_to_f64(sv: *mut SV) -> f64 {
    sv_2nv(sv)
}

/// Extract a `String` from a Perl SV argument.
///
/// Returns `None` if the SV does not have a string representation or if
/// the string is not valid UTF-8. The SV retains ownership; we copy the bytes.
pub unsafe fn sv_to_string(sv: *mut SV) -> Option<String> {
    let mut len: usize = 0;
    let ptr = sv_2pv_flags(sv, &mut len, 0);
    if ptr.is_null() {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    String::from_utf8(bytes.to_vec()).ok()
}

/// Create a new Perl SV from an `f64`.
///
/// The returned SV has refcount 1 and must be managed by the caller.
pub unsafe fn f64_to_sv(n: f64) -> *mut SV {
    newSVnv(n)
}

/// Create a new Perl SV from an `i64`.
///
/// The returned SV has refcount 1 and must be managed by the caller.
pub unsafe fn i64_to_sv(i: i64) -> *mut SV {
    newSViv(i as IV)
}

/// Push a `&[f64]` as a Perl AV (array).
///
/// Creates a new AV with one SV per element. The caller must manage the
/// AV's refcount.
pub unsafe fn f64_vec_to_av(values: &[f64]) -> *mut AV {
    let av = newAV();
    for &v in values {
        av_push(av, newSVnv(v));
    }
    av
}

/// Read a Perl AV as `Vec<f64>`.
///
/// Returns `None` if any element cannot be coerced to a float (e.g. a string
/// that is not numeric). Iterates from index 0 to `av_len(av)`.
pub unsafe fn av_to_f64_vec(av: *mut AV) -> Option<Vec<f64>> {
    let last_idx = av_len(av);
    if last_idx < 0 {
        return Some(Vec::new()); // empty array
    }
    let len = (last_idx + 1) as usize;
    let mut result = Vec::with_capacity(len);
    for i in 0..=last_idx {
        let slot = av_fetch(av, i, 0);
        if slot.is_null() {
            return None;
        }
        let sv = *slot;
        if sv.is_null() {
            return None;
        }
        result.push(sv_2nv(sv));
    }
    Some(result)
}

/// Die with a Rust string message. Never returns.
///
/// This is the safe wrapper around `croak`. It creates a null-terminated
/// C string from `msg` and calls Perl's `croak()`, which throws a Perl
/// exception (die) and `longjmp`s out of the current call frame.
pub unsafe fn die(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error in die)").unwrap());
    raw_croak_message(c_msg.as_ptr())
}

/// Warn with a Rust string message.
pub unsafe fn warn(msg: &str) {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("(error in warn)").unwrap());
    raw_warn_message(c_msg.as_ptr());
}

// ---------------------------------------------------------------------------
// XS boot helpers
// ---------------------------------------------------------------------------
//
// Perl calls `boot_Module` as an XSUB, not a plain C function. That means the
// bootstrap function must perform the XS handshake up front and run Perl's
// epilog before returning, otherwise DynaLoader observes a corrupted stack.

/// Handshake key matching Perl's `XS_SETXSUBFN_POPMARK | HSf_NOCHK`.
const XS_BOOT_HANDSHAKE_KEY_NOCHK: u32 = 0xFFFF00E0;

/// Begin a boot XSUB and return the stack offset Perl expects at epilog time.
pub unsafe fn xs_bootstrap(cv: *mut CV, file: *const c_char) -> StackOffset {
    Perl_xs_handshake(XS_BOOT_HANDSHAKE_KEY_NOCHK, cv.cast::<c_void>(), file)
}

/// Finish a boot XSUB after all `newXS` registrations have completed.
pub unsafe fn xs_boot_finish(ax: StackOffset) {
    raw_xs_boot_finish(ax);
}

// ---------------------------------------------------------------------------
// XS boot helpers
// ---------------------------------------------------------------------------
//
// Perl calls `boot_Module` as an XSUB, not a plain C function. That means the
// bootstrap function must perform the XS handshake up front and run Perl's
// epilog before returning, otherwise DynaLoader observes a corrupted stack.

/// Handshake key matching Perl's `XS_SETXSUBFN_POPMARK | HSf_NOCHK`.
const XS_BOOT_HANDSHAKE_KEY_NOCHK: u32 = 0xFFFF00E0;

/// Begin a boot XSUB and return the stack offset Perl expects at epilog time.
pub unsafe fn xs_bootstrap(cv: *mut CV, file: *const c_char) -> StackOffset {
    Perl_xs_handshake(XS_BOOT_HANDSHAKE_KEY_NOCHK, cv.cast::<c_void>(), file)
}

/// Finish a boot XSUB after all `newXS` registrations have completed.
pub unsafe fn xs_boot_finish(ax: StackOffset) {
    Perl_xs_boot_epilog(ax);
}

// ---------------------------------------------------------------------------
// xs_init! — define the XS boot function
// ---------------------------------------------------------------------------
//
// Every XS module must export a `boot_<ModuleName>` function. Perl's
// DynaLoader calls this when the module is first loaded with `use Module;`.
// The boot function registers all the XSUBs with Perl's symbol table.
//
// In a generated XS file, this is the function that the `BOOT:` section
// and `MODULE = ... PACKAGE = ...` directives create automatically.
// We replicate it here as a macro for zero-XS-toolchain Rust code.
//
// Usage:
//
// ```rust,ignore
// xs_init!(MyModule, |cv| {
//     // register functions here
// });
// ```

/// Define the boot function for a Perl XS module.
///
/// `$module` is a Rust identifier matching the Perl module name.
/// `$boot` is an expression of type `unsafe fn(*mut CV)`.
///
/// Generates a function named `boot_<$module>` that Perl's DynaLoader
/// will find and call when the module is loaded.
///
/// # Example
///
/// ```rust,ignore
/// unsafe fn my_boot(cv: *mut CV) {
///     // register XSUBs here
/// }
///
/// xs_init!(MyMath, my_boot);
/// ```
#[macro_export]
macro_rules! xs_init {
    ($module:ident, $boot:expr) => {
        ::std::concat_idents::concat_idents!(fn_name = boot_, $module {
            /// The XS module boot function. Called by Perl's DynaLoader on `use Module;`.
            ///
            /// # Safety
            /// Only called by the Perl runtime during module loading.
            #[no_mangle]
            pub unsafe extern "C" fn fn_name(cv: *mut $crate::CV) {
                ($boot)(cv);
            }
        });
    };
}
