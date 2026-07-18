//! # Precise stack walk — building the root set from a live frame-pointer chain
//!
//! This is the platform layer that turns a running machine stack into the two
//! root inputs [`gc_core::FlatHeap::collect_mixed`] consumes: **precise slots** for
//! the frames a backend stack-mapped, and **conservative regions** for the frames
//! it did not. It is the precise-root analogue of [`stack_scan`](crate::stack_scan)
//! (the fully-conservative C-stack scan): where that hands the whole `[sp, base)`
//! span to `collect_region`, this classifies the stack **frame by frame**.
//!
//! ## The frame-pointer chain
//!
//! Both supported ABIs (AArch64 AAPCS, x86-64 SysV/Win64) keep a **frame-pointer
//! chain** when frame pointers are enabled: a function's prologue saves the
//! caller's frame pointer and the return address at the top of its frame, so at any
//! frame pointer `fp`:
//!
//! ```text
//!   [fp]      = the caller's saved frame pointer  (a higher stack address)
//!   [fp + 8]  = the return address into the caller's code
//! ```
//!
//! Every supported target grows the stack **downward**, so the chain of frame
//! pointers is strictly **increasing** as we walk outward toward the stack base.
//! That monotonicity is also the walk's termination and sanity guarantee: a
//! `caller_fp` that is not strictly above the current `fp` (or lands outside the
//! stack) means the chain is broken, corrupt, or we reached the top — we stop and
//! conservatively scan whatever stack remains.
//!
//! ## Which frame owns which return address
//!
//! The return address at `[fp + 8]` is a program counter in the **caller's**
//! function — the instruction after the `call`/`bl` that entered the current frame.
//! The caller's frame pointer is `caller_fp = [fp]`, and a stack map for that
//! return address describes the caller's live references as offsets from
//! `caller_fp`. So each step resolves `[fp + 8]` and, if it maps, emits precise
//! slots relative to `caller_fp`; if it does not, emits the caller's frame span
//! `[fp, caller_fp)` as one conservative region. (`fp` and `caller_fp` bracket the
//! caller's locals, because the callee's frame sits just below the caller's.)
//!
//! ## Why this is sound — no live root is ever missed
//!
//! The union of everything the walk emits **covers every stack word that could be
//! a heap reference**:
//!
//! - `[sp, start_fp)` — the collector's own frames, below the first walked frame
//!   pointer — is always emitted as one conservative region (those frames are never
//!   stack-mapped).
//! - Each `[fp, caller_fp)` span is emitted either conservatively (unmapped caller)
//!   or, for a mapped caller, *replaced* by that caller's exact reference slots. The
//!   only stack words a mapped frame excludes from scanning are its non-reference
//!   locals and the saved-fp / return-address words — none of which is ever a heap
//!   pointer (a saved frame pointer is a stack address; a return address is a code
//!   address), so skipping them cannot drop a root.
//! - If the chain breaks (a `caller_fp` fails the sanity check), the **entire
//!   remaining span** `[fp, base)` is scanned conservatively before stopping.
//!
//! So precision is strictly additive: a mapped frame contributes only its real
//! references (removing floating garbage), while everything else — and any frame we
//! cannot trust — stays as safe as today's fully-conservative scan. A false
//! positive from a conservative span only retains a dead object one cycle; a missed
//! root would be a use-after-free, and the coverage argument above shows one cannot
//! occur.
//!
//! This module does the **walk logic only** — it takes the frame pointer, stack
//! pointer and stack base as plain integers, so it is exhaustively unit-testable
//! against *synthetic* stacks (arrays that simulate frames and return addresses)
//! with no `asm!` and no real thread stack. Capturing the real `fp`/`sp`/`base` of
//! the running thread and calling this is the tiny `asm!` entry point layered on top
//! (a follow-up), exactly as [`stack_scan`](crate::stack_scan) layers its `asm!`
//! capture on `collect_region`.

use crate::stackmap_registry;
use core::ptr;
use gc_core::frame_root_slots;

