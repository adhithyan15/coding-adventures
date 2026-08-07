//! # Golden divergence guard for the native lisp runtime (LANG77).
//!
//! The AOT pipeline ships a C implementation of `lispy-runtime`'s value
//! model (`runtime/dynval_runtime.c`) so that lisp-family programs compile to
//! native executables.  That C runtime and the Rust `lispy-runtime` crate
//! (used by the VM/JIT) are **two implementations of one documented ABI** —
//! the 3-bit-tagged 64-bit `LispyValue`.  This test is what makes that split
//! safe instead of a duplication hazard: it links the C runtime into this
//! test binary (cargo links the `cc`-built archive automatically) and asserts
//! that every tag constant and encoding the C side produces matches
//! `lispy-runtime`'s canonical `pub const`s and constructors.
//!
//! If a future change to `lispy-runtime/src/value.rs` moves a tag or alters
//! an encoding, these assertions fail at `cargo test` — the two runtimes can
//! never silently drift into corrupting each other's values.
//!
//! See `code/specs/LANG77-lisp-native-runtime.md`.
//!
//! ## Why this runs on every host (including macOS)
//!
//! There is a separate, pre-existing limitation that runtime helpers don't
//! link into AOT-*produced* macOS executables.  That does not apply here:
//! linking the static archive into *this Rust test binary* uses the normal
//! host linker, so the golden test runs on the dev host (macOS arm64) and on
//! every CI runner alike.

use dynval_runtime::{
    LispyValue, TAG_BITS, TAG_FALSE, TAG_HEAP, TAG_INT, TAG_NIL, TAG_SYMBOL, TAG_TRUE,
};

// The C runtime's exported ABI.  These symbols live in the
// `libtwig_aot_runtime` archive that `build.rs` compiles from
// `runtime/dynval_runtime.c` and that cargo links into this test binary.
//
// SAFETY: each function is a pure value transform over `u64`s with the tag
// discipline documented in LANG77; `car`/`cdr` are only ever called here on
// values we just built with `cons`, so the heap dereference is in-bounds.
extern "C" {
    fn __dyn_box_int(n: i64) -> u64;
    fn __dyn_unbox_int(v: u64) -> i64;
    fn __dyn_nil() -> u64;
    fn __dyn_cons(car: u64, cdr: u64) -> u64;
    fn __dyn_car(pair: u64) -> u64;
    fn __dyn_cdr(pair: u64) -> u64;
    fn __dyn_pair_p(v: u64) -> u64;
    fn __dyn_not(v: u64) -> u64;
    fn __dyn_truthy(v: u64) -> i64;
    fn __dyn_equal(a: u64, b: u64) -> u64;
    fn __dyn_make_symbol(name: *const u8, len: i64) -> u64;

    fn __dyn_tag_int() -> u64;
    fn __dyn_tag_nil() -> u64;
    fn __dyn_tag_symbol() -> u64;
    fn __dyn_tag_false() -> u64;
    fn __dyn_tag_true() -> u64;
    fn __dyn_tag_heap() -> u64;
    fn __dyn_tag_mask() -> u64;
}

// ---------------------------------------------------------------------------
// 1. Tag constants — pinned against lispy-runtime's `pub const`s.
// ---------------------------------------------------------------------------

#[test]
fn c_tag_constants_match_rust() {
    unsafe {
        assert_eq!(__dyn_tag_int(), TAG_INT, "TAG_INT drift");
        assert_eq!(__dyn_tag_nil(), TAG_NIL, "TAG_NIL drift");
        assert_eq!(__dyn_tag_symbol(), TAG_SYMBOL, "TAG_SYMBOL drift");
        assert_eq!(__dyn_tag_false(), TAG_FALSE, "TAG_FALSE drift");
        assert_eq!(__dyn_tag_true(), TAG_TRUE, "TAG_TRUE drift");
        assert_eq!(__dyn_tag_heap(), TAG_HEAP, "TAG_HEAP drift");
        assert_eq!(__dyn_tag_mask(), TAG_BITS, "TAG_BITS drift");
    }
}

// ---------------------------------------------------------------------------
// 2. Integer boxing — pinned against `LispyValue::int(_).bits()`.
// ---------------------------------------------------------------------------

