//! # Value-related types: `SymbolId`, `BoxedReprToken`.
//!
//! These two types are tiny but show up everywhere in the runtime
//! contract.
//!
//! ## `SymbolId`
//!
//! A 32-bit handle into the runtime's intern table.  Symbols are
//! interned strings — every distinct string value (a Ruby method
//! name, a JavaScript property key, a Lisp atom) is mapped to a
//! unique [`SymbolId`] so the runtime can compare names with a
//! single 32-bit equality check instead of byte-by-byte string
//! comparison.
//!
//! Examples of where `SymbolId` shows up:
//!
//! - The `selector` argument to `LangBinding::send_message`
//! - The `key` argument to `LangBinding::load_property` and
//!   `LangBinding::store_property`
//! - The constant pool of an `IIRFunction` (LANG01 + LANG20 §"IIR
//!   additions")
//!
//! ## `BoxedReprToken`
//!
//! A discriminator that tells the runtime *how* a value is encoded
//! at a given native location during deopt.  When a JIT- or
//! AOT-compiled function holds a value in a hardware register, the
//! value may be:
//!
//! - A boxed reference (the register holds the raw `Value` word)
//! - An unboxed `i64` (the register holds the raw integer; need to
//!   re-box before handing back to the interpreter)
//! - An unboxed `f64`, `bool`, or a derived pointer
//!
//! On deopt the runtime walks the frame descriptor (LANG20 §"Deopt
//! protocol") and uses each register's `BoxedReprToken` to ask the
//! [`crate::LangBinding`] to materialise the right `Value`.
//!
//! Both types are `Copy + Eq + Hash` because they appear in cache
//! keys, hash maps, and across ABI boundaries.

// ---------------------------------------------------------------------------
// SymbolId
// ---------------------------------------------------------------------------

/// A 32-bit handle into the runtime's symbol intern table.
///
/// The intern table is owned by `lang-runtime-core` (per LANG20
/// §"Cross-language value representation") so all languages
/// share one symbol namespace inside a process.  Two interned
/// strings with identical bytes get the same `SymbolId` and
/// compare equal in O(1).
///
/// # Reserved values
///
/// | Value | Meaning |
/// |-------|---------|
/// | [`SymbolId::NONE`] (`u32::MAX`) | Sentinel: "no symbol".  Never returned by the intern table; bindings use it where they need to express "absent selector / property key". |
/// | [`SymbolId::EMPTY`] (`SymbolId(0)`) | The empty string `""`.  Always interned at runtime startup so its id is stable across runs. |
///
/// The two values are deliberately distinct so a binding can
/// distinguish "the user really did intern the empty string"
/// (legitimate in JS property keys, Ruby method names, …) from
/// "no selector was supplied".  Conflating them would cause
/// `obj[""]` lookups to silently behave like missing-property
/// errors.
///
/// # Why a newtype, not `u32`?
///
/// A bare `u32` would let callers accidentally mix `SymbolId`s with
/// `ClassId`s or `ICId`s.  The newtype makes the intent explicit
/// and gives us a single place to add validation later (e.g. an
/// upper bound when intern-table-id width changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SymbolId(pub u32);

impl SymbolId {
    /// The reserved id for the empty string `""`.
    ///
    /// The intern table allocates this id eagerly at startup so
    /// language frontends that need to reference the empty
    /// string get a stable id without an explicit intern call.
    pub const EMPTY: SymbolId = SymbolId(0);

    /// Sentinel for "no symbol" — never returned by the intern
    /// table.  Use this in error variants and "absent selector"
    /// contexts where you must fit a `SymbolId` slot but don't
    /// have a real one.
    ///
    /// Distinct from [`EMPTY`](Self::EMPTY) so the empty-string
    /// key isn't mistaken for missing-symbol — see the type-level
    /// docs for why.
    pub const NONE: SymbolId = SymbolId(u32::MAX);