/// Upper bound on frame-pointer-chain length, a corruption backstop. The strictly-
/// increasing `caller_fp` check already forces termination (each step moves `fp`
/// strictly toward `base`), so this only fences off a pathologically deep or
/// adversarial chain. Real stacks are thousands of frames at most; a million is
/// unreachable in practice yet bounds the loop unconditionally.
const MAX_FRAMES: usize = 1 << 20;

/// Walk the frame-pointer chain rooted at `start_fp` (toward the higher-addressed
/// stack `base`), appending the precise root **slots** and conservative **regions**
/// it discovers to `slots` and `regions` — the two inputs to
/// [`gc_core::FlatHeap::collect_mixed`].
///
/// `sp` is the current stack pointer (lowest live address) and `base` the thread's
/// stack base (highest); `start_fp` is the frame pointer to begin from. All three
/// are plain addresses so this is testable without a real stack.
///
/// The classification, coverage and soundness argument are in the module docs. In
/// brief: `[sp, start_fp)` is emitted conservatively; then for each frame, the
/// caller's return address `[fp + 8]` is resolved — a hit emits precise slots
/// relative to `caller_fp = [fp]`, a miss emits the caller's span `[fp, caller_fp)`
/// conservatively; a broken chain emits the remaining `[fp, base)` and stops.
///
/// # Safety
///
/// `[sp, base)` must be a readable span of the current thread's stack for the
/// duration of the call (the frame-pointer reads `[fp]` and `[fp + 8]` all lie
/// within it, gated by `fp + 16 <= base`). A `start_fp` **outside** `[sp, base]`, or
/// degenerate bounds (`sp == 0` or `sp >= base`), is handled defensively: the walk
/// conservatively scans the whole readable `[sp, base)` (or nothing, if the bounds
/// are unusable) rather than dropping roots — it never trusts a chain it cannot
/// place. The emitted slot addresses and regions are only *recorded* here, not
/// dereferenced — `collect_mixed` reads them later under its own contract.
///
/// `#[allow(dead_code)]` until the `asm!` entry point (`__gc_collect_precise`, a
/// follow-up PR) captures the running thread's `fp`/`sp`/`base` and calls this; it
/// is exhaustively exercised by this module's synthetic-stack unit tests today,
/// exactly as `gc-core` shipped `collect_mixed` ahead of its consumer.
#[allow(dead_code)]
pub(crate) unsafe fn build_precise_roots(
    start_fp: usize,
    sp: usize,
    base: usize,
    slots: &mut Vec<usize>,
    regions: &mut Vec<(*const u8, usize)>,
) {
    build_precise_roots_bounded(start_fp, sp, base, MAX_FRAMES, slots, regions)
}