#[test]
fn c_box_int_matches_rust_constructor() {
    // A spread of values including zero, small, negative, and the extremes
    // of the representable ±2^60 range.
    for n in [0_i64, 1, 7, -1, 42, -42, 1 << 59, -(1 << 59), (1 << 60) - 1, -(1 << 60)] {
        let c = unsafe { __dyn_box_int(n) };
        assert_eq!(c, LispyValue::int(n).bits(), "box_int({n}) drift");
        // The boxed value is tagged as an integer …
        assert_eq!(c & TAG_BITS, TAG_INT, "box_int({n}) wrong tag");
        // … and unboxing round-trips with sign extension.
        assert_eq!(unsafe { __dyn_unbox_int(c) }, n, "unbox_int round-trip {n}");
    }
}

// ---------------------------------------------------------------------------
// 3. Singletons.
// ---------------------------------------------------------------------------

#[test]
fn c_nil_matches_rust() {
    assert_eq!(unsafe { __dyn_nil() }, LispyValue::NIL.bits());
}

// ---------------------------------------------------------------------------
// 4. Cons / car / cdr — structural round-trip with correct heap tag.
// ---------------------------------------------------------------------------

#[test]
fn c_cons_car_cdr_round_trip() {
    let a = unsafe { __dyn_box_int(7) };
    let b = unsafe { __dyn_box_int(9) };
    let pair = unsafe { __dyn_cons(a, b) };

    // A cons is heap-tagged.
    assert_eq!(pair & TAG_BITS, TAG_HEAP, "cons result not heap-tagged");
    // car/cdr recover the exact boxed values.
    assert_eq!(unsafe { __dyn_car(pair) }, a, "car mismatch");
    assert_eq!(unsafe { __dyn_cdr(pair) }, b, "cdr mismatch");
    // And those decode back to 7 / 9.
    assert_eq!(unsafe { __dyn_unbox_int(__dyn_car(pair)) }, 7);
    assert_eq!(unsafe { __dyn_unbox_int(__dyn_cdr(pair)) }, 9);
}

#[test]
fn c_nested_cons_round_trip() {
    // (CONS 1 (CONS 2 nil)) — a two-element list.
    let one = unsafe { __dyn_box_int(1) };
    let two = unsafe { __dyn_box_int(2) };
    let nil = unsafe { __dyn_nil() };
    let inner = unsafe { __dyn_cons(two, nil) };
    let outer = unsafe { __dyn_cons(one, inner) };

    assert_eq!(unsafe { __dyn_unbox_int(__dyn_car(outer)) }, 1);
    let tail = unsafe { __dyn_cdr(outer) };
    assert_eq!(unsafe { __dyn_unbox_int(__dyn_car(tail)) }, 2);
    assert_eq!(unsafe { __dyn_cdr(tail) }, nil);
}

// ---------------------------------------------------------------------------
// 5. pair? — tagged boolean, matching lispy truthiness.
// ---------------------------------------------------------------------------

#[test]
fn c_pair_p_returns_tagged_booleans() {
    let pair = unsafe { __dyn_cons(__dyn_box_int(1), __dyn_nil()) };
    assert_eq!(unsafe { __dyn_pair_p(pair) }, LispyValue::TRUE.bits());
    assert_eq!(unsafe { __dyn_pair_p(__dyn_box_int(7)) }, LispyValue::FALSE.bits());
    assert_eq!(unsafe { __dyn_pair_p(__dyn_nil()) }, LispyValue::FALSE.bits());
}

// ---------------------------------------------------------------------------
// 6. not — false iff #f or nil.
// ---------------------------------------------------------------------------

#[test]
fn c_not_follows_lispy_truthiness() {
    let t = LispyValue::TRUE.bits();
    let f = LispyValue::FALSE.bits();
    let nil = unsafe { __dyn_nil() };
    assert_eq!(unsafe { __dyn_not(f) }, t, "not(#f) = #t");
    assert_eq!(unsafe { __dyn_not(nil) }, t, "not(nil) = #t");
    assert_eq!(unsafe { __dyn_not(t) }, f, "not(#t) = #f");
    // 0 is a *truthy* integer in lispy — not(0) = #f.
    assert_eq!(unsafe { __dyn_not(__dyn_box_int(0)) }, f, "not(0) = #f");
}

