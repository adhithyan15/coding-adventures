//! # Stack-map registry — the code-address → live-reference lookup for precise roots
//!
//! Precise-root collection needs to answer, for **every live frame on the machine
//! stack**, the question `gc-core`'s [`StackMapTable`] answers *within one
//! function*: at this program counter, where are the live references?
//! [`StackMapTable::lookup`] is keyed by `pc_offset` — the distance of a safepoint
//! from its function's first byte. But the precise stack **walker** (a follow-up
//! PR) does not unwind to a `pc_offset`; it unwinds to an absolute **return
//! address** — an arbitrary code pointer somewhere in some compiled function. This
//! module is the missing link: a process-wide map from *absolute code address* to
//! *the [`StackMapRecord`] live there*.
//!
//! ```text
//!   return address (absolute)                     precise root slots
//!         │                                              ▲
//!         ▼                                              │
//!   resolve(ret) ──► find func [start,start+len) ──► StackMapTable::lookup(ret-start)
//! ```
//!
//! ## What a backend registers
//!
//! Each compiled function is registered **once** (at image start-up, before any
//! collection) via [`__gc_register_stackmap`](crate::__gc_register_stackmap) with:
//!
//! - `func_start` — the function's first instruction address (a code pointer), and
//! - `func_len` — its code length in bytes, so it occupies `[start, start + len)`, and
//! - its per-safepoint records, as **parallel flattened arrays** (the C-friendly
//!   shape a code generator emits): one `pc_offset`, `frame_size`,
//!   `callee_saved_mask` and `slot_count` per record, plus one concatenated
//!   `slots` array read record-by-record through the counts.
//!
//! `frame_size` and `callee_saved_mask` are not consumed by resolution — they are
//! carried through faithfully because the **walker** needs them (to step to the
//! caller's frame and to spill reference-holding callee-saved registers). Fixing
//! the ABI to carry them now, exactly as `gc-core`'s [`StackMapRecord`] carries
//! them ahead of the walker, means the format is settled once.
//!
//! ## Why the functions form a sorted, non-overlapping table
//!
//! Real compiled functions occupy disjoint code ranges. Keeping the registry
//! **sorted by `start`** lets [`resolve`] find the containing function by binary
//! search (`O(log n)` per unwound frame) instead of scanning every function. A
//! registration whose range would **overlap** an existing one is rejected (returns
//! `0`) rather than silently corrupting the table — overlap can only come from a
//! buggy caller, and a wrong function match would hand the collector wrong slot
//! offsets. A miss (an address in no registered function — a C runtime frame, or a
//! not-yet-migrated backend) simply returns [`None`]; the walker falls back to a
//! conservative scan of that frame, so a miss is safe, never a crash.
//!
//! ## Safety posture
//!
//! This module does **no** stack walking and dereferences **no** machine frame —
//! that is the walker's job. Its only `unsafe` is reading the caller-supplied
//! parallel arrays during registration, under the identical C-array contract
//! [`__gc_register_kind`](crate::__gc_register_kind) already relies on. Everything
//! else — the sorted insert, overlap rejection, binary-search resolution — is safe
//! Rust, and unit-tested here in isolation from the heap.

use gc_core::{StackMapRecord, StackMapTable};
use std::sync::Mutex;

/// One compiled function's code range paired with its per-safepoint stack maps.
///
/// The function occupies the half-open code range `[start, start + len)`; a return
/// address `ret` belongs to it iff `start <= ret < start + len`, and its
/// safepoint key is `pc_offset = ret - start`.
struct FuncStackMap {
    /// First instruction address of the function (an absolute code pointer).
    start: usize,
    /// Code length in bytes. The function occupies `[start, start + len)`.
    len: usize,
    /// Live-reference records for this function, keyed by `pc_offset` from `start`.
    ///
    /// Read by [`resolve`] — which the **precise stack walker** (a follow-up PR)
    /// calls per unwound frame. `#[allow(dead_code)]` until that caller lands: the
    /// registry is exercised through [`resolve`] in this crate's tests, but no
    /// non-test path consults it yet, exactly as `gc-core` shipped
    /// [`StackMapTable`] ahead of its consumer.
    #[allow(dead_code)]
    table: StackMapTable,
}

