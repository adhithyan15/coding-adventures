//! # `LispyValue` — the tagged-i64 value representation.
//!
//! Every Lisp / Scheme / Twig / Clojure value is a single `u64` with
//! a 3-bit tag in the low bits.  This is the **immediate-tag**
//! scheme — V8-style for integers, with extra tags for nil / true /
//! false / interned symbol so those don't need heap allocations.
//!
//! ## Tag layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┬─────┐
//! │            payload (high 61 bits)                        │ tag │
//! └─────────────────────────────────────────────────────────┴─────┘
//!  63                                                       3   2 0
//! ```
//!
//! | Tag (low 3 bits) | Kind | Payload |
//! |------------------|------|---------|
//! | `0b000` | Integer | high 61 bits, signed (range ±2⁶⁰ ≈ ±10¹⁸) |
//! | `0b001` | Nil singleton | (none) |
//! | `0b010` | Symbol immediate | high 32 bits = [`SymbolId`] |
//! | `0b011` | False singleton | (none) |
//! | `0b101` | True singleton | (none) |
//! | `0b111` | Heap pointer | the full word with low 3 bits cleared |
//! | `0b100` | reserved | future: char / weak ref / boxed-flonum |
//! | `0b110` | reserved | future: native-fn handle |
//!
//! ## Why these specific tags
//!
//! `TAG_INT = 0b000` makes the common case (integer arithmetic) the
//! cheapest: extracting the value is a single arithmetic shift right
//! by 3.  Comparison of two integers is direct word equality without
//! masking — both have the same low 3 bits.
//!
//! Nil / false / true are singletons whose entire bit pattern is the
//! constant — comparing a `LispyValue` to [`LispyValue::NIL`] is one
//! `cmp` instruction.
//!
//! Symbols carry their [`SymbolId`] in the high 32 bits — interning
//! is owned by the runtime intern table, so two `'foo`s anywhere in
//! the program share the same id and compare equal in O(1).
//!
//! `TAG_HEAP = 0b111` is chosen so heap pointers can be extracted by
//! a single `AND !0b111` — clearing the low 3 bits — provided every
//! heap allocation is 8-byte aligned (which they are by Rust's
//! default layout rules for any struct containing a `u64`).
//!
//! ## Invariants
//!
//! - `Copy`, `Eq`, `Hash` — `LispyValue` is a thin wrapper around a
//!   `u64`.
//! - **8 bytes**, asserted at compile time via `const _: () =
//!   assert!(...)` so an accidental enlargement breaks the build.
//! - Heap pointers are always 8-byte aligned; the OR-with-tag trick
//!   in [`LispyValue::from_heap`] is sound under that invariant.
//!
//! ## What this PR ships
//!
//! PR 2 implements the value type, immediate constructors, and tag
//! introspection.  The heap allocator is intentionally simple
//! (`Box::leak` per call — a non-collecting "GC") because the real
//! GC integration lands in LANG16 work; once `gc-core` is wired,
//! [`LispyValue::from_heap`] keeps the same shape and only the
//! allocator changes.

use lang_runtime_core::SymbolId;

// ---------------------------------------------------------------------------
// Tag constants
// ---------------------------------------------------------------------------

/// Bit mask covering the low 3 tag bits.
pub const TAG_BITS: u64 = 0b111;

/// Tag for an immediate signed integer.  Payload = high 61 bits
/// interpreted as a signed value (`(value << 3) | TAG_INT`).
pub const TAG_INT: u64 = 0b000;

/// Tag for the singleton `nil`.  Entire word equals this constant.
pub const TAG_NIL: u64 = 0b001;

/// Tag for an immediate interned symbol.  Payload = high 32 bits
/// interpreted as a [`SymbolId`].
pub const TAG_SYMBOL: u64 = 0b010;

/// Tag for the singleton `#f`.  Entire word equals this constant.
pub const TAG_FALSE: u64 = 0b011;

/// Tag for the singleton `#t`.  Entire word equals this constant.
pub const TAG_TRUE: u64 = 0b101;

/// Tag for a heap pointer.  The full word with low 3 bits cleared
/// is the pointer to a [`crate::heap::ConsCell`] / [`crate::heap::Closure`]
/// (etc.) header.
pub const TAG_HEAP: u64 = 0b111;

/// Maximum integer value representable as an immediate (`2⁶⁰ - 1`).
/// Values larger require a future bignum heap object.
pub const INT_MAX: i64 = (1 << 60) - 1;

/// Minimum integer value representable as an immediate (`-2⁶⁰`).
pub const INT_MIN: i64 = -(1 << 60);