/// `dyn_truthy` returns a RAW machine 0/1 (for `jmp_if_false`): false iff
/// `#f` or nil, true for everything else (including the integer 0 and pairs).
#[test]
fn c_truthy_returns_raw_bool() {
    let nil = unsafe { __dyn_nil() };
    assert_eq!(unsafe { __dyn_truthy(LispyValue::FALSE.bits()) }, 0, "truthy(#f) = 0");
    assert_eq!(unsafe { __dyn_truthy(nil) }, 0, "truthy(nil) = 0");
    assert_eq!(unsafe { __dyn_truthy(LispyValue::TRUE.bits()) }, 1, "truthy(#t) = 1");
    // A boxed integer 0 is truthy (only #f and nil are false).
    assert_eq!(unsafe { __dyn_truthy(__dyn_box_int(0)) }, 1, "truthy(0) = 1");
    // A cons cell (pair) is truthy.
    let pair = unsafe { __dyn_cons(__dyn_box_int(1), nil) };
    assert_eq!(unsafe { __dyn_truthy(pair) }, 1, "truthy(pair) = 1");
}

// ---------------------------------------------------------------------------
// 7. equal? — structural deep equality.
// ---------------------------------------------------------------------------

#[test]
fn c_equal_atoms() {
    let t = LispyValue::TRUE.bits();
    let f = LispyValue::FALSE.bits();
    assert_eq!(unsafe { __dyn_equal(__dyn_box_int(5), __dyn_box_int(5)) }, t);
    assert_eq!(unsafe { __dyn_equal(__dyn_box_int(5), __dyn_box_int(6)) }, f);
    assert_eq!(unsafe { __dyn_equal(__dyn_nil(), __dyn_nil()) }, t);
}

#[test]
fn c_equal_structural_pairs() {
    // (1 . 2) equals a freshly-built (1 . 2) even though the pointers differ.
    let p1 = unsafe { __dyn_cons(__dyn_box_int(1), __dyn_box_int(2)) };
    let p2 = unsafe { __dyn_cons(__dyn_box_int(1), __dyn_box_int(2)) };
    let p3 = unsafe { __dyn_cons(__dyn_box_int(1), __dyn_box_int(3)) };
    assert_eq!(unsafe { __dyn_equal(p1, p2) }, LispyValue::TRUE.bits(), "(1.2)=(1.2)");
    assert_eq!(unsafe { __dyn_equal(p1, p3) }, LispyValue::FALSE.bits(), "(1.2)≠(1.3)");
    // A pair and an atom are never equal.
    assert_eq!(
        unsafe { __dyn_equal(p1, __dyn_box_int(1)) },
        LispyValue::FALSE.bits(),
        "pair ≠ atom",
    );
}

// ---------------------------------------------------------------------------
// 8. Symbols — interning gives same name ⇒ same id, with the symbol tag.
// ---------------------------------------------------------------------------

#[test]
fn c_make_symbol_interns_consistently() {
    let foo1 = unsafe { __dyn_make_symbol(b"FOO".as_ptr(), 3) };
    let foo2 = unsafe { __dyn_make_symbol(b"FOO".as_ptr(), 3) };
    let bar = unsafe { __dyn_make_symbol(b"BAR".as_ptr(), 3) };

    // Same name interns to the identical bit pattern (so EQ is bit-equality).
    assert_eq!(foo1, foo2, "FOO interned inconsistently");
    // Distinct names get distinct ids.
    assert_ne!(foo1, bar, "FOO and BAR collided");
    // Correct tag, and the id lives in the high 32 bits.
    assert_eq!(foo1 & TAG_BITS, TAG_SYMBOL, "symbol wrong tag");
    assert_eq!(bar & TAG_BITS, TAG_SYMBOL, "symbol wrong tag");
    // Two distinct symbols are not `equal?`; a symbol equals itself.
    assert_eq!(unsafe { __dyn_equal(foo1, foo2) }, LispyValue::TRUE.bits());
    assert_eq!(unsafe { __dyn_equal(foo1, bar) }, LispyValue::FALSE.bits());
}

#[test]
fn c_make_symbol_empty_name() {
    // Degenerate but well-defined: the empty symbol interns consistently.
    let e1 = unsafe { __dyn_make_symbol(b"".as_ptr(), 0) };
    let e2 = unsafe { __dyn_make_symbol(b"".as_ptr(), 0) };
    assert_eq!(e1, e2);
    assert_eq!(e1 & TAG_BITS, TAG_SYMBOL);
}