/// Process-wide registry of every compiled function's stack maps, **sorted by
/// `start`** and **non-overlapping** so [`resolve`] can binary-search by return
/// address. Populated once at start-up by
/// [`__gc_register_stackmap`](crate::__gc_register_stackmap); read by the precise
/// stack walker at every collection. Empty until the first registration.
///
/// A single `Mutex` matches the rest of this crate's state model (the native AOT
/// runtime is single-threaded; the lock simply makes the `static` sound and costs
/// nothing uncontended). A poisoned lock is recovered rather than propagated — a
/// GC data structure must not become unusable because some unrelated code panicked.
static REGISTRY: Mutex<Vec<FuncStackMap>> = Mutex::new(Vec::new());

/// Serialises the registry tests, which share the one process-wide [`REGISTRY`].
#[cfg(test)]
static REG_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Register one compiled function's stack maps. Returns the number of records
/// stored (`> 0`) on success, or `0` if the registration is rejected.
///
/// Rejections (return `0`, registry unchanged): `func_len == 0`,
/// `func_len > u32::MAX` (a `pc_offset` is a `u32`; a longer function would let
/// resolution truncate an address into the wrong record), `num_records <= 0`, a
/// required array pointer is null, `slots_flat` is null while some record's count
/// is positive (fail-loud against under-marking), the code range
/// `[start, start + len)` wraps the address space, or the range overlaps an
/// already-registered function.
///
/// The records are supplied as parallel flattened arrays, each of length
/// `num_records`, plus one concatenated `slots` array walked record-by-record:
/// record `i` names `slot_counts[i]` reference slots, taken in order from
/// `slots_flat`. A negative `slot_counts[i]` is clamped to `0` (that record has no
/// slots), mirroring how [`__gc_register_kind`](crate::__gc_register_kind) ignores
/// negative field offsets. `frame_sizes` and `callee_masks` may be null, read then
/// as all-zero (a first-cut backend that spills every live ref to the stack needs
/// neither).
///
/// # Safety
///
/// `pc_offsets` and `slot_counts` must each point to `num_records` readable words.
/// `frame_sizes` / `callee_masks` must likewise cover `num_records`, or be null.
/// `slots_flat` must cover the sum of the non-negative `slot_counts` (`int32`
/// words), or be null if that sum is `0`. This is the standard C-array contract;
/// the generated caller (or a test) upholds it.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn register(
    func_start: u64,
    func_len: u64,
    num_records: i64,
    pc_offsets: *const u32,
    frame_sizes: *const u32,
    callee_masks: *const u16,
    slot_counts: *const i32,
    slots_flat: *const i32,
) -> i64 {
    if func_len == 0 || num_records <= 0 || pc_offsets.is_null() || slot_counts.is_null() {
        return 0;
    }
    // A `pc_offset` is a `u32` (distance of a safepoint from `func_start`). A
    // function longer than `u32::MAX` therefore cannot be fully described, and —
    // worse — would let [`resolve`] compute `ret_addr - start` past `u32::MAX` and
    // **truncate** it to a `u32` that could alias a *real* record's `pc_offset`,
    // handing the walker the wrong slot set (an under-marking → use-after-free).
    // Reject such a range outright, so `resolve`'s cast is always exact. Real
    // compiled functions are kilobytes; this never fires on legitimate input.
    if func_len > u32::MAX as u64 {
        return 0;
    }
    let start = func_start as usize;
    let len = func_len as usize;
    // A range that wraps past usize::MAX is a bogus registration — reject it rather
    // than let the overlap arithmetic below wrap into a false "no overlap".
    if start.checked_add(len).is_none() {
        return 0;
    }
    let n = num_records as usize;

    // SAFETY: caller guarantees `pc_offsets` and `slot_counts` cover `n` words.
    let pcs = std::slice::from_raw_parts(pc_offsets, n);
    let counts = std::slice::from_raw_parts(slot_counts, n);

    // Rebuild the records, walking `slots_flat` with a running cursor: record `i`
    // consumes the next `slot_counts[i]` (non-negative) words.
    let mut records = Vec::with_capacity(n);
    let mut cursor = 0usize;
    for i in 0..n {
        let sc = if counts[i] < 0 { 0 } else { counts[i] as usize };
        let slots: Vec<i32> = if sc == 0 {
            Vec::new()
        } else if slots_flat.is_null() {
            // A record claims `sc > 0` slots but no `slots_flat` was supplied — a
            // contract violation (null is permitted only when every count is 0).
            // FAIL LOUD (reject the whole registration) rather than silently record
            // an empty slot list: an under-described safepoint would make the walker
            // miss a live root and free it (use-after-free). Nothing is inserted yet
            // (the loop runs before the lock), so this is a clean rejection.
            return 0;
        } else {
            // SAFETY: caller guarantees `slots_flat` covers the running sum of the
            // non-negative counts; `[cursor, cursor + sc)` lies within it.
            std::slice::from_raw_parts(slots_flat.add(cursor), sc).to_vec()
        };
        cursor += sc;
        // SAFETY: `frame_sizes`/`callee_masks` cover `n` words when non-null.
        let frame_size = if frame_sizes.is_null() { 0 } else { *frame_sizes.add(i) };
        let callee_saved_mask = if callee_masks.is_null() { 0 } else { *callee_masks.add(i) };
        records.push(StackMapRecord {
            pc_offset: pcs[i],
            frame_size,
            slots,
            callee_saved_mask,
        });
    }
    let table = StackMapTable::from_records(records);

    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    // Insertion point keeps the vector sorted by `start`. `pos` is the first entry
    // whose start is >= the new one; the new range must not overlap that entry
    // (to its right) nor `pos - 1` (to its left).
    let pos = reg.partition_point(|f| f.start < start);
    if pos < reg.len() && reg[pos].start < start + len {
        return 0; // overlaps the function to the right
    }
    if pos > 0 {
        let prev = &reg[pos - 1];
        if prev.start + prev.len > start {
            return 0; // overlaps the function to the left
        }
    }
    reg.insert(pos, FuncStackMap { start, len, table });
    n as i64
}