// ---------------------------------------------------------------------------
// LispyValue
// ---------------------------------------------------------------------------

/// A single Lisp/Scheme/Twig/Clojure value: 64 bits with a 3-bit
/// tag in the low bits.
///
/// See module docs for the tag layout and the ABI contract.  This
/// type is `Copy` and exactly 8 bytes; the LANG20 runtime treats
/// it as an opaque machine word at boundary crossings.
///
/// # Privacy
///
/// The inner `u64` is **private**.  Constructing a `LispyValue`
/// from arbitrary bits is unsafe because a caller could fabricate
/// a heap-tagged value pointing at an arbitrary address; safe
/// Rust must use the typed constructors (`int`, `bool`, `symbol`,
/// `from_heap`, `NIL`/`TRUE`/`FALSE`).  FFI callers receive
/// `u64`s and use [`LispyValue::from_raw_bits`] (unsafe) to
/// reconstruct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LispyValue(u64);

// Compile-time ABI check — the LANG20 contract requires Value
// to be exactly 8 bytes (LangBinding::Value's bound is Copy +
// 'static; this assertion is what catches an accidental
// enlargement at compile time).
const _: () = assert!(std::mem::size_of::<LispyValue>() == 8);

impl LispyValue {
    // ── Singletons ───────────────────────────────────────────────────

    /// The `nil` value — the empty-list / null-reference sentinel.
    /// Exactly one bit pattern: `0b001`.
    pub const NIL: LispyValue = LispyValue(TAG_NIL);

    /// The `#t` boolean.
    pub const TRUE: LispyValue = LispyValue(TAG_TRUE);

    /// The `#f` boolean.
    pub const FALSE: LispyValue = LispyValue(TAG_FALSE);

    // ── Constructors ─────────────────────────────────────────────────

    /// Construct an immediate integer.
    ///
    /// The integer is left-shifted by 3 to make room for the tag.
    /// Values outside the range ±2⁶⁰ get truncated by the shift —
    /// callers that need full-range integers should box them as a
    /// future heap object (PR 4+).  In debug builds, out-of-range
    /// values trigger an assertion so test cases catch silent
    /// truncation.
    ///
    /// # Examples
    ///
    /// ```
    /// use lispy_runtime::LispyValue;
    /// let v = LispyValue::int(42);
    /// assert_eq!(v.as_int(), Some(42));
    /// ```
    #[inline]
    pub const fn int(n: i64) -> LispyValue {
        // Range check: shift-left by 3 must not lose the sign bit.
        // i.e. the value must fit in 61 signed bits.
        debug_assert!(
            n >= INT_MIN && n <= INT_MAX,
            "LispyValue::int: value out of range — would truncate. Box as a future big-int instead.",
        );
        LispyValue(((n as u64) << 3) | TAG_INT)
    }

    /// Construct an immediate boolean.
    #[inline]
    pub const fn bool(b: bool) -> LispyValue {
        if b { Self::TRUE } else { Self::FALSE }
    }

    /// Construct an immediate symbol from its [`SymbolId`].
    ///
    /// The id occupies the high 32 bits; the low 32 carry only the
    /// tag.  Two symbols with the same id compare bitwise-equal —
    /// `eq?` semantics fall out for free.
    #[inline]
    pub const fn symbol(id: SymbolId) -> LispyValue {
        LispyValue(((id.as_u32() as u64) << 32) | TAG_SYMBOL)
    }

