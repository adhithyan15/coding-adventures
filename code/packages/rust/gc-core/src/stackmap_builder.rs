//! # Building a function's stack maps — the producer side of the precise-root format
//!
//! [`flat_heap`](crate::flat_heap) defines the stack-map *format*
//! ([`StackMapRecord`] / [`StackMapTable`]) and the *consumer* helper
//! ([`frame_root_slots`](crate::frame_root_slots)) that turns a record into root
//! addresses. This module is the missing *producer*: the helper a native code
//! generator drives while it lowers a function, so it can hand the runtime a table
//! of "which stack slots hold live references at each safepoint".
//!
//! Design and rationale: `AOT00-T1-stackmap-emission.md`. Until a backend feeds the
//! runtime records, `resolve(return_address)` finds nothing and every frame falls
//! back to a conservative scan — correct, but no more precise than the old
//! collector. This builder is the first step of closing that gap.
//!
//! ## Rule R1 — every reference slot of the function, at every safepoint
//!
//! The precise answer to "which references are live *here*?" needs a dataflow
//! analysis the hand-written native backends do not have. So we use a sound,
//! **flow-insensitive** over-approximation:
//!
//! > **R1.** Every safepoint in a function names **every** stack slot that function
//! > ever uses to hold a GC reference.
//!
//! R1 is safe by construction and, crucially, **independent of the order the backend
//! drives this builder** — see the warning below for why that matters. It still
//! delivers the main prize over a conservative frame scan: it excludes **every
//! non-reference slot** (integers, floats, booleans), so a stack integer that happens
//! to look like a heap address no longer pins a dead object. What it gives up versus
//! an exact analysis is only the ability to drop a reference slot that is dead *at a
//! particular PC* — which merely retains floating garbage for a cycle, exactly as a
//! conservative scan would.
//!
//! ### Why not "only the slots defined before this safepoint"?
//!
//! That flow-*sensitive* rule (an earlier draft of this design) is **unsound**,
//! because it silently equates the order the backend emits code with the order the
//! machine executes it. A backward edge breaks the equivalence:
//!
//! ```text
//!   loop_top:  call use(x)      ← safepoint emitted here, before x is defined
//!              x = alloc()      ← the slot is only defined *after* it, in emission order
//!              b loop_top       ← on iteration 2+, x IS live at the safepoint
//! ```
//!
//! On the second iteration that slot holds a live reference the record would not
//! name. And an *incomplete* record is far more dangerous than a missing one: the
//! walker treats a hit as authoritative and **skips the conservative scan of that
//! frame** ([`crate::flat_heap::collect_mixed`] is only handed the named slots), so
//! the omission frees a live object instead of merely retaining garbage. The same
//! trap springs for any backend that lays blocks out of execution order (outlined
//! cold paths, tail duplication, landing pads emitted last).
//!
//! Because the builder cannot *detect* such a violation, it does not rely on the
//! backend avoiding it: [`StackMapBuilder::safepoint`] records only the PC, and the
//! slot set is resolved once at [`StackMapBuilder::into_records`] time from the
//! union of everything ever declared. Order simply cannot matter.
//!
//! A real backward-liveness pass may later shrink R1 per-PC — but only a pass that
//! reasons about *execution* paths, not emission order. That is a pure precision
//! gain and never a safety change.
//!
//! ## Safety contract the backend MUST uphold
//!
//! Getting a record *wrong* is worse than emitting none, so these are obligations,
//! not suggestions:
//!
//! 1. **Spill before the safepoint.** Every GC reference live across a safepoint must
//!    live in a stack slot of the current frame at that point, and that slot must be
//!    declared with [`StackMapBuilder::define_ref_slot`]. This builder describes
//!    *stack slots only* — `callee_saved_mask` is always `0` — so a reference kept
//!    solely in a callee-saved register across a call is named by **nobody** (neither
//!    the caller's record nor the callee's), and will be freed while live. The
//!    slot-per-variable native backends satisfy this naturally: they spill every
//!    value to its slot at instruction boundaries.
//! 2. **Declare incoming reference parameters.** They arrive in argument registers;
//!    the prologue spills them, and those slots must be declared too.
//! 3. **Declare only reference-typed slots.** Declaring a non-reference slot
//!    re-introduces exactly the false-root imprecision this rung removes. (It is not
//!    *unsafe* — just a wasted root.)
//!
//! Naming a slot the executed path has not written yet is safe: the collector routes
//! every slot word through the same validated candidate-pointer lookup as a
//! conservative scan, so uninitialised stack garbage either resolves to a real block
//! (harmless false retention) or to nothing at all.
//!
//! ## How a backend drives it
//!
//! One builder per function, in lowering order:
//!
//! ```
//! use gc_core::StackMapBuilder;
//!
//! // Frame is 48 bytes; slot offsets are FP-relative (see below).
//! let mut b = StackMapBuilder::new(48);
//!
//! b.define_ref_slot(-8);   // a `ref<…>` local lives at [fp-8]
//! b.safepoint(0x10);       // a call at pc 0x10 → its return address is a safepoint
//! b.define_ref_slot(-16);  // a second reference slot, declared later …
//! b.safepoint(0x24);
//!
//! // … yet BOTH records name BOTH slots: under R1 the set is the function's, not
//! // the prefix's, so no drive order can produce an incomplete record.
//! let records = b.into_records();
//! assert_eq!(records[0].slots, vec![-16, -8]); // sorted, ascending
//! assert_eq!(records[1].slots, vec![-16, -8]);
//! ```
//!
//! **Offsets are frame-pointer-relative**, matching [`StackMapRecord::slots`]. Both
//! native backends make this a no-op: aarch64 pins `x29 == sp` so its SP-relative
//! slots *are* FP-relative, and x86-64 already addresses slots from `rbp`.
//!
//! **Reference-ness is the backend's call.** This crate has no opinion on the IR's
//! type system, so the backend decides which values are `ref<…>` (from its IR type
//! information) and reports only those via [`StackMapBuilder::define_ref_slot`].
//! That keeps `gc-core` free of any compiler-frontend dependency.