/// [`build_precise_roots`] with an explicit frame budget, so the `MAX_FRAMES`
/// exhaustion path (which then conservatively scans the remainder) is unit-testable
/// without a million-frame stack. The public entry fixes `max_frames = MAX_FRAMES`.
///
/// # Safety
///
/// Same contract as [`build_precise_roots`].
unsafe fn build_precise_roots_bounded(
    start_fp: usize,
    sp: usize,
    base: usize,
    max_frames: usize,
    slots: &mut Vec<usize>,
    regions: &mut Vec<(*const u8, usize)>,
) {
    // Degenerate stack bounds (null, or non-increasing) — nothing safe to scan.
    if sp == 0 || sp >= base {
        return;
    }
    // A `start_fp` outside the live stack means the chain cannot be trusted at all.
    // Rather than the catastrophic failure of dropping *every* root (walking nothing),
    // fall back to conservatively scanning the whole stack — never worse than the
    // fully-conservative `__gc_collect`. This is defense-in-depth for the `asm!`
    // entry point that will feed real captured registers.
    if start_fp < sp || start_fp > base {
        regions.push((sp as *const u8, base - sp));
        return;
    }
    // `start_fp ∈ [sp, base]`. The collector's own frames, below the first frame
    // pointer, are never stack-mapped — scan `[sp, start_fp)` conservatively (empty
    // and skipped when `start_fp == sp`).
    if start_fp > sp {
        regions.push((sp as *const u8, start_fp - sp));
    }

    let mut fp = start_fp;
    let mut frames = 0usize;
    while frames < max_frames {
        frames += 1;

        // Need to read the two 8-byte words at [fp] and [fp+8]; both must lie inside
        // the stack span. `fp + 16 <= base` keeps the reads in-bounds (and rejects a
        // wrapping `fp`). A frame pointer at/near the base with no room for its own
        // link record means we have reached the top of the walkable stack: the only
        // stack left above `fp` is a sub-16-byte tail whose aligned words are this
        // frame's saved-fp / return-address (never a heap pointer). Returning here —
        // the *sole* early exit that skips the remainder scan — is therefore sound.
        if fp < sp || fp.checked_add(16).is_none_or(|end| end > base) {
            return;
        }

        // SAFETY: `[fp, fp+16) ⊆ [sp, base)` is readable per the contract and the
        // bound just checked; `read_unaligned` tolerates any alignment.
        let caller_fp = ptr::read_unaligned(fp as *const usize);
        let ret = ptr::read_unaligned((fp + 8) as *const usize);

        // Chain sanity: the caller's frame pointer must be strictly above this one
        // (stack grows down) and stay within the stack. Anything else — a null
        // terminator, a corrupt link, or the outermost frame — means we can trust the
        // chain no further, so we `break` and (below) conservatively scan whatever
        // stack remains, never following the bad pointer.
        if caller_fp <= fp || caller_fp > base {
            break;
        }

        // The caller's frame occupies `[fp, caller_fp)`; `[fp+8]` is its safepoint.
        match stackmap_registry::resolve(ret) {
            // Mapped: precise slots relative to the caller's frame pointer. The
            // caller's span is *not* scanned conservatively — that is the precision.
            Some(rec) => frame_root_slots(caller_fp, &rec, slots),
            // Unmapped: the caller's whole frame span is a conservative region.
            None => regions.push((fp as *const u8, caller_fp - fp)),
        }

        fp = caller_fp;
    }

    // We exited the loop by `break` (a broken/untrustworthy chain link) OR by
    // exhausting `MAX_FRAMES` (a pathologically long chain). In BOTH cases `fp` is a
    // valid in-stack address with potentially-unscanned stack above it — scan the
    // whole remainder `[fp, base)` conservatively so no live reference is dropped.
    // (The top-of-stack case returned early above, so it never reaches here.)
    if fp >= sp && fp < base {
        regions.push((fp as *const u8, base - fp));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::__gc_register_stackmap;

    /// Serialises against every other registry-touching test (the shared
    /// process-wide `REGISTRY`), then resets it so registrations don't leak between
    /// cases. Takes the SAME lock the `stackmap_registry` tests use, so a parallel
    /// test runner cannot interleave registrations underneath us.
    fn with_clean_registry<R>(f: impl FnOnce() -> R) -> R {
        let _guard = stackmap_registry::REG_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        crate::__gc_stackmap_reset();
        let r = f();
        crate::__gc_stackmap_reset();
        r
    }

    /// Register a one-record stack map for a synthetic function `[start, start+len)`
    /// whose safepoint at `pc` names `slot_offsets` (FP-relative byte offsets).
    unsafe fn register(start: usize, len: usize, pc: u32, slot_offsets: &[i32]) {
        let pcs = [pc];
        let counts = [slot_offsets.len() as i32];
        let n = __gc_register_stackmap(
            start as u64,
            len as u64,
            1,
            pcs.as_ptr(),
            core::ptr::null(),
            core::ptr::null(),
            counts.as_ptr(),
            slot_offsets.as_ptr(),
        );
        assert_eq!(n, 1, "stackmap registration should succeed");
    }

    /// Build a synthetic down-growing stack in a `usize` array (higher index = higher
    /// address, matching a stack whose caller frames are at higher addresses). Returns
    /// the base address of `stack[0]`. Frame `k` has its frame pointer at index
    /// `2*k + 2`; `[fp]` (that index) holds the next frame pointer and `[fp+8]` (the
    /// next index) holds the return address. The caller fills those in.
    fn addr_of(stack: &[usize], idx: usize) -> usize {
        stack.as_ptr() as usize + idx * core::mem::size_of::<usize>()
    }

    /// With no stack maps registered, every frame is unmapped: the walk emits only
    /// conservative regions (and no precise slots), and they tile the stack.
    #[test]
    fn all_unmapped_frames_become_conservative_regions() {
        with_clean_registry(|| {
            // stack layout (index → contents):
            //   0,1: collector locals (below first fp)
            //   2:   fp0 → [fp0] = &stack[4] (caller_fp = fp1)
            //   3:            [fp0+8] = ret0 (unmapped)
            //   4:   fp1 → [fp1] = 0 (chain terminates)
            //   5:            [fp1+8] = ret1
            let mut stack = [0usize; 6];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let fp1 = addr_of(&stack, 4);
            let base = addr_of(&stack, 6); // one past the end
            stack[2] = fp1; // [fp0] = fp1
            stack[3] = 0xaaaa; // ret0 (nothing registered → unmapped)
            stack[4] = 0; // [fp1] = 0 → terminate
            stack[5] = 0xbbbb; // ret1

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            assert!(slots.is_empty(), "no maps → no precise slots");
            // Expect: [sp, fp0) collector region, then [fp0, fp1) unmapped caller, then
            // the terminator at fp1 emits the remaining [fp1, base).
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            assert_eq!(
                got,
                vec![(sp, fp0 - sp), (fp0, fp1 - fp0), (fp1, base - fp1)],
                "conservative regions tile the stack"
            );
        });
    }

    /// A mapped frame contributes precise slots (at `caller_fp + offset`) instead of
    /// a conservative region for that frame.
    #[test]
    fn mapped_frame_becomes_precise_slots() {
        with_clean_registry(|| {
            let mut stack = [0usize; 6];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let fp1 = addr_of(&stack, 4);
            let base = addr_of(&stack, 6);
            stack[2] = fp1; // [fp0] = fp1 (caller_fp)
            stack[3] = 0x1000; // ret0 — will be mapped
            stack[4] = 0; // terminate
            stack[5] = 0;

            // Map ret0 = 0x1000 into a function [0x1000, 0x1100) with a safepoint at
            // pc 0 naming two ref slots at offsets 0 and -8 (relative to caller_fp=fp1).
            unsafe { register(0x1000, 0x100, 0, &[0, -8]) };

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            // The mapped caller (fp1) contributes exact slot addresses, not a region.
            assert_eq!(
                slots,
                vec![fp1, (fp1 as isize - 8) as usize],
                "precise slots are caller_fp + offset"
            );
            // Regions: [sp, fp0) collector frame, then the terminator's [fp1, base).
            // The mapped frame's [fp0, fp1) span is NOT scanned conservatively.
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            assert_eq!(got, vec![(sp, fp0 - sp), (fp1, base - fp1)]);
        });
    }

    /// A mix: an unmapped inner frame (region) and a mapped outer frame (slots) in the
    /// same walk.
    #[test]
    fn mixed_mapped_and_unmapped_frames() {
        with_clean_registry(|| {
            // Three frames: fp0 (caller ret0 unmapped), fp1 (caller ret1 mapped), then
            // terminate at fp2.
            let mut stack = [0usize; 8];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let fp1 = addr_of(&stack, 4);
            let fp2 = addr_of(&stack, 6);
            let base = addr_of(&stack, 8);
            stack[2] = fp1; // [fp0]=fp1
            stack[3] = 0x2222; // ret0 unmapped
            stack[4] = fp2; // [fp1]=fp2
            stack[5] = 0x3000; // ret1 mapped
            stack[6] = 0; // terminate
            stack[7] = 0;

            unsafe { register(0x3000, 0x80, 0, &[16]) }; // slot at fp2 + 16

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            assert_eq!(slots, vec![fp2 + 16], "mapped outer frame's precise slot");
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            assert_eq!(
                got,
                vec![
                    (sp, fp0 - sp),   // collector frame
                    (fp0, fp1 - fp0), // unmapped inner caller
                    (fp2, base - fp2) // terminator remainder (fp1's mapped span excluded)
                ]
            );
        });
    }

    /// A `caller_fp` that is not strictly above `fp` (a corrupt or backward link) is
    /// rejected: the walk conservatively scans the rest of the stack and stops,
    /// never following the bad pointer.
    #[test]
    fn backward_caller_fp_scans_remainder_and_stops() {
        with_clean_registry(|| {
            let mut stack = [0usize; 6];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let base = addr_of(&stack, 6);
            // [fp0] points BELOW fp0 (backward) — a broken chain. (ret at stack[3] is
            // never read, because the walk breaks on the bad link before resolving it.)
            stack[2] = addr_of(&stack, 1);

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            assert!(slots.is_empty());
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            // [sp, fp0) collector, then the whole remainder [fp0, base) conservatively.
            assert_eq!(got, vec![(sp, fp0 - sp), (fp0, base - fp0)]);
        });
    }

    /// An out-of-range `caller_fp` (above the stack base) is likewise rejected.
    #[test]
    fn out_of_range_caller_fp_scans_remainder_and_stops() {
        with_clean_registry(|| {
            let mut stack = [0usize; 6];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let base = addr_of(&stack, 6);
            stack[2] = base + 0x1000; // caller_fp beyond the stack base (ret unread)

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            assert!(slots.is_empty());
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            assert_eq!(got, vec![(sp, fp0 - sp), (fp0, base - fp0)]);
        });
    }

    /// A `start_fp` outside `[sp, base]` — which would otherwise walk no frames and
    /// drop every root — falls back to conservatively scanning the whole stack, and
    /// degenerate bounds emit nothing. Defense-in-depth for the `asm!` entry point.
    #[test]
    fn degenerate_start_fp_falls_back_to_full_conservative_scan() {
        with_clean_registry(|| {
            let stack = [0usize; 6];
            let sp = addr_of(&stack, 0);
            let base = addr_of(&stack, 6);

            // start_fp below the stack → full conservative fallback [sp, base).
            let mut slots = Vec::new();
            let mut regions = Vec::new();
            unsafe { build_precise_roots(sp - 64, sp, base, &mut slots, &mut regions) };
            assert!(slots.is_empty());
            assert_eq!(
                regions.iter().map(|&(b, l)| (b as usize, l)).collect::<Vec<_>>(),
                vec![(sp, base - sp)],
                "start_fp below the stack → whole stack scanned, roots never dropped"
            );

            // start_fp above the base → same full fallback.
            let mut slots2 = Vec::new();
            let mut regions2 = Vec::new();
            unsafe { build_precise_roots(base + 64, sp, base, &mut slots2, &mut regions2) };
            assert_eq!(
                regions2.iter().map(|&(b, l)| (b as usize, l)).collect::<Vec<_>>(),
                vec![(sp, base - sp)]
            );

            // Degenerate bounds (sp >= base) emit nothing (no usable stack).
            let mut slots3 = Vec::new();
            let mut regions3 = Vec::new();
            unsafe { build_precise_roots(base, base, sp, &mut slots3, &mut regions3) };
            assert!(slots3.is_empty() && regions3.is_empty());

            core::hint::black_box(&stack);
        });
    }

    /// Exhausting the frame budget mid-stack does NOT drop the unscanned remainder:
    /// the walk falls back to conservatively scanning `[fp, base)` before stopping,
    /// so a pathologically long (or corrupt) chain can never under-mark. Driven with
    /// a `max_frames` of 1 over a two-frame chain.
    #[test]
    fn frame_budget_exhaustion_scans_remainder() {
        with_clean_registry(|| {
            let mut stack = [0usize; 8];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let fp1 = addr_of(&stack, 4);
            let fp2 = addr_of(&stack, 6);
            let base = addr_of(&stack, 8);
            stack[2] = fp1; // [fp0]=fp1
            stack[3] = 0x5555; // ret0 (unmapped)
            stack[4] = fp2; // [fp1]=fp2 — a second frame the budget won't reach
            stack[5] = 0x6666;

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            core::hint::black_box(&stack);
            // Budget of 1: process only frame fp0, then bail — but the remainder
            // [fp1, base) must still be scanned conservatively.
            unsafe { build_precise_roots_bounded(fp0, sp, base, 1, &mut slots, &mut regions) };

            assert!(slots.is_empty());
            let got: Vec<(usize, usize)> =
                regions.iter().map(|&(b, l)| (b as usize, l)).collect();
            assert_eq!(
                got,
                vec![
                    (sp, fp0 - sp),   // collector frame
                    (fp0, fp1 - fp0), // the one frame processed (unmapped → region)
                    (fp1, base - fp1) // budget exhausted → remainder scanned, not dropped
                ],
                "the unscanned remainder is conservatively covered"
            );
        });
    }

    /// End-to-end through `collect_mixed`: a synthetic stack precisely names a live
    /// heap object while an adjacent unnamed local's object — sitting *inside the
    /// same mapped frame*, so it is excluded from every conservative region — is
    /// reclaimed. Proves the walk's output drives a correct precise collection.
    #[test]
    fn walk_output_drives_precise_collection() {
        use gc_core::FlatHeap;
        with_clean_registry(|| {
            let mut heap = FlatHeap::new();
            let live = heap.alloc(16, 0) as usize; // named by a slot → survives
            let dead = heap.alloc(16, 0) as usize; // unnamed local of the mapped frame

            // Layout (index → contents):
            //   0,1: collector locals (region [sp, fp0) — both zero, no roots)
            //   2:   fp0 → [fp0] = fp1 (caller_fp)
            //   3:            ret0 = 0x4000 (MAPPED)
            //   4:   `live`  — the mapped frame's local at fp1-16 (NAMED)
            //   5:   `dead`  — the mapped frame's local at fp1-8  (UNNAMED)
            //   6:   fp1 → [fp1] = 0 (terminate)
            //   7:            0
            // The mapped caller has frame pointer fp1 and span [fp0, fp1) = indices
            // 2..6, which holds both `live` and `dead`. Because that span is mapped it
            // is NOT scanned conservatively; only the named slot (fp1-16 → `live`) is
            // rooted, so `dead` at fp1-8 is reclaimed.
            let mut stack = [0usize; 8];
            let sp = addr_of(&stack, 0);
            let fp0 = addr_of(&stack, 2);
            let fp1 = addr_of(&stack, 6);
            let base = addr_of(&stack, 8);
            stack[2] = fp1; // [fp0] = fp1
            stack[3] = 0x4000; // ret0 → mapped
            stack[4] = live; // fp1 - 16 : NAMED slot
            stack[5] = dead; // fp1 - 8  : unnamed local
            stack[6] = 0; // [fp1] = 0 → terminate (remainder [fp1, base) is zeros)
            stack[7] = 0;

            unsafe { register(0x4000, 0x80, 0, &[-16]) }; // names only fp1 - 16 (`live`)

            let mut slots = Vec::new();
            let mut regions = Vec::new();
            // The walk reads the synthetic frames through raw pointers the borrow
            // checker can't see; black_box keeps every slot write live (and unelided).
            core::hint::black_box(&stack);
            unsafe { build_precise_roots(fp0, sp, base, &mut slots, &mut regions) };

            // The one precise slot is fp1 - 16 = &stack[4], which holds `live`.
            assert_eq!(slots, vec![(fp1 as isize - 16) as usize]);

            // Only `live`'s address is rooted (the single named slot); `dead` is in no
            // slot and in no region (it sits inside the excluded mapped span). So
            // exactly one 16-byte object — `live` — must survive.
            let _ = dead; // its address is deliberately rooted nowhere
            let stats = unsafe { heap.collect_mixed(&slots, &regions) };
            assert_eq!(stats.freed, 1, "the unnamed local is reclaimed");
            assert_eq!(heap.object_count(), 1, "exactly one object survives");
            assert_eq!(heap.live_bytes(), 16, "the survivor is the 16-byte `live`");
            assert_eq!(stack[4], live); // keep the stack live to the end
        });
    }
}