    /// Construct a heap-pointer value.
    ///
    /// # Safety
    ///
    /// - `ptr` must be 8-byte aligned (the OR-with-tag trick relies
    ///   on the low 3 bits being zero).  The check is hard, not
    ///   debug-only — an unaligned pointer would silently corrupt
    ///   the encoded address.
    /// - `ptr` must point at an [`crate::ObjectHeader`] followed
    ///   by a payload whose layout the runtime can decode (i.e.
    ///   `class_or_kind` matches a registered class).
    /// - The pointer must remain live for as long as any
    ///   `LispyValue` references it.  In PR 2 the allocator is
    ///   `Box::leak` (intentionally non-collecting), so live-for-
    ///   ever satisfies this; once LANG16 GC lands, callers
    ///   maintain liveness via the GC's root set.
    ///
    /// Marked `unsafe` because the function is `pub` — a safe
    /// constructor would let safe Rust forge fake heap values and
    /// trigger UB when the runtime later dereferences them.
    /// Internal callers (`heap::alloc_cons` / `alloc_closure`)
    /// satisfy these invariants by construction.
    ///
    /// # Provenance
    ///
    /// We use `expose_provenance` rather than `as u64` so the
    /// pointer's provenance is added to the per-thread "exposed
    /// set".  Recovering the pointer via [`as_heap_ptr`] uses
    /// [`std::ptr::with_exposed_provenance`] to pull a usable
    /// pointer back out of that set.  This is the strict-
    /// provenance idiom for tagged-pointer schemes (the only
    /// alternative is to lose provenance and let Miri reject
    /// every dereference).
    #[inline]
    pub unsafe fn from_heap<T>(ptr: *const T) -> LispyValue {
        let raw: u64 = ptr.expose_provenance() as u64;
        // Hard assert (not debug-only): an unaligned pointer is
        // a programmer bug at every callsite, and silently
        // corrupting the address would be much worse than a
        // panic.
        assert_eq!(
            raw & TAG_BITS,
            0,
            "heap pointer must be 8-byte aligned to use the OR-with-tag scheme \
             — got {ptr:p} with low 3 bits = {:b}",
            raw & TAG_BITS,
        );
        LispyValue(raw | TAG_HEAP)
    }

    /// Reconstruct a `LispyValue` from its raw `u64` bits.
    ///
    /// # Safety
    ///
    /// The caller must ensure `bits` was originally produced by a
    /// safe constructor of `LispyValue` (or equivalently, came
    /// from a prior `lispy_*` FFI call).  An arbitrary `u64` may
    /// have a heap-tag pattern (`bits & 0b111 == 0b111`) without
    /// a real heap allocation behind it; downstream
    /// `heap::car` / `cdr` / `as_closure` would then dereference
    /// an attacker-controlled address.
    ///
    /// FFI entry points (`lispy_cons` etc.) are `unsafe extern "C"`
    /// and use this constructor inside their `unsafe` block.
    #[inline]
    pub const unsafe fn from_raw_bits(bits: u64) -> LispyValue {
        LispyValue(bits)
    }

    // ── Tag introspection ────────────────────────────────────────────

    /// Return the low 3 tag bits.  Constant-time.
    #[inline]
    pub const fn tag(self) -> u64 {
        self.0 & TAG_BITS
    }

    /// `true` iff this value is an immediate integer.
    #[inline]
    pub const fn is_int(self) -> bool {
        self.tag() == TAG_INT
    }

    /// `true` iff this value is `nil`.
    #[inline]
    pub const fn is_nil(self) -> bool {
        self.0 == TAG_NIL
    }

    /// `true` iff this value is `#t`.
    #[inline]
    pub const fn is_true(self) -> bool {
        self.0 == TAG_TRUE
    }

    /// `true` iff this value is `#f`.
    #[inline]
    pub const fn is_false(self) -> bool {
        self.0 == TAG_FALSE
    }

    /// `true` iff this value is a boolean (`#t` or `#f`).
    #[inline]
    pub const fn is_bool(self) -> bool {
        self.is_true() || self.is_false()
    }

    /// `true` iff this value is an immediate symbol.
    #[inline]
    pub const fn is_symbol(self) -> bool {
        self.tag() == TAG_SYMBOL
    }

    /// `true` iff this value is a heap pointer (cons / closure / …).
    #[inline]
    pub const fn is_heap(self) -> bool {
        self.tag() == TAG_HEAP
    }

    // ── Tag extraction ───────────────────────────────────────────────

    /// Extract the integer value, or `None` if this isn't an
    /// integer.  Arithmetic shift-right by 3 sign-extends the
    /// stored 61-bit value back to a full `i64`.
    #[inline]
    pub const fn as_int(self) -> Option<i64> {
        if self.is_int() {
            Some((self.0 as i64) >> 3)
        } else {
            None
        }
    }

    /// Extract the boolean value, or `None` if this isn't a bool.
    #[inline]
    pub const fn as_bool(self) -> Option<bool> {
        if self.is_true() { Some(true) }
        else if self.is_false() { Some(false) }
        else { None }
    }

    /// Extract the [`SymbolId`], or `None` if this isn't a symbol.
    #[inline]
    pub const fn as_symbol(self) -> Option<SymbolId> {
        if self.is_symbol() {
            Some(SymbolId((self.0 >> 32) as u32))
        } else {
            None
        }
    }

