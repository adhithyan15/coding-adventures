//! # Heap object header: the uniform 16-byte preamble.
//!
//! Per LANG20 §"Cross-language value representation", every heap
//! object — a Lispy cons cell, a JS object, a Ruby `RObject`, a
//! Smalltalk instance — carries the **same 16-byte header** before
//! its language-specific payload.  This is a non-negotiable ABI
//! commitment that makes one GC serve every language without
//! per-language plumbing.
//!
//! ## Layout (all little-endian on supported targets)
//!
//! ```text
//! ┌──────────────────────┬──────────────┬────────────────┬──────────────┐
//! │  class_or_kind: u32  │  gc_word: u32│  size_bytes: u32│  flags: u32  │
//! └──────────────────────┴──────────────┴────────────────┴──────────────┘
//!     0                4                8                12          16
//!                                                                       │
//!                                                       language-specific payload follows
//! ```
//!
//! ### Field semantics
//!
//! | Field | Owner | Use |
//! |-------|-------|-----|
//! | `class_or_kind` | per-language | Opaque ID the binding uses to dispatch trace, finalize, etc. Registered with the GC at runtime startup. |
//! | `gc_word` | collector | Mark bit, age, forwarding pointer, lock word — collector's choice. |
//! | `size_bytes` | allocator | Object size in bytes including this header. Needed for sweep, copy, and finalization. |
//! | `flags` | shared | Lower 8 bits reserved by `lang-runtime-core` (see [`HeaderFlags`]); upper 24 bits free for language use. |
//!
//! ## Why uniform?
//!
//! Three reasons:
//!
//! 1. **One GC, many languages.**  The collector dispatches trace
//!    via `binding_for(header.class_or_kind).trace_object(header,
//!    visitor)`.  The collector itself never needs language-specific
//!    knowledge.
//! 2. **Cross-language references work.**  A Twig program can hold a
//!    Ruby regex object (or vice versa) because both languages agree
//!    on the header layout.  Tracing crosses the boundary cleanly.
//! 3. **Tooling (debuggers, profilers, heap dumps) can walk the
//!    heap** without per-language adapters.

use std::sync::atomic::AtomicU32;

// ---------------------------------------------------------------------------
// ObjectHeader
// ---------------------------------------------------------------------------

/// The 16-byte header that prefixes every heap-allocated object,
/// across every language.
///
/// `#[repr(C)]` is essential: this is an ABI commitment to all
/// runtime tiers (interpreter, JIT, AOT, debugger, GC).  Reordering
/// or padding would break every binding silently.
///
/// # Atomicity
///
/// `gc_word` is the collector's bookkeeping field and may be written
/// by parallel/concurrent collectors.  We expose it as `AtomicU32`
/// so any access uses explicit ordering and the compiler doesn't
/// optimise away cross-thread visibility.  Single-threaded
/// collectors can still write through `Relaxed`.
///
/// # Safety
///
/// `ObjectHeader` instances live at the start of allocations;
/// constructing an `ObjectHeader` in isolation is safe but rarely
/// useful.  The runtime obtains a `&ObjectHeader` from a heap
/// pointer via `&*ptr.cast::<ObjectHeader>()`.
#[repr(C)]
pub struct ObjectHeader {
    /// Per-language class identity.  Registered with the GC at
    /// startup; used to dispatch trace/finalize via
    /// `LangBinding::trace_object` / `LangBinding::finalize`.
    pub class_or_kind: u32,

    /// Collector's bookkeeping (mark bit, age, forwarding ptr, …).
    /// `AtomicU32` so concurrent collectors can update it safely;
    /// single-threaded collectors use `Ordering::Relaxed`.
    pub gc_word: AtomicU32,

    /// Total object size in bytes, including this 16-byte header.
    /// Allocator writes; collector reads during sweep/copy.
    pub size_bytes: u32,

    /// Bitmask of header flags.  Lower 8 bits reserved by
    /// `lang-runtime-core`; upper 24 bits free for language use.
    pub flags: u32,
}

// Compile-time size assertion: 16 bytes is part of the ABI
// contract.  AtomicU32 is repr-equivalent to u32 so the header
// stays 4+4+4+4 = 16.
const _: () = assert!(std::mem::size_of::<ObjectHeader>() == 16);
const _: () = assert!(std::mem::align_of::<ObjectHeader>() == 4);