    /// Return the underlying `u32` for ABI passing.
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// True if this is the [`NONE`](Self::NONE) sentinel.
    pub const fn is_none(self) -> bool {
        self.0 == u32::MAX
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sym#{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// BoxedReprToken
// ---------------------------------------------------------------------------

/// Discriminator for how a `LangBinding::Value` is encoded at a
/// specific native location during JIT/AOT execution.
///
/// Used inside [`crate::deopt::FrameDescriptor`] entries: the JIT
/// emits one token per live register at every deopt anchor; on a
/// guard failure the runtime walks the descriptor and asks the
/// active [`crate::LangBinding`] to materialise each register
/// back into a real `Value`.
///
/// ## Variant table
///
/// | Variant | Layout in native register | Materialisation |
/// |---------|--------------------------|-----------------|
/// | `BoxedRef` | the raw `Value` word | take as-is |
/// | `I64Unboxed` | a raw signed i64 | wrap as language integer |
/// | `F64Unboxed` | a raw `f64::to_bits()` | wrap as language float |
/// | `BoolUnboxed` | low bit set / clear | wrap as language bool |
/// | `DerivedPtr { base_register, offset }` | NOT in this register; reconstruct from `base_register + offset` | re-box |
///
/// ## Why include `DerivedPtr`?
///
/// Optimising codegen often hoists "interior pointer" arithmetic
/// out of loops — e.g. instead of computing `cell.cdr` inside
/// every iteration, the JIT may keep `cdr_ptr = base + 8` in a
/// register.  The deopt path can't materialise that directly as a
/// `Value` (it's a derived pointer, not a real heap reference);
/// it must be reconstructed from the base + offset.
///
/// `DerivedPtr` records exactly that: where the base lives and how
/// far the derived pointer is from it.  Materialisation reads the
/// base register, adds the offset, and hands the resulting
/// pointer to the binding's `materialize_value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoxedReprToken {
    // (Variants below.  See module docs for the table.)
    //
    // Compile-time-asserted to fit in ≤ 8 bytes via `const _:` at
    // module scope so a future variant addition can't silently bloat
    // FrameDescriptor entries.
    /// Value is a fully boxed `LangBinding::Value` — the native
    /// location holds the raw `Value` word; just store the bits.
    BoxedRef,

    /// Value is an unboxed signed i64 — wrap as the language's
    /// integer type via `materialize_value`.
    I64Unboxed,

    /// Value is an unboxed double — wrap as the language's float
    /// type via `materialize_value`.
    F64Unboxed,

    /// Value is an unboxed bool — wrap as the language's bool
    /// type via `materialize_value`.
    BoolUnboxed,

    /// Value is a derived pointer (e.g. interior cons-cdr ptr);
    /// rebuild from `base_register + offset` before materialising.
    DerivedPtr {
        /// Hardware-register index holding the base pointer.
        base_register: u8,
        /// Byte offset added to `base_register` to get the derived ptr.
        offset: i32,
    },
}

// Compile-time ABI commitments — caught at compile time, not in
// downstream test runs.
const _: () = assert!(std::mem::size_of::<SymbolId>() == 4);
const _: () = assert!(std::mem::size_of::<BoxedReprToken>() <= 8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_id_empty_is_zero() {
        assert_eq!(SymbolId::EMPTY.as_u32(), 0);
    }

    #[test]
    fn symbol_id_none_is_distinct_from_empty() {
        // The two reserved values must not collide; conflating them
        // would cause obj[""] lookups to behave like missing-property
        // errors in any binding that uses NONE as its absent-symbol
        // sentinel.
        assert_ne!(SymbolId::NONE, SymbolId::EMPTY);
        assert_eq!(SymbolId::NONE.as_u32(), u32::MAX);
        assert!(SymbolId::NONE.is_none());
        assert!(!SymbolId::EMPTY.is_none());
    }

    #[test]
    fn symbol_id_equality() {
        assert_eq!(SymbolId(7), SymbolId(7));
        assert_ne!(SymbolId(7), SymbolId(8));
    }

    #[test]
    fn symbol_id_display_includes_value() {
        let s = format!("{}", SymbolId(42));
        assert_eq!(s, "sym#42");
    }

    #[test]
    fn symbol_id_is_repr_transparent_size_4() {
        // Catch accidental enlargement; ABI assumes 4 bytes.
        assert_eq!(std::mem::size_of::<SymbolId>(), 4);
    }

    #[test]
    fn boxed_repr_token_size_is_small() {
        // The token appears in dense FrameDescriptor tables so its
        // size matters for cache footprint.  Allow up to 8 bytes
        // (DerivedPtr fits in 1 + 1 + 4 = 6 with discriminant).
        assert!(std::mem::size_of::<BoxedReprToken>() <= 8);
    }

    #[test]
    fn boxed_repr_token_variants_are_copy() {
        let a = BoxedReprToken::BoxedRef;
        let _b = a;          // copies
        let _c = a;          // still usable
        let _ = a;
    }

    #[test]
    fn boxed_repr_token_derived_ptr_carries_payload() {
        let t = BoxedReprToken::DerivedPtr { base_register: 7, offset: 16 };
        match t {
            BoxedReprToken::DerivedPtr { base_register, offset } => {
                assert_eq!(base_register, 7);
                assert_eq!(offset, 16);
            }
            _ => panic!("expected DerivedPtr"),
        }
    }
}
