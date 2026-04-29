//! # Visitor traits used by the GC and root scanner.
//!
//! Two visitor traits, both deliberately *non-generic* over the
//! [`crate::LangBinding`]:
//!
//! - [`ValueVisitor`] — passed to `LangBinding::trace_object` /
//!   `LangBinding::trace_value` so the binding can report each
//!   reference field it finds.
//! - [`RootVisitor`] — passed to the GC's root-scanning entry
//!   point so it can enumerate every live root location (register
//!   slot, native frame slot, intern-table head).
//!
//! ## Why non-generic?
//!
//! The collector is one piece of code that serves every language.
//! If the visitor were generic over `LangBinding`, the collector
//! would need to be parameterised over every binding it traces,
//! which doesn't compose for cross-language heap interop.
//!
//! Instead the visitor accepts opaque words (`u64`).  A binding
//! converts its typed `Value` to `u64` (via `box_value` or just a
//! transmute if `Value` is a 64-bit POD) before calling
//! `visit_value`.  The collector enqueues the word; later it
//! dispatches on the heap object's `class_or_kind` (LANG20
//! §"Cross-language value representation") to reach the right
//! binding's `trace_object` for further walking.
//!
//! ## What about type safety?
//!
//! Type safety lives at the binding boundary: each binding owns
//! its `Value` encoding and only its own `trace_*` methods write
//! into the visitor.  The collector never invents `u64`s — it
//! only forwards what bindings give it.

// ---------------------------------------------------------------------------
// ValueVisitor
// ---------------------------------------------------------------------------

/// Visitor passed to `LangBinding::trace_object` and
/// `LangBinding::trace_value`.
///
/// The binding calls [`visit_value`](Self::visit_value) once for
/// each reference field it finds in the object payload.  The
/// collector implementation behind the visitor enqueues, marks,
/// or copies the reference as appropriate for its algorithm.
///
/// Implementations are expected to be cheap (a single push onto a
/// worklist for mark-sweep; a forward-and-fixup for copying GC).
pub trait ValueVisitor {
    /// Called by the binding for each reference encountered while
    /// tracing.
    ///
    /// `raw` is the `Value` word as a `u64`.  The collector
    /// inspects the object header it points at (if it's a pointer)
    /// or treats it as an immediate (if the binding's tagging
    /// scheme says so).  The collector does NOT decode language-
    /// specific tags — that's the binding's job before calling.
    fn visit_value(&mut self, raw: u64);
}

// ---------------------------------------------------------------------------
// RootVisitor
// ---------------------------------------------------------------------------

/// Visitor passed to the runtime's root-scanning entry point.
///
/// At collection time the runtime walks every live root location —
/// registers in interpreter `VMFrame`s, slots in JIT/AOT native
/// frames described by stack maps, intern-table heads, GC pinning
/// roots — and calls [`visit_root`](Self::visit_root) for each.
///
/// The visitor receives a *mutable pointer* to the root location,
/// not the value.  This lets a copying collector forward the
/// reference in place: read the value, copy the object,
/// overwrite the slot with the new pointer.  Mark-sweep
/// collectors only read through the pointer.
///
/// # Safety
///
/// Implementors must not retain `location` past the call — the
/// runtime guarantees the location is live during the scan but
/// not after.  Reading and writing a single `u64` through
/// `location` is sound; aliasing is enforced by the runtime
/// (mutators are paused during root scanning).
pub trait RootVisitor {
    /// Called by the runtime for each root location.
    ///
    /// `location` points at the live `Value` slot.  Mark-sweep
    /// collectors read; copying collectors read+write to forward.
    ///
    /// # Safety
    ///
    /// `location` must point to an aligned 8-byte word that holds
    /// a `Value`.  The runtime upholds this; visitor implementations
    /// should not validate.
    unsafe fn visit_root(&mut self, location: *mut u64);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial recording visitor used by both binding implementations
    /// and collector tests to assert on what was visited.
    struct RecordingVisitor {
        seen: Vec<u64>,
    }

    impl ValueVisitor for RecordingVisitor {
        fn visit_value(&mut self, raw: u64) {
            self.seen.push(raw);
        }
    }

    #[test]
    fn value_visitor_records_each_call() {
        let mut v = RecordingVisitor { seen: Vec::new() };
        v.visit_value(0xDEAD_BEEF);
        v.visit_value(42);
        v.visit_value(u64::MAX);
        assert_eq!(v.seen, vec![0xDEAD_BEEF, 42, u64::MAX]);
    }

    #[test]
    fn value_visitor_works_through_dyn_trait_object() {
        // The trait must be object-safe so LangBinding::trace_object
        // can take `&mut dyn ValueVisitor`.
        let mut v = RecordingVisitor { seen: Vec::new() };
        let dyn_v: &mut dyn ValueVisitor = &mut v;
        dyn_v.visit_value(7);
        assert_eq!(v.seen, vec![7]);
    }

    /// Recording root visitor for tests.
    struct RecordingRootVisitor {
        seen: Vec<u64>,
    }

    impl RootVisitor for RecordingRootVisitor {
        unsafe fn visit_root(&mut self, location: *mut u64) {
            self.seen.push(*location);
        }
    }

    #[test]
    fn root_visitor_reads_through_pointer() {
        let mut v = RecordingRootVisitor { seen: Vec::new() };
        let mut slot1 = 42u64;
        let mut slot2 = 0xCAFEu64;
        unsafe {
            v.visit_root(&mut slot1 as *mut u64);
            v.visit_root(&mut slot2 as *mut u64);
        }
        assert_eq!(v.seen, vec![42, 0xCAFE]);
    }

    #[test]
    fn root_visitor_can_forward_via_write() {
        // Sketch a minimal copying-style use: read, then overwrite.
        struct ForwardingVisitor;
        impl RootVisitor for ForwardingVisitor {
            unsafe fn visit_root(&mut self, location: *mut u64) {
                let old = *location;
                *location = old + 1;     // pretend "forward"
            }
        }
        let mut slot = 99u64;
        let mut v = ForwardingVisitor;
        unsafe { v.visit_root(&mut slot as *mut u64) };
        assert_eq!(slot, 100);
    }

    #[test]
    fn root_visitor_works_through_dyn_trait_object() {
        let mut v = RecordingRootVisitor { seen: Vec::new() };
        let dyn_v: &mut dyn RootVisitor = &mut v;
        let mut slot = 5u64;
        unsafe { dyn_v.visit_root(&mut slot as *mut u64) };
        assert_eq!(v.seen, vec![5]);
    }
}
