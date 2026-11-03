//! # Global-root registry — word slots outside the stack that every collection roots
//!
//! The precise and compacting collectors find their roots by walking the machine
//! stack ([`crate::precise_walk`]): a stack-mapped frame contributes the exact slots
//! its map names, an unmapped frame its whole span. That covers every *local*. It
//! does not cover a compiled program's **globals**: a top-level Twig `(define g …)`
//! lives in a word of the image's writable data section (`_twig_globals`), not on
//! the stack, so no stack map names it and no conservative region contains it.
//!
//! Before this registry existed that made every global a dangling reference waiting
//! to happen (#18765):
//!
//! | collection        | what happened to an object only a global named                         |
//! |-------------------|-------------------------------------------------------------------------|
//! | precise           | unmarked, so **freed**; the global then named a freed block              |
//! | compacting        | if a stack slot also named it, **moved** and that slot rewritten — the global kept the from-space address, which the sweep frees |
//! | conservative (`__gc_collect`, allocation-triggered) | unmarked, so **freed**                                   |
//!
//! In practice a stale stack word (a frame slot not yet written on this path, or a
//! dead callee's frame inside the collector's conservative gap) often kept the
//! object alive by accident, which is why the bug survived testing. Compaction
//! defeats that luck: it moves the object and rewrites the stack words, but not the
//! global.
//!
//! ## The design
//!
//! A program registers each block of global slots **once, at start-up**, with
//! [`__gc_register_global_roots`](crate::__gc_register_global_roots)`(base, count)`.
//! twig-aot's generated `__gc_init_stackmaps` makes that call next to its stack-map
//! registrations, passing `&_twig_globals` and the module's global count.
//!
//! Every collection entry then appends the address of every registered slot to the
//! **precise root slots** it hands the heap ([`extend_slots`]) — exactly the role a
//! stack map's named slots play:
//!
//! ```text
//!   stack walk ──► slots (stack-mapped frames) ─┐
//!                  regions (unmapped frames)    │
//!   global roots ► slots (every global slot) ───┴──► collect_{mixed,compacting,…}
//! ```
//!
//! So a global's referent is **marked** by every collector, and the compacting ones
//! **rewrite** the global in place when they move it (the heap writes the forwarded
//! address back through each root-slot address, just as it does for a stack slot).
//!
//! ### Why precise slots, not a conservative region
//!
//! Passing the global block as a conservative `(base, len)` region would also be
//! sound, but a conservative root **pins** its target and, transitively, everything
//! reachable from it (`FlatHeap::classify_mobility`'s pinning wave): a global
//! holding a large list would stop that whole list from ever being compacted.
//! Precise slots keep global-reachable data movable. The cost is the one already
//! accepted for every dynamic (`any`) stack slot (`AOT00-T1-stackmap-emission.md`
//! §4.1 item 3): a global holding a plain integer that happens to equal a moved
//! object's address, or one of its tagged forms, is rewritten. Every global slot is
//! registered, not only those a type analysis says may hold a heap word: a filter
//! would make soundness depend on every store into a global being typed right,
//! which is exactly the property #18492 and #18766 showed does not hold.
//!
//! ### Why a registry and not a symbol the collector looks up
//!
//! The collector is a library linked into images built by several code generators
//! on three object formats; it cannot name a program's data symbol. A registration
//! call keeps the ABI the same shape as [`__gc_register_stackmap`](crate::__gc_register_stackmap):
//! the image describes itself to the runtime at start-up.
//!
//! ## Safety posture
//!
//! [`register`] dereferences nothing; it validates and stores the range. The
//! *collections* read and (when compacting) write each registered slot, so the
//! registration contract is that the range stays readable and writable for as long
//! as it is registered — true for a data section, which lives as long as the image.

use std::sync::Mutex;

/// Upper bound on the slots one registration may name: 2²⁴ words (128 MiB).
/// twig-aot's AArch64 backend addresses at most 4 096 globals and x86-64 at most
/// `i32::MAX / 8`, so a legitimate image is far below this. It exists so a corrupt
/// `count` is rejected instead of making every collection push billions of slots.
pub(crate) const MAX_GLOBAL_ROOT_SLOTS: i64 = 1 << 24;

/// The registered blocks, as `(base address, slot count)`. Kept sorted by `base`
/// and non-overlapping (see [`register`]). A `Mutex` for the same reason as the
/// stack-map registry: the runtime is single-threaded, the lock makes the `static`
/// sound and is uncontended. A poisoned lock is recovered, never propagated.
static GLOBAL_ROOTS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

fn roots() -> std::sync::MutexGuard<'static, Vec<(usize, usize)>> {
    GLOBAL_ROOTS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Register `count` 8-byte slots starting at `base` as precise roots for every
