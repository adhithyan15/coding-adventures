//! # `extern "C"` ABI surface — the `lispy_*` symbols.
//!
//! Per LANG20 §"Per-language symbols", each `<lang>-runtime` crate
//! exposes `extern "C"` entry points that JIT- and AOT-emitted
//! code calls directly.  The runtime substrate (`lang-runtime-core`)
//! ships the language-agnostic `rt_*` symbols; this module ships
//! the language-specific Lispy operations.
//!
//! ## ABI shape
//!
//! Every value crosses the boundary as a single `u64` (the
//! [`LispyValue`] bits).  Pointers and lengths use C-friendly
//! types (`*const u8` + `usize`, `u32` for counts).  Return
//! values are likewise `u64`.
//!
//! ## Calling convention
//!
//! System-V x86_64 / AAPCS64 / RV64 ELF psABI for the platform.
//! Standard C calling convention — the JIT codegen knows how to
//! emit a direct call (`call lispy_cons` on x86_64; `bl lispy_cons`
//! on AArch64).
//!
//! ## Error handling
//!
//! PR 2 ships a **panic-on-misuse** policy: the `lispy_*` symbols
//! are intended to be called from JIT/AOT codegen that has
//! already type-checked its inputs (via guards).  A wrong-type
//! call panics with a clear message.  PR 4+ will introduce the
//! shared `rt_*` error channel (a thread-local `Option<RuntimeError>`
//! the caller polls); these symbols will then signal through it
//! instead of panicking.
//!
//! Until PR 4+, **production callers should use the Rust API
//! ([`crate::LispyBinding`])**, not the `extern "C"` surface, to
//! get proper error returns.  The C surface exists now so
//! later-PR JIT/AOT codegen has stable symbols to link against.
//!
//! ## What's locked
//!
//! - Symbol names: `lispy_cons`, `lispy_car`, `lispy_cdr`,
//!   `lispy_make_symbol`, `lispy_make_closure`,
//!   `lispy_apply_closure`.  Adding a new operation gets a new
//!   symbol; renaming an existing one is a breaking ABI change.
//! - Argument and return types: all `u64` (values), `*const u8`
//!   (byte slices), `usize` (lengths).  No `bool`, no `&str`.
//! - Calling convention: platform-default System V / AAPCS / RV64.
//!
//! ## What's not locked yet
//!
//! - The error channel (PR 4+ — see "Error handling" above).
//! - Whether `lispy_apply_closure` blocks the calling thread or
//!   trampolines through a cooperative scheduler (concurrency
//!   model is out of LANG20 scope per §"Out of scope").

use lang_runtime_core::SymbolId;

use crate::heap::{self, Closure};
use crate::intern;
use crate::value::LispyValue;

// ---------------------------------------------------------------------------
// Cons cell operations
// ---------------------------------------------------------------------------

/// `lispy_cons(car, cdr) -> u64` — allocate a cons cell and
/// return the tagged `LispyValue` bits.
///
/// # Safety
///
/// `car` and `cdr` must each be the bits of a value previously
/// returned by a `lispy_*` constructor (or a constant such as
/// `LispyValue::NIL.bits()` or `LispyValue::int(n).bits()`).  An
/// arbitrary `u64` whose low 3 bits happen to equal `0b111`
/// would form a fake heap pointer; storing it in the new cell
/// and later passing the cell through `lispy_car` would
/// dereference an attacker-chosen address.
#[no_mangle]
pub unsafe extern "C" fn lispy_cons(car: u64, cdr: u64) -> u64 {
    // SAFETY: caller upholds that both bits are valid LispyValues.
    let car_v = unsafe { LispyValue::from_raw_bits(car) };
    let cdr_v = unsafe { LispyValue::from_raw_bits(cdr) };
    heap::alloc_cons(car_v, cdr_v).bits()
}

/// `lispy_car(pair) -> u64` — extract the first element of a
/// cons cell.
///
/// # Safety
///
/// `pair` must be the bits of a value previously returned by
/// `lispy_cons`.  An arbitrary `u64` with low 3 bits = `0b111`
/// would form a fake heap pointer that the function dereferences.
///
/// # Panics
///
/// Panics if the value is well-formed but not a cons cell.
/// JIT/AOT codegen emits a type guard before this call; the
/// panic indicates the speculation was wrong (JIT bug or stale IC).
#[no_mangle]
pub unsafe extern "C" fn lispy_car(pair: u64) -> u64 {
    let v = unsafe { LispyValue::from_raw_bits(pair) };
    // SAFETY: caller upholds that `pair` is a valid LispyValue.
    unsafe { heap::car(v) }
        .unwrap_or_else(|| panic!("lispy_car: argument {:#x} is not a cons cell", pair))
        .bits()
}