use crate::flat_heap::{StackMapRecord, StackMapTable};

/// Accumulates one compiled function's [`StackMapRecord`]s under [Rule R1](self).
///
/// Created per function and driven while lowering — `define_ref_slot` for each stack
/// slot that holds a GC reference, `safepoint` at each call site / safepoint PC —
/// then consumed with [`Self::into_records`] or [`Self::into_table`].
///
/// **The two calls are independent.** Slots and safepoints are collected separately
/// and joined only at the end, so a `define_ref_slot` after a `safepoint` still
/// applies to it. That is deliberate: see [the module docs](self) for the loop that
/// makes any order-sensitive rule unsound.
#[derive(Debug, Clone, Default)]
pub struct StackMapBuilder {
    /// The frame size stamped onto every record (informational for the walker).
    frame_size: u32,
    /// Every reference slot this function uses, sorted and deduplicated. Under R1
    /// this whole set is the slot list of every record.
    ref_slots: Vec<i32>,
    /// Safepoint PCs, sorted and deduplicated.
    safepoints: Vec<u32>,
}

impl StackMapBuilder {
    /// A builder for a function whose frame is `frame_size` bytes.
    ///
    /// `frame_size` is copied into every record. It is informational today — the
    /// walker brackets a frame with the frame-pointer chain rather than the size —
    /// but it is part of the fixed record format, so backends fill it in now.
    pub fn new(frame_size: u32) -> Self {
        Self {
            frame_size,
            ref_slots: Vec::new(),
            safepoints: Vec::new(),
        }
    }

    /// Declare that the stack slot at FP-relative byte `offset` (negative for slots
    /// below the frame pointer) holds a **GC reference** somewhere in this function.
    ///
    /// Under [R1](self) the slot is named by **every** record, regardless of whether
    /// this call comes before or after the corresponding [`Self::safepoint`] — the
    /// builder is order-independent by design.
    ///
    /// Repeat calls for the same offset are idempotent (the native backends give each
    /// variable one permanent slot, so a re-assigned variable re-reports its offset).
    ///
    /// Only reference-typed slots may be declared: a non-reference slot re-introduces
    /// exactly the false-root imprecision this rung removes. Conversely, **failing**
    /// to declare a slot that holds a live reference is a use-after-free — see the
    /// safety contract in [the module docs](self).
    pub fn define_ref_slot(&mut self, offset: i32) {
        if let Err(pos) = self.ref_slots.binary_search(&offset) {
            self.ref_slots.insert(pos, offset);
        }
    }

