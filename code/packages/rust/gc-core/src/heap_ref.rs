//! # HeapRef — an opaque, typed reference to a garbage-collected object.
//!
//! A `HeapRef` is the LANG pipeline's representation of a pointer into the
//! managed heap.  It wraps a bare `usize` address (matching the `uintptr_t`
//! used in the C ABI of `vm-runtime`) so that safe Rust code never holds a
//! raw pointer to a moving or dead object.
//!
//! ## Null safety
//!
//! `HeapRef::NULL` is address `0`.  The underlying `MarkAndSweepGC`
//! starts allocation at `0x10000`, so `0` is never a valid live address.
//! All unboxing opcodes (`unbox`, `field_load`) check for null before
//! dereferencing and raise a `VM_TRAP` if null is observed.
//!
//! ## Why a newtype?
//!
//! `usize` is also used as an array index, a byte count, and a loop counter.
//! Wrapping it in `HeapRef` makes function signatures self-documenting and
//! prevents accidentally passing a raw integer where a heap address is
//! expected — a class of bug that would otherwise be invisible until
//! production.
//!
//! ```
//! use gc_core::HeapRef;
//!
//! let r = HeapRef::new(0x10042);
//! assert!(!r.is_null());
//! assert_eq!(r.addr(), 0x10042);
//!
//! let null = HeapRef::NULL;
//! assert!(null.is_null());
//! ```

/// An opaque heap address produced by `GcCore::alloc`.
///
/// Values of this type flow through registers typed `ref<T>` in the IIR
/// (`type_hint = "ref<u8>"`, etc.).  The number inside is meaningful only
/// to the `GcAdapter` that issued it; no arithmetic on refs is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HeapRef(usize);

impl HeapRef {
    /// The null reference — address 0, which is never a live heap object.
    pub const NULL: HeapRef = HeapRef(0);

    /// Wrap a raw address as a `HeapRef`.
    ///
    /// Only call this from `GcAdapter::alloc` (where the address was
    /// freshly returned by the underlying `GarbageCollector`).
    #[inline]
    pub fn new(addr: usize) -> Self {
        HeapRef(addr)
    }

    /// The underlying integer address.
    ///
    /// Pass this to `GarbageCollector::deref` or `is_valid_address`.
    #[inline]
    pub fn addr(self) -> usize {
        self.0
    }

    /// `true` if this is the null reference.
    #[inline]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for HeapRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else {
            write!(f, "ref({:#x})", self.0)
        }
    }
}