impl ObjectHeader {
    /// Initial `gc_word` value — all bookkeeping bits clear.
    pub const FRESH_GC_WORD: u32 = 0;

    /// Construct a fresh header for a newly-allocated object.
    ///
    /// `class_or_kind`: the binding's class id for this object.
    /// `size_bytes`: total size including the header.
    pub fn new(class_or_kind: u32, size_bytes: u32) -> Self {
        ObjectHeader {
            class_or_kind,
            gc_word: AtomicU32::new(Self::FRESH_GC_WORD),
            size_bytes,
            flags: 0,
        }
    }

    /// Set / clear / test header flags.  Languages may use the
    /// upper 24 bits freely; lower 8 bits are reserved.
    #[inline]
    pub fn has_flag(&self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }
}

// ---------------------------------------------------------------------------
// HeaderFlags — reserved low-8-bit flag namespace
// ---------------------------------------------------------------------------

/// Reserved flags in the lower 8 bits of [`ObjectHeader::flags`].
///
/// Only flag bits 0..7 are reserved; bits 8..31 are free for
/// per-language use (e.g. JS's "frozen", Ruby's "tainted",
/// Smalltalk's "indexed").
pub mod header_flags {
    /// Bit 0: object has a finalizer registered.  The collector
    /// calls `LangBinding::finalize(header)` before reclaiming the
    /// object.
    pub const HAS_FINALIZER: u32 = 1 << 0;

    /// Bit 1: object is immortal — the collector skips it during
    /// sweep.  Useful for canonical singletons (intern-table heads,
    /// type metadata, the global `nil`).
    pub const IS_IMMORTAL: u32 = 1 << 1;

    /// Bit 2: object is in the old generation (relevant only to
    /// generational collectors; ignored by mark-sweep).
    pub const IS_OLD_GEN: u32 = 1 << 2;

    /// Bit 3: object's payload contains pointers that the binding
    /// wants the collector to skip-trace (rare; e.g. weak refs
    /// stored opaquely).  The binding's `trace_object` is still
    /// called but the binding is expected to do nothing.
    pub const SKIP_TRACE: u32 = 1 << 3;

    // Bits 4..7 reserved for future use.
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn header_is_16_bytes() {
        // This is the LANG20 ABI commitment.  If this test fails the
        // multi-language heap interop is broken.
        assert_eq!(std::mem::size_of::<ObjectHeader>(), 16);
    }

    #[test]
    fn header_layout_matches_spec() {
        // Field offsets: class_or_kind=0, gc_word=4, size_bytes=8, flags=12.
        // We can't take offset_of! on stable, so check via byte writes.
        let h = ObjectHeader::new(0xDEAD_BEEF, 32);
        let ptr = &h as *const ObjectHeader as *const u32;
        unsafe {
            assert_eq!(*ptr.add(0), 0xDEAD_BEEF);
            // gc_word is AtomicU32 but loads identically.
            assert_eq!(*ptr.add(1), ObjectHeader::FRESH_GC_WORD);
            assert_eq!(*ptr.add(2), 32);
            assert_eq!(*ptr.add(3), 0);
        }
    }

    #[test]
    fn fresh_gc_word_is_zero() {
        let h = ObjectHeader::new(1, 16);
        assert_eq!(h.gc_word.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn has_flag_returns_correctly() {
        let mut h = ObjectHeader::new(1, 16);
        assert!(!h.has_flag(header_flags::HAS_FINALIZER));
        h.flags |= header_flags::HAS_FINALIZER;
        assert!(h.has_flag(header_flags::HAS_FINALIZER));
        assert!(!h.has_flag(header_flags::IS_IMMORTAL));
    }

    #[test]
    fn reserved_flags_are_distinct_powers_of_two() {
        for f in [
            header_flags::HAS_FINALIZER,
            header_flags::IS_IMMORTAL,
            header_flags::IS_OLD_GEN,
            header_flags::SKIP_TRACE,
        ] {
            assert!(f.is_power_of_two(), "flag {f:#b} should be a power of two");
            assert!(f < (1 << 8), "reserved flags must fit in lower 8 bits");
        }
    }

    #[test]
    fn upper_24_bits_are_user_writable() {
        let mut h = ObjectHeader::new(1, 16);
        let user_flag = 1u32 << 16;
        h.flags |= user_flag;
        assert!(h.has_flag(user_flag));
    }
}