/// `lispy_cdr(pair) -> u64` — extract the rest of a cons cell.
///
/// # Safety
///
/// Same contract as [`lispy_car`]: `pair` must be the bits of a
/// value previously returned by `lispy_cons`.
///
/// # Panics
///
/// Same as [`lispy_car`].
#[no_mangle]
pub unsafe extern "C" fn lispy_cdr(pair: u64) -> u64 {
    let v = unsafe { LispyValue::from_raw_bits(pair) };
    // SAFETY: caller upholds that `pair` is a valid LispyValue.
    unsafe { heap::cdr(v) }
        .unwrap_or_else(|| panic!("lispy_cdr: argument {:#x} is not a cons cell", pair))
        .bits()
}

// ---------------------------------------------------------------------------
// Symbol interning
// ---------------------------------------------------------------------------

/// `lispy_make_symbol(bytes, len) -> u64` — intern a symbol from
/// a UTF-8 byte slice and return the tagged immediate symbol value.
///
/// # Safety
///
/// `bytes` must point at exactly `len` valid bytes.  The bytes
/// must be valid UTF-8 (the intern table requires owned `String`s);
/// invalid UTF-8 panics.
#[no_mangle]
pub unsafe extern "C" fn lispy_make_symbol(bytes: *const u8, len: usize) -> u64 {
    // SAFETY: caller upholds (bytes, len) describes a live byte slice.
    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let name = std::str::from_utf8(slice)
        .unwrap_or_else(|e| panic!("lispy_make_symbol: invalid UTF-8: {e}"));
    let id = intern::intern(name);
    LispyValue::symbol(id).bits()
}

// ---------------------------------------------------------------------------
// Closure construction
// ---------------------------------------------------------------------------

/// `lispy_make_closure(fn_name_id, captures, n) -> u64` — allocate
/// a closure with the given underlying function name and captured
/// values, returning the tagged closure value.
///
/// `fn_name_id` is a [`SymbolId`] (as `u32`) that the
/// IIRFunction's name was interned to.  `captures` points at `n`
/// pre-tagged `LispyValue` bits.
///
/// # Safety
///
/// `captures` must point at exactly `n` valid `u64` values.
#[no_mangle]
pub unsafe extern "C" fn lispy_make_closure(
    fn_name_id: u32,
    captures: *const u64,
    n: u32,
) -> u64 {
    // `slice::from_raw_parts` requires the pointer to be aligned
    // and non-null even when `len == 0`.  Branch on the empty
    // case so callers can pass `null + 0` for the no-captures
    // scenario.
    let captures_vec: Vec<LispyValue> = if n == 0 {
        Vec::new()
    } else {
        // SAFETY: caller upholds (captures, n) describes a live array
        // of valid LispyValue bits.
        let raw = unsafe { std::slice::from_raw_parts(captures, n as usize) };
        raw.iter()
            .map(|&bits| unsafe { LispyValue::from_raw_bits(bits) })
            .collect()
    };
    heap::alloc_closure(SymbolId(fn_name_id), captures_vec).bits()
}

// ---------------------------------------------------------------------------
// Closure application
// ---------------------------------------------------------------------------

/// `lispy_apply_closure(closure, args, n) -> u64` — apply a
/// closure to user-supplied arguments and return the result.
///
/// Per the TW00 apply-closure semantics, the closure's captured
/// environment is prepended to `args` before invoking the
/// underlying function.
///
/// # PR 2 placeholder
///
/// This entry point cannot actually invoke an `IIRFunction` until
/// PR 4 wires `vm-core` into `DispatchCx`.  PR 2 panics with a
/// descriptive message so a misconfigured caller fails loudly
/// instead of silently misbehaving.
///
/// # Safety
///
/// Same as [`lispy_make_closure`] (`args` must be a live slice).
#[no_mangle]
pub unsafe extern "C" fn lispy_apply_closure(
    closure: u64,
    args: *const u64,
    n: u32,
) -> u64 {
    // SAFETY: caller upholds that `closure` is a valid LispyValue
    // (specifically a closure heap value).
    let v = unsafe { LispyValue::from_raw_bits(closure) };
    if !unsafe { heap::is_closure(v) } {
        panic!("lispy_apply_closure: argument {closure:#x} is not a closure");
    }
    // PR 2 doesn't actually invoke the closure — but we still
    // validate the args slice the caller provided so this entry
    // point's safety contract is exercised.  If/when the
    // implementation lands in PR 4, the validation here keeps
    // the same shape.
    if n != 0 {
        // SAFETY: caller upholds (args, n) describes a live array.
        let _ = unsafe { std::slice::from_raw_parts(args, n as usize) };
    }
    panic!(
        "lispy_apply_closure: closure dispatch is not wired in PR 2; \
         lands in PR 4 (vm-core wiring)"
    );
}