/// Resolve an absolute **return address** to the [`StackMapRecord`] live there, or
/// [`None`] if the address is in no registered function, or in a function but at a
/// PC with no safepoint record (an unmapped point — the walker scans that frame
/// conservatively).
///
/// `O(log n)` in the number of registered functions: a binary search for the
/// containing range, then [`StackMapTable::lookup`]'s binary search within it. The
/// record is cloned out so the registry lock is not held across the caller's use of
/// it.
///
/// `#[allow(dead_code)]` until the precise stack walker (a follow-up PR) calls this
/// per unwound frame; it is fully exercised by this module's unit tests today.
#[allow(dead_code)]
pub(crate) fn resolve(ret_addr: usize) -> Option<StackMapRecord> {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    // The containing function, if any, is the last one whose `start <= ret_addr`
    // (functions are sorted and disjoint). `partition_point` yields the count of
    // entries with `start <= ret_addr`, so `idx - 1` is that candidate.
    let idx = reg.partition_point(|f| f.start <= ret_addr);
    if idx == 0 {
        return None;
    }
    let f = &reg[idx - 1];
    // `start + len` cannot overflow — `register` rejected any range that would.
    if ret_addr < f.start + f.len {
        let pc = (ret_addr - f.start) as u32;
        f.table.lookup(pc).cloned()
    } else {
        None // in the gap after `f`, before the next function
    }
}

/// Number of functions currently registered. Introspection for tests and the
/// walker's diagnostics.
pub(crate) fn count() -> i64 {
    REGISTRY.lock().unwrap_or_else(|e| e.into_inner()).len() as i64
}