    /// Record a safepoint at `pc_offset` (bytes from the function's first byte).
    ///
    /// For a call site this is the **return address** — the PC the walker actually
    /// observes at `[fp + 8]` in the caller's frame — i.e. the offset *just after*
    /// the call instruction, not of the call itself.
    ///
    /// Duplicate PCs collapse to one record. That matters: `StackMapTable::lookup`
    /// binary-searches, which with equal keys may return *either* record, so two
    /// records at one PC would make the resolved slot set a coin flip.
    ///
    /// A function whose references are all dead here still gets a record (with an
    /// empty slot list), and that is deliberate: an *absent* record makes `resolve`
    /// return `None` and the walker fall back to conservatively scanning the whole
    /// frame, whereas an empty record is the precise claim "nothing here is a
    /// reference".
    pub fn safepoint(&mut self, pc_offset: u32) {
        if let Err(pos) = self.safepoints.binary_search(&pc_offset) {
            self.safepoints.insert(pos, pc_offset);
        }
    }

    /// Whether no safepoint has been recorded (so the function needs no table).
    pub fn is_empty(&self) -> bool {
        self.safepoints.is_empty()
    }

    /// The reference slots declared so far (ascending). Mostly for tests and backend
    /// assertions.
    pub fn ref_slots(&self) -> &[i32] {
        &self.ref_slots
    }

    /// The safepoint PCs recorded so far (ascending).
    pub fn safepoint_pcs(&self) -> &[u32] {
        &self.safepoints
    }

    /// Consume the builder, yielding one record per safepoint (ascending by PC),
    /// each naming the function's full reference-slot set per [R1](self).
    pub fn into_records(self) -> Vec<StackMapRecord> {
        self.safepoints
            .iter()
            .map(|&pc_offset| StackMapRecord {
                pc_offset,
                frame_size: self.frame_size,
                slots: self.ref_slots.clone(),
                callee_saved_mask: 0,
            })
            .collect()
    }