    /// Extract the raw heap pointer (with tag bits cleared), or
    /// `None` if this isn't a heap value.
    ///
    /// The returned pointer is to the start of an
    /// [`lang_runtime_core::ObjectHeader`] — callers cast to the
    /// language-specific payload type after checking the header's
    /// `class_or_kind`.
    ///
    /// # Provenance
    ///
    /// Uses [`std::ptr::with_exposed_provenance`] to recover a
    /// usable pointer from the address — the corresponding
    /// `expose_provenance` happened in [`from_heap`].  Together
    /// they form the strict-provenance round-trip: tagging
    /// exposes the provenance, untagging recovers it from the
    /// thread's exposed set.  Without these explicit calls Miri
    /// (under `-Zmiri-strict-provenance`) rejects every heap
    /// dereference because integer-to-pointer casts lose
    /// provenance.
    ///
    /// `const fn` is no longer applicable here because
    /// `with_exposed_provenance` is not yet const; pre-strict-
    /// provenance code used `as *const T` (a const op) but that
    /// relied on miri's permissive provenance tracking.
    #[inline]
    pub fn as_heap_ptr<T>(self) -> Option<*const T> {
        if self.is_heap() {
            let addr = (self.0 & !TAG_BITS) as usize;
            Some(std::ptr::with_exposed_provenance::<T>(addr))
        } else {
            None
        }
    }

    /// Return the raw `u64` representation.  Used at ABI boundaries
    /// (the `extern "C"` surface) where values cross as plain words.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    // ── Truthiness (Scheme semantics) ────────────────────────────────

    /// Truthiness in Scheme / Twig: only `#f` and `nil` are false;
    /// everything else (including `0`) is true.
    ///
    /// This is the implementation behind
    /// `<crate::LispyBinding as LangBinding>::is_truthy`.
    #[inline]
    pub const fn is_truthy(self) -> bool {
        !(self.is_false() || self.is_nil())
    }
}

// ---------------------------------------------------------------------------
// Display: round-trip-friendly textual rendering for tests + debug
// ---------------------------------------------------------------------------