// ---------------------------------------------------------------------------
// Inspection helpers (used by tests / debuggers)
// ---------------------------------------------------------------------------

/// `lispy_closure_capture_count(closure) -> u32` — return the
/// number of captured values in a closure.
///
/// # Safety
///
/// Same contract as [`lispy_car`]: `closure` must be the bits of
/// a valid `LispyValue` previously produced by this crate.
///
/// # Panics
///
/// Panics if the value is well-formed but not a closure.
#[no_mangle]
pub unsafe extern "C" fn lispy_closure_capture_count(closure: u64) -> u32 {
    // SAFETY: caller upholds the safety contract.
    let v = unsafe { LispyValue::from_raw_bits(closure) };
    let clos: &Closure = unsafe { heap::as_closure(v) }
        .unwrap_or_else(|| panic!("lispy_closure_capture_count: not a closure"));
    clos.capture_count() as u32
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lispy_cons_round_trip() {
        // SAFETY: the inputs are bits of valid LispyValues
        // (LispyValue::int constructors).
        let v = unsafe { lispy_cons(LispyValue::int(1).bits(), LispyValue::int(2).bits()) };
        let pair = unsafe { LispyValue::from_raw_bits(v) };
        assert!(pair.is_heap());
        unsafe {
            assert_eq!(lispy_car(v), LispyValue::int(1).bits());
            assert_eq!(lispy_cdr(v), LispyValue::int(2).bits());
        }
    }

    // Note: panicking-across-FFI tests are intentionally omitted.
    // Rust panics that cross an `extern "C"` boundary are
    // undefined behaviour (the runtime aborts the process), so we
    // can't `#[should_panic]` against the FFI surface.  The
    // underlying Rust paths (`heap::car`, `heap::cdr`,
    // `heap::is_closure`) are tested via the binding tests in
    // `binding::tests`.

    #[test]
    fn lispy_make_symbol_interns() {
        let bytes = b"abi_symbol_test";
        let v = unsafe { lispy_make_symbol(bytes.as_ptr(), bytes.len()) };
        let val = unsafe { LispyValue::from_raw_bits(v) };
        assert!(val.is_symbol());
        // Same name interns to the same id.
        let v2 = unsafe { lispy_make_symbol(bytes.as_ptr(), bytes.len()) };
        assert_eq!(v, v2);
    }

    #[test]
    fn lispy_make_symbol_handles_empty_bytes() {
        let v = unsafe { lispy_make_symbol(b"".as_ptr(), 0) };
        let val = unsafe { LispyValue::from_raw_bits(v) };
        assert!(val.is_symbol());
        assert_eq!(val.as_symbol(), Some(SymbolId::EMPTY));
    }

    #[test]
    fn lispy_make_closure_records_captures() {
        let captures = [LispyValue::int(1).bits(), LispyValue::int(2).bits()];
        let v = unsafe { lispy_make_closure(7, captures.as_ptr(), 2) };
        unsafe {
            assert_eq!(lispy_closure_capture_count(v), 2);
            let clos = heap::as_closure(LispyValue::from_raw_bits(v)).unwrap();
            assert_eq!(clos.fn_name, SymbolId(7));
        }
    }

    #[test]
    fn lispy_make_closure_with_zero_captures() {
        let v = unsafe { lispy_make_closure(3, std::ptr::null(), 0) };
        unsafe {
            assert_eq!(lispy_closure_capture_count(v), 0);
        }
    }

    // `lispy_apply_closure` panic tests are intentionally omitted
    // for the same reason as `lispy_car` / `lispy_cdr` (panic-
    // across-FFI is UB).  The closure validation logic
    // (`heap::is_closure`) is tested via the binding tests.
}