    /// Consume the builder into a lookup-ready [`StackMapTable`].
    pub fn into_table(self) -> StackMapTable {
        StackMapTable::from_records(self.into_records())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The headline: every safepoint names the function's whole reference-slot set
    /// (Rule R1), so the record is the same no matter where the declarations fall.
    #[test]
    fn every_safepoint_names_the_full_ref_slot_set() {
        let mut b = StackMapBuilder::new(48);
        b.safepoint(0x00);
        b.define_ref_slot(-8);
        b.safepoint(0x10);
        b.define_ref_slot(-16);
        b.safepoint(0x24);

        let r = b.into_records();
        assert_eq!(r.len(), 3);
        for rec in &r {
            assert_eq!(rec.slots, vec![-16, -8], "R1: the function's full set");
            assert_eq!(rec.frame_size, 48);
        }
        assert_eq!((r[0].pc_offset, r[1].pc_offset, r[2].pc_offset), (0, 0x10, 0x24));
    }

    /// **Regression for a real use-after-free.** An earlier flow-sensitive rule
    /// ("only slots defined before this safepoint") was unsound across a backward
    /// edge: in `loop { use(x); x = alloc(); }` the slot is declared *after* the
    /// safepoint in emission order, yet holds a live reference there on iteration 2+.
    /// An incomplete record is worse than none, because a hit suppresses the frame's
    /// conservative scan — so the omission would free a live object. R1 must name the
    /// slot regardless of declaration order.
    #[test]
    fn slot_declared_after_safepoint_is_still_named_loop_regression() {
        let mut b = StackMapBuilder::new(32);
        b.safepoint(0x20); // `call use(x)` at the loop top
        b.define_ref_slot(-8); // `x = alloc()` — declared afterwards
        let r = b.into_records();
        assert_eq!(
            r[0].slots,
            vec![-8],
            "the loop-carried reference must be named at the loop-top safepoint"
        );
    }

    /// Re-defining the same variable (the backends reuse one permanent slot per
    /// name) must not duplicate it in a record.
    #[test]
    fn repeated_definitions_are_idempotent() {
        let mut b = StackMapBuilder::new(32);
        b.define_ref_slot(-8);
        b.define_ref_slot(-8);
        b.define_ref_slot(-8);
        b.safepoint(0x08);
        assert_eq!(b.ref_slots(), &[-8]);
        assert_eq!(b.into_records()[0].slots, vec![-8]);
    }

    /// A repeated `pc_offset` collapses to ONE record. Two records at one PC would
    /// make `StackMapTable::lookup` (a binary search) return an arbitrary one of
    /// them, so the resolved slot set would be a coin flip.
    #[test]
    fn duplicate_safepoint_pcs_collapse_to_one_record() {
        let mut b = StackMapBuilder::new(16);
        b.safepoint(0x10);
        b.define_ref_slot(-8);
        b.safepoint(0x10); // same PC again
        let r = b.into_records();
        assert_eq!(r.len(), 1, "one record per PC");
        assert_eq!(r[0].slots, vec![-8]);
    }

    /// Safepoints recorded out of order still produce an ascending, well-formed
    /// table — the builder sorts, so a backend that lowers blocks out of layout
    /// order cannot corrupt lookups.
    #[test]
    fn out_of_order_safepoints_are_sorted() {
        let mut b = StackMapBuilder::new(16);
        b.safepoint(0x30);
        b.safepoint(0x08);
        b.safepoint(0x20);
        let pcs: Vec<u32> = b.into_records().iter().map(|r| r.pc_offset).collect();
        assert_eq!(pcs, vec![0x08, 0x20, 0x30]);
    }

    /// Extreme offsets round-trip intact (the format is `i32`, and the consumer adds
    /// them to the frame pointer in the wrapping `isize` domain).
    #[test]
    fn extreme_offsets_round_trip() {
        let mut b = StackMapBuilder::new(16);
        for off in [i32::MIN, -1, 0, 1, i32::MAX] {
            b.define_ref_slot(off);
        }
        b.safepoint(0);
        assert_eq!(
            b.into_records()[0].slots,
            vec![i32::MIN, -1, 0, 1, i32::MAX]
        );
    }

    /// Slots are emitted ascending regardless of definition order, so records are
    /// canonical and byte-comparable across runs.
    #[test]
    fn slots_are_sorted_regardless_of_definition_order() {
        let mut b = StackMapBuilder::new(64);
        for off in [-8, -40, 16, -24, 0] {
            b.define_ref_slot(off);
        }
        b.safepoint(0x0c);
        assert_eq!(b.into_records()[0].slots, vec![-40, -24, -8, 0, 16]);
    }

    /// A safepoint with nothing live still emits a record: that is the precise
    /// claim "no references here", where a *missing* record would instead demote the
    /// frame to a conservative scan.
    #[test]
    fn empty_safepoint_still_emits_a_record() {
        let mut b = StackMapBuilder::new(16);
        b.safepoint(0x20);
        let r = b.into_records();
        assert_eq!(r.len(), 1, "the record exists …");
        assert!(r[0].slots.is_empty(), "… and precisely names nothing");
    }

    /// A function with no safepoints produces no table at all.
    #[test]
    fn no_safepoints_means_empty() {
        let mut b = StackMapBuilder::new(16);
        assert!(b.is_empty());
        b.define_ref_slot(-8); // declaring without a safepoint emits nothing
        assert!(b.is_empty());
        assert!(b.into_records().is_empty());
    }

    /// The built table resolves a PC back to the slots that were live there — the
    /// round trip the runtime walker actually performs.
    #[test]
    fn into_table_round_trips_through_lookup() {
        let mut b = StackMapBuilder::new(48);
        b.define_ref_slot(-8);
        b.safepoint(0x10);
        b.define_ref_slot(-16);
        b.safepoint(0x24);

        let table = b.into_table();
        // Both safepoints resolve to the function's full reference-slot set (R1).
        assert_eq!(
            table.lookup(0x10).expect("record at 0x10").slots,
            vec![-16, -8]
        );
        assert_eq!(
            table.lookup(0x24).expect("record at 0x24").slots,
            vec![-16, -8]
        );
        assert!(table.lookup(0x18).is_none(), "not a safepoint → no record");
    }

    /// End-to-end with the consumer side: a record built here, resolved through
    /// `frame_root_slots`, yields the real root addresses for a frame.
    #[test]
    fn records_feed_frame_root_slots() {
        use crate::frame_root_slots;

        let mut b = StackMapBuilder::new(32);
        b.define_ref_slot(-8);
        b.define_ref_slot(-16);
        b.safepoint(0x10);
        let table = b.into_table();
        let rec = table.lookup(0x10).expect("record");

        // A frame pointer of 0x1000 → roots at 0x1000-16 and 0x1000-8.
        let mut roots = Vec::new();
        frame_root_slots(0x1000, rec, &mut roots);
        assert_eq!(roots, vec![0x1000 - 16, 0x1000 - 8]);
    }
}