impl std::fmt::Display for LispyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(n) = self.as_int() {
            write!(f, "{n}")
        } else if self.is_nil() {
            write!(f, "nil")
        } else if self.is_true() {
            write!(f, "#t")
        } else if self.is_false() {
            write!(f, "#f")
        } else if let Some(sym) = self.as_symbol() {
            // Without access to the intern table we render by id;
            // the binding's Display goes through the symbol table
            // for human-readable names.
            write!(f, "'{sym}")
        } else if self.is_heap() {
            write!(f, "<heap@0x{:x}>", self.0 & !TAG_BITS)
        } else {
            write!(f, "<unknown-tag@0b{:b}>", self.tag())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lispy_value_is_8_bytes() {
        // ABI commitment — checked at compile time via `const _:`
        // above, but also exposed here so the failure message is
        // legible.
        assert_eq!(std::mem::size_of::<LispyValue>(), 8);
    }

    // ── Integer round-trip ───────────────────────────────────────────

    #[test]
    fn int_zero_round_trips() {
        let v = LispyValue::int(0);
        assert_eq!(v.as_int(), Some(0));
        assert!(v.is_int());
    }

    #[test]
    fn int_positive_round_trips() {
        for n in [1, 42, 1_000_000, INT_MAX] {
            assert_eq!(LispyValue::int(n).as_int(), Some(n), "{n}");
        }
    }

    #[test]
    fn int_negative_round_trips() {
        for n in [-1, -42, -1_000_000, INT_MIN] {
            assert_eq!(LispyValue::int(n).as_int(), Some(n), "{n}");
        }
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn int_above_range_panics_in_debug() {
        // Value just outside the representable range — silently
        // truncated in earlier drafts, now caught by the debug
        // assert in `int()`.
        let _ = LispyValue::int(INT_MAX + 1);
    }

    #[test]
    fn int_low_bits_are_clear() {
        let v = LispyValue::int(7);
        assert_eq!(v.tag(), TAG_INT);
        assert_eq!(v.0 & TAG_BITS, 0);
    }

    // ── Singletons ───────────────────────────────────────────────────

    #[test]
    fn nil_is_distinct_from_false() {
        // Distinct bit patterns is a Scheme correctness requirement:
        // `(null? #f)` must be false, `(eq? nil #f)` must be false.
        assert_ne!(LispyValue::NIL, LispyValue::FALSE);
        assert!(LispyValue::NIL.is_nil());
        assert!(LispyValue::FALSE.is_false());
    }

    #[test]
    fn true_and_false_are_distinct() {
        assert_ne!(LispyValue::TRUE, LispyValue::FALSE);
        assert!(LispyValue::TRUE.is_true());
        assert!(LispyValue::FALSE.is_false());
    }

    #[test]
    fn bool_constructor_round_trips() {
        assert_eq!(LispyValue::bool(true).as_bool(), Some(true));
        assert_eq!(LispyValue::bool(false).as_bool(), Some(false));
        assert_eq!(LispyValue::int(0).as_bool(), None);
    }

    // ── Symbols ──────────────────────────────────────────────────────

    #[test]
    fn symbol_round_trips_id() {
        let s = LispyValue::symbol(SymbolId(42));
        assert_eq!(s.as_symbol(), Some(SymbolId(42)));
        assert!(s.is_symbol());
        assert!(!s.is_int());
    }

    #[test]
    fn symbol_max_id_round_trips() {
        let s = LispyValue::symbol(SymbolId(u32::MAX - 1));
        assert_eq!(s.as_symbol(), Some(SymbolId(u32::MAX - 1)));
    }

    #[test]
    fn two_symbols_with_same_id_are_eq() {
        // Compile-time identity for IC keying.
        let a = LispyValue::symbol(SymbolId(7));
        let b = LispyValue::symbol(SymbolId(7));
        assert_eq!(a, b);
    }

    // ── Heap pointers ────────────────────────────────────────────────

    #[test]
    fn heap_round_trips_pointer() {
        // Build an 8-aligned dummy "heap object" for the round-trip.
        let aligned: Box<u64> = Box::new(0xCAFEu64);
        let raw_ptr = Box::into_raw(aligned);
        // SAFETY: raw_ptr is freshly Box::into_raw'd; 8-aligned and live.
        let v = unsafe { LispyValue::from_heap(raw_ptr) };
        assert!(v.is_heap());
        let recovered: *const u64 = v.as_heap_ptr().unwrap();
        assert_eq!(recovered as u64, raw_ptr as u64);
        // SAFETY: same pointer we just leaked, recovered exactly.
        unsafe {
            assert_eq!(*recovered, 0xCAFE);
            // Drop the leaked allocation so the test doesn't leak.
            let _ = Box::from_raw(raw_ptr);
        }
    }

    #[test]
    #[should_panic(expected = "heap pointer must be 8-byte aligned")]
    fn unaligned_heap_pointer_panics() {
        // Construct an unaligned pointer for the test.  Now a
        // hard `assert!`, fires in debug AND release.
        // SAFETY: testing the panic path; we don't dereference.
        let _ = unsafe { LispyValue::from_heap(0x123 as *const u8) };
    }

    // ── Truthiness ───────────────────────────────────────────────────

    #[test]
    fn truthiness_follows_scheme_semantics() {
        // Only nil and #f are false; 0 is true (NOT like C/Python).
        assert!(LispyValue::TRUE.is_truthy());
        assert!(LispyValue::int(0).is_truthy(), "0 is truthy in Scheme");
        assert!(LispyValue::int(-1).is_truthy());
        assert!(LispyValue::symbol(SymbolId(0)).is_truthy());
        assert!(!LispyValue::FALSE.is_truthy());
        assert!(!LispyValue::NIL.is_truthy());
    }

    // ── Bits / display ───────────────────────────────────────────────

    #[test]
    fn bits_returns_full_word() {
        assert_eq!(LispyValue::int(7).bits(), (7u64 << 3) | TAG_INT);
        assert_eq!(LispyValue::NIL.bits(), TAG_NIL);
    }

    #[test]
    fn display_renders_human_text() {
        assert_eq!(format!("{}", LispyValue::int(42)), "42");
        assert_eq!(format!("{}", LispyValue::int(-7)), "-7");
        assert_eq!(format!("{}", LispyValue::NIL), "nil");
        assert_eq!(format!("{}", LispyValue::TRUE), "#t");
        assert_eq!(format!("{}", LispyValue::FALSE), "#f");
        assert_eq!(format!("{}", LispyValue::symbol(SymbolId(3))), "'sym#3");
    }

    #[test]
    fn display_renders_heap_with_address() {
        // Box-allocated u64 is guaranteed 8-aligned (the trick this
        // tagging scheme depends on).
        let aligned: Box<u64> = Box::new(0xCAFEu64);
        let raw_ptr = Box::into_raw(aligned);
        // SAFETY: raw_ptr is 8-aligned and live.
        let v = unsafe { LispyValue::from_heap(raw_ptr) };
        let rendered = format!("{v}");
        assert!(rendered.starts_with("<heap@0x"), "got {rendered}");
        // Drop the leaked allocation so the test doesn't leak.
        unsafe { drop(Box::from_raw(raw_ptr)); }
    }
}