/// Drop every registered function's stack maps. Code maps normally live for the
/// whole process (they describe the image, not the heap), so this is **not** tied
/// to `__gc_reset`; it exists for deterministic test isolation and process
/// teardown.
pub(crate) fn reset() {
    REGISTRY.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Register a function whose records are given as `(pc_offset, slots)` pairs,
    /// through the same flattened-array ABI the C entry point uses. Returns the
    /// `register` result. `frame_sizes`/`callee_masks` are passed null (zero).
    unsafe fn register_simple(start: u64, len: u64, records: &[(u32, &[i32])]) -> i64 {
        let pcs: Vec<u32> = records.iter().map(|(pc, _)| *pc).collect();
        let counts: Vec<i32> = records.iter().map(|(_, s)| s.len() as i32).collect();
        let slots_flat: Vec<i32> = records.iter().flat_map(|(_, s)| s.iter().copied()).collect();
        register(
            start,
            len,
            records.len() as i64,
            pcs.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            counts.as_ptr(),
            if slots_flat.is_empty() { std::ptr::null() } else { slots_flat.as_ptr() },
        )
    }

    /// A return address inside a registered function resolves to the record at its
    /// `pc_offset`; an address outside the range, and a mapped-function address at
    /// an unmapped PC, both resolve to `None`.
    #[test]
    fn resolves_return_address_to_its_record() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        // Function at [0x1000, 0x1100) with safepoints at pc 0x10 and 0x40.
        let n = unsafe { register_simple(0x1000, 0x100, &[(0x10, &[8, -16]), (0x40, &[0])]) };
        assert_eq!(n, 2);
        assert_eq!(count(), 1);

        // Return address 0x1010 → pc_offset 0x10 → slots [8, -16].
        let rec = resolve(0x1010).expect("mapped safepoint resolves");
        assert_eq!(rec.pc_offset, 0x10);
        assert_eq!(rec.slots, vec![8, -16]);

        // Return address 0x1040 → pc_offset 0x40 → slots [0].
        assert_eq!(resolve(0x1040).unwrap().slots, vec![0]);

        // In range but at an unmapped PC → None (walker scans that frame conservatively).
        assert!(resolve(0x1020).is_none());
        // Below and above the range → None.
        assert!(resolve(0x0fff).is_none());
        assert!(resolve(0x1100).is_none(), "end is exclusive");
        assert!(resolve(0x2000).is_none());

        reset();
    }

    /// With several disjoint functions the binary search picks the correct one,
    /// regardless of registration order.
    #[test]
    fn binary_search_picks_the_containing_function() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        // Register out of address order; resolution must still be correct.
        unsafe {
            assert_eq!(register_simple(0x3000, 0x100, &[(0, &[24])]), 1);
            assert_eq!(register_simple(0x1000, 0x100, &[(0, &[8])]), 1);
            assert_eq!(register_simple(0x2000, 0x100, &[(0, &[16])]), 1);
        }
        assert_eq!(count(), 3);

        assert_eq!(resolve(0x1000).unwrap().slots, vec![8]);
        assert_eq!(resolve(0x2000).unwrap().slots, vec![16]);
        assert_eq!(resolve(0x3000).unwrap().slots, vec![24]);
        // A gap between functions resolves to nothing.
        assert!(resolve(0x1500).is_none());

        reset();
    }

    /// Slots for multiple records are demultiplexed from the one concatenated
    /// `slots_flat` array by walking the per-record counts.
    #[test]
    fn flattened_slots_are_demultiplexed_by_count() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        // Three records with 1, 3, then 0 slots — the cursor must advance by each
        // count so record 2 gets exactly its middle three.
        let n = unsafe {
            register_simple(0x4000, 0x100, &[(4, &[100]), (8, &[200, 201, 202]), (12, &[])])
        };
        assert_eq!(n, 3);
        assert_eq!(resolve(0x4004).unwrap().slots, vec![100]);
        assert_eq!(resolve(0x4008).unwrap().slots, vec![200, 201, 202]);
        assert_eq!(resolve(0x400c).unwrap().slots, Vec::<i32>::new());

        reset();
    }

    /// `frame_size` and `callee_saved_mask` are stored faithfully (they are carried
    /// for the walker), when supplied through the full array form.
    #[test]
    fn frame_size_and_mask_are_carried_through() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        let pcs = [0u32];
        let frames = [48u32];
        let masks = [0b101u16];
        let counts = [1i32];
        let slots = [16i32];
        let n = unsafe {
            register(
                0x5000,
                0x80,
                1,
                pcs.as_ptr(),
                frames.as_ptr(),
                masks.as_ptr(),
                counts.as_ptr(),
                slots.as_ptr(),
            )
        };
        assert_eq!(n, 1);
        let rec = resolve(0x5000).unwrap();
        assert_eq!(rec.frame_size, 48);
        assert_eq!(rec.callee_saved_mask, 0b101);
        assert_eq!(rec.slots, vec![16]);

        reset();
    }

    /// Overlapping registrations are rejected (return `0`, registry unchanged),
    /// whether the newcomer overlaps to the left or the right of an existing entry.
    #[test]
    fn overlapping_registration_is_rejected() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        assert_eq!(unsafe { register_simple(0x1000, 0x100, &[(0, &[8])]) }, 1);

        // Exact same range, a range straddling the start, and one straddling the
        // end all overlap → all rejected.
        assert_eq!(unsafe { register_simple(0x1000, 0x100, &[(0, &[8])]) }, 0);
        assert_eq!(unsafe { register_simple(0x0f80, 0x100, &[(0, &[8])]) }, 0);
        assert_eq!(unsafe { register_simple(0x1080, 0x100, &[(0, &[8])]) }, 0);
        assert_eq!(count(), 1, "no overlapping entry was inserted");

        // An abutting range (starts exactly where the first ends) does NOT overlap.
        assert_eq!(unsafe { register_simple(0x1100, 0x100, &[(0, &[8])]) }, 1);
        assert_eq!(count(), 2);

        reset();
    }

    /// Degenerate registrations are rejected without touching the registry.
    #[test]
    fn degenerate_registrations_rejected() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        // Zero length, non-positive record count, and a null required array.
        assert_eq!(unsafe { register_simple(0x1000, 0, &[(0, &[8])]) }, 0);
        assert_eq!(unsafe { register_simple(0x1000, 0x100, &[]) }, 0);
        let counts = [1i32];
        let slots = [8i32];
        assert_eq!(
            unsafe {
                register(
                    0x1000,
                    0x100,
                    1,
                    std::ptr::null(), // null pc_offsets → reject
                    std::ptr::null(),
                    std::ptr::null(),
                    counts.as_ptr(),
                    slots.as_ptr(),
                )
            },
            0
        );
        // A range that wraps the address space is rejected.
        assert_eq!(unsafe { register_simple(u64::MAX - 4, 0x100, &[(0, &[8])]) }, 0);
        // slots_flat null while a record claims a positive count is rejected
        // (fail-loud against under-marking), and nothing is inserted.
        let pcs = [0u32];
        let counts = [2i32]; // claims 2 slots...
        assert_eq!(
            unsafe {
                register(
                    0x8000,
                    0x80,
                    1,
                    pcs.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    counts.as_ptr(),
                    std::ptr::null(), // ...but no slots array
                )
            },
            0
        );
        // A function longer than u32::MAX is rejected: pc_offset is a u32, so a
        // longer function would let resolve truncate an address into a wrong record.
        assert_eq!(
            unsafe { register_simple(0x1000, u32::MAX as u64 + 1, &[(0, &[8])]) },
            0
        );
        // Exactly u32::MAX is the largest accepted length.
        assert_eq!(
            unsafe { register_simple(0x1_0000_0000, u32::MAX as u64, &[(0, &[8])]) },
            1
        );
        assert_eq!(count(), 1);

        reset();
    }

    /// A negative `slot_count` is clamped to zero (that record simply has no slots),
    /// and does not disturb the cursor for the following record.
    #[test]
    fn negative_slot_count_is_clamped() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();

        let pcs = [0u32, 4u32];
        let counts = [-3i32, 2i32]; // first record's count is negative
        let slots = [77i32, 88i32]; // belong to the SECOND record
        let n = unsafe {
            register(
                0x6000,
                0x80,
                2,
                pcs.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                counts.as_ptr(),
                slots.as_ptr(),
            )
        };
        assert_eq!(n, 2);
        assert_eq!(resolve(0x6000).unwrap().slots, Vec::<i32>::new());
        assert_eq!(resolve(0x6004).unwrap().slots, vec![77, 88]);

        reset();
    }

    /// `reset` empties the registry; `count` tracks it.
    #[test]
    fn reset_clears_the_registry() {
        let _g = REG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        assert_eq!(count(), 0);
        assert_eq!(unsafe { register_simple(0x7000, 0x40, &[(0, &[0])]) }, 1);
        assert_eq!(count(), 1);
        reset();
        assert_eq!(count(), 0);
        assert!(resolve(0x7000).is_none());
    }
}