/// later collection. Returns `count` on success (also when this exact block is
/// already registered — registration is idempotent), or `0` if rejected.
///
/// Rejections (registry unchanged):
///
/// | input                                              | why                                                     |
/// |----------------------------------------------------|---------------------------------------------------------|
/// | `base` null, `count <= 0`                          | nothing to register                                     |
/// | `count > MAX_GLOBAL_ROOT_SLOTS`                    | a corrupt count, not a real image                       |
/// | `base` not 8-byte aligned                          | a compaction writes each slot back as one 8-byte word    |
/// | `base + 8·count` wraps the address space           | the range cannot be real memory                         |
/// | the block overlaps a different registered block   | a buggy caller; overlapping roots would be read twice and are never what an image means |
///
/// # Safety
///
/// `[base, base + 8·count)` must stay **readable and writable** while registered
/// (every collection reads each slot; a compacting one writes a moved object's new
/// address back into it). A data section satisfies this for the image's lifetime.
pub(crate) unsafe fn register(base: *mut i64, count: i64) -> i64 {
    if base.is_null() || count <= 0 || count > MAX_GLOBAL_ROOT_SLOTS {
        return 0;
    }
    let start = base as usize;
    if !start.is_multiple_of(8) {
        return 0;
    }
    let Some(end) = (count as usize).checked_mul(8).and_then(|len| start.checked_add(len)) else {
        return 0;
    };
    let mut table = roots();
    for &(b, n) in table.iter() {
        if b == start && n == count as usize {
            return count; // the same block again: idempotent
        }
        let e = b + n * 8;
        if start < e && b < end {
            return 0; // overlaps a different block
        }
    }
    let at = table.partition_point(|&(b, _)| b < start);
    table.insert(at, (start, count as usize));
    count
}

/// Append the address of every registered global slot to `out` — the precise root
/// slots a collection passes to the heap alongside the stack walk's.
pub(crate) fn extend_slots(out: &mut Vec<usize>) {
    let table = roots();
    for &(base, n) in table.iter() {
        out.extend((0..n).map(|i| base + i * 8));
    }
}

/// The registered global slots, as a fresh vector (for the conservative entries,
/// which have no stack-walk slots of their own to extend).
pub(crate) fn slots() -> Vec<usize> {
    let mut v = Vec::new();
    extend_slots(&mut v);
    v
}

/// Total number of registered global slots.
pub(crate) fn count() -> i64 {
    roots().iter().map(|&(_, n)| n as i64).sum()
}

/// Forget every registered block. Not run by `__gc_reset` (globals, like stack
/// maps, live for the whole process); for test isolation and teardown.
pub(crate) fn reset() {
    roots().clear();
}

/// Test-only guard: forgets every registered block when dropped, so a test that
/// registers a block on its own stack or heap cannot leave the registry pointing at
/// freed memory if one of its assertions panics. Declare it **after** the block it
/// guards: locals drop in reverse order, so the registry is cleared before the
/// block goes away.
#[cfg(test)]
pub(crate) struct ResetOnDrop;

#[cfg(test)]
impl Drop for ResetOnDrop {
    fn drop(&mut self) {
        reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every collection reads the registered slots, so a test that registers a
    /// block it later drops must not overlap one that collects: take the crate's
    /// heap lock, which every collecting test holds.
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn register_lists_every_slot_and_is_idempotent() {
        let _g = lock();
        reset();
        let mut block = [0i64; 3];
        let _reset = ResetOnDrop;
        let base = block.as_mut_ptr();
        assert_eq!(unsafe { register(base, 3) }, 3);
        assert_eq!(unsafe { register(base, 3) }, 3, "same block again is accepted");
        assert_eq!(count(), 3, "and not duplicated");
        let b = base as usize;
        assert_eq!(slots(), vec![b, b + 8, b + 16]);
        reset();
        assert_eq!(count(), 0);
    }

    #[test]
    fn register_rejects_bad_ranges() {
        let _g = lock();
        reset();
        let mut block = [0i64; 4];
        let _reset = ResetOnDrop;
        let base = block.as_mut_ptr();
        unsafe {
            assert_eq!(register(std::ptr::null_mut(), 1), 0, "null");
            assert_eq!(register(base, 0), 0, "empty");
            assert_eq!(register(base, -1), 0, "negative");
            assert_eq!(register(base, MAX_GLOBAL_ROOT_SLOTS + 1), 0, "absurd count");
            let misaligned = (base as *mut u8).add(4) as *mut i64;
            assert_eq!(register(misaligned, 1), 0, "misaligned");
            let near_top = (usize::MAX & !7) as *mut i64;
            assert_eq!(register(near_top, 2), 0, "wraps");
            assert_eq!(register(base, 2), 2);
            assert_eq!(register(base.add(1), 2), 0, "overlaps the block above");
            assert_eq!(register(base.add(2), 2), 2, "the adjacent block is fine");
        }
        assert_eq!(count(), 4);
        reset();
    }
}
