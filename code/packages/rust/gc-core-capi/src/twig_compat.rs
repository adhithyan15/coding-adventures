//! # Twig-compat ABI aliases (`__twig_gc_*`)
//!
//! The native-AOT toolchain was built against `twig-aot/runtime/twig_gc.c`, whose
//! collector exported the symbols `__twig_gc_alloc`, `__twig_gc_collect`,
//! `__twig_gc_safepoint`, `__twig_gc_live_bytes`, and `__twig_gc_collection_count`.
//! Those names are **baked into the code generators**: the aarch64 backend emits
//! calls to `__twig_gc_alloc` and `__twig_gc_safepoint`, the LLVM backend
//! references them too, and the C runtime `dynval_runtime.c` calls
//! `__twig_gc_alloc` directly.
//!
//! When `twig_gc.c` is retired and this crate's `libgc_core_capi.a` provides the
//! collector instead, those references must still resolve — otherwise every AOT
//! link fails with undefined symbols. This module re-exports the generic `__gc_*`
//! ABI under the `__twig_gc_*` names the emitted code expects. Each alias is a
//! thin `#[no_mangle]` wrapper that forwards to the real entry point — pure Rust,
//! no C shim, so the collector stays one generic implementation.
//!
//! ## Signature fidelity
//!
//! The aliases match `twig_gc.c`'s prototypes exactly, including the two that
//! return `void` there — `__twig_gc_collect` / `__twig_gc_safepoint`. The generic
//! `__gc_collect` / `__gc_safepoint` return the freed-object count (`i64`); the
//! void aliases simply discard it, so a caller that declared the twig prototype
//! (`void (*)(void)`) links and calls correctly.
//!
//! Once the code generators are migrated to emit the `__gc_*` names directly,
//! this shim can be deleted.

use crate::stack_scan::{
    __gc_collect, __gc_collect_compacting, __gc_collect_incremental_finish,
    __gc_collect_incremental_start, __gc_collect_incremental_step, __gc_collect_precise,
    __gc_safepoint,
};
use crate::{
    __gc_alloc, __gc_alloc_kind, __gc_collection_count, __gc_live_bytes, __gc_register_kind,
    __gc_register_ref_array_kind,
};
use core::sync::atomic::{AtomicI64, Ordering};

/// `__twig_gc_alloc(n)` → [`__gc_alloc`]. Called by the emitted code and by
/// `dynval_runtime.c` for every heap allocation. Allocates `n` bytes under
/// **kind 0** — the collector traces those bytes *conservatively* (any word that
/// looks like a heap pointer pins its target) and therefore never relocates the
/// block. Correct and safe, but not movable. For the fixed 2-word record/pair
/// cell whose layout the frontend knows precisely, prefer
/// [`__twig_gc_alloc_pair`].
#[no_mangle]
pub extern "C" fn __twig_gc_alloc(n: i64) -> i64 {
    __gc_alloc(n)
}

/// `__twig_gc_alloc_pair()` → a 16-byte, 2-word heap cell registered under the
/// **movable `{0,8}` pair kind** (both words are reference slots), the same
/// layout `dynval_runtime.c`'s `__dyn_cons` uses for a cons cell.
///
/// The native code generators emit this — instead of the kind-0 `__twig_gc_alloc`
/// — for the record/union constructor `alloc` op, whose cell is always a
/// two-field pair of **boxed `any` values** (the constructor's parameters are
/// typed `any`, so its stored fields are tagged: an immediate int/nil/bool or a
/// heap reference — never a raw look-alike integer). That makes the precise
/// `{0,8}` layout *sound*: the collector traces exactly the two reference slots
/// and, under compaction, **relocates** the cell and fixes up every reference to
/// it, instead of pinning it as kind 0 would. A record is thereby a first-class
/// movable heap object, on par with cons cells, closures, and arrays.
///
/// Unlike `__dyn_cons`, this only *allocates* (returns a **raw**, untagged,
/// 16-aligned pointer); the caller's `field_store`s write the two fields. The
/// payload arrives zeroed, so a collection between the alloc and the stores
/// traces two null slots — safe.
///
/// A **union** constructor cell uses the same op with a synthesized integer
/// *discriminant* in word 0 — not a boxed `any`. That is still sound: gc-core is
/// **provenance-filtered**, tracing/relocating a `{0,8}` slot only when its value
/// resolves to a real live block (`find_header`) / is a key in the compaction
/// forwarding map. A small discriminant never is, so it is never followed or
/// rewritten regardless of its low bits (proven by the twig-aot smoke test
/// `end_to_end_gc_raw_discriminant_word_not_relocated`, which relocates a cell whose
/// word 0 is the heap-tag-looking `0b10111`).
///
/// The pair kind is registered once and memoized in `PAIR_KIND`. The AOT runtime
/// is single-threaded, so the relaxed load/store carries no ordering hazard; a
/// (theoretical) racing double-init would only register the identical `{0,8}`
/// layout twice, and either id is a correct movable pair kind.
#[no_mangle]
pub extern "C" fn __twig_gc_alloc_pair() -> i64 {
    static PAIR_KIND: AtomicI64 = AtomicI64::new(0); // 0 = not yet registered
    let mut kind = PAIR_KIND.load(Ordering::Relaxed);
    if kind == 0 {
        let offsets: [i64; 2] = [0, 8];
        // SAFETY: `offsets` points to exactly `2` readable `i64` words.
        kind = unsafe { __gc_register_kind(offsets.as_ptr(), 2) };
        PAIR_KIND.store(kind, Ordering::Relaxed);
    }
    __gc_alloc_kind(2 * core::mem::size_of::<i64>() as i64, kind as u16)
}

/// `__twig_gc_register_ref_array_kind(fixed, fixed_count, tail_from)` →
/// [`__gc_register_ref_array_kind`]. The `__twig_gc_*` linker name a native code generator
/// emits for the `gc_register_ref_array_kind` builtin — the seam a language frontend's **array**
/// type calls to declare its layout (fixed ref fields + a variable reference tail), so the
/// collector traces and, under compaction, **relocates** the array and its elements precisely
/// instead of pinning them. Registers a kind; performs no allocation or stack scan.
///
/// # Safety
///
/// Same contract as [`__gc_register_ref_array_kind`]: `fixed` must point to `fixed_count`
/// readable `int64` words (or be null with `fixed_count <= 0`).
#[no_mangle]
pub unsafe extern "C" fn __twig_gc_register_ref_array_kind(
    fixed: *const i64,
    fixed_count: i64,
    tail_from: i64,
) -> i64 {
    __gc_register_ref_array_kind(fixed, fixed_count, tail_from)
}

/// `__twig_gc_collect()` → [`__gc_collect`] (return value discarded to match the
/// original `void` prototype). A full conservative stack-scan collection.
///
/// # Safety
///
/// Same contract as [`__gc_collect`]: the calling thread must own its stack.
/// `#[inline(never)]` keeps this a real frame below the mutator so the scan
/// covers the caller.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect() {
    let _ = __gc_collect();
}

/// `__twig_gc_safepoint()` → [`__gc_safepoint`] (return value discarded to match
/// the original `void` prototype). A throttled collect: runs only past the
/// adaptive threshold.
///
/// # Safety
///
/// Same contract as [`__gc_safepoint`]: the calling thread must own its stack.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_safepoint() {
    let _ = __gc_safepoint();
}

/// `__twig_gc_collect_precise()` → [`__gc_collect_precise`]. A full collection
/// rooted **precisely** at the caller's stack via the frame-pointer walk (mapped
/// frames contribute exact reference slots; the rest are conservative). Returns the
/// freed-object count. This is the `__twig_gc_*` name the native code generators
/// emit for a precise collect (`gc_collect_precise` builtin), the AOT00-T1
/// increment-C entry point that makes precise roots observable end to end.
///
/// # Safety
///
/// Same contract as [`__gc_collect_precise`]: the calling thread must own its stack.
/// `#[inline(never)]` keeps it a real frame below the mutator so the walk starts at
/// the caller.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect_precise() -> i64 {
    __gc_collect_precise()
}

/// `__twig_gc_collect_compacting()` → [`__gc_collect_compacting`]. A full **moving**
/// collection rooted precisely at the caller's stack (spec AOT00-T3 §5): the movable
/// survivors are evacuated into an arena and every pointer that named them — including the
/// caller's precise root slots — is rewritten to the new location. Returns the
/// freed-object count. This is the `__twig_gc_*` name a native code generator emits for the
/// `gc_collect_compacting` builtin, letting a compiled program trigger a compaction. With
/// no stack maps registered (or only conservatively-reachable objects, as today's kind-0
/// frontend allocations) nothing is movable and it degrades to exactly
/// [`__gc_collect_precise`], so it is always safe to call.
///
/// # Safety
///
/// Same contract as [`__gc_collect_compacting`]: the calling thread must own its stack.
/// `#[inline(never)]` keeps it a real frame below the mutator so the walk starts at the
/// caller.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect_compacting() -> i64 {
    __gc_collect_compacting()
}

/// `__twig_gc_collect_incremental_start()` → [`__gc_collect_incremental_start`]. Begins a
/// bounded-pause incremental collection rooted precisely at the caller's stack (spec
/// AOT00-T4 §6). The `__twig_gc_*` name a native code generator emits for the
/// `gc_collect_incremental_start` builtin. `#[inline(never)]` keeps it a real frame below the
/// mutator so the root walk starts at the caller.
///
/// # Safety
/// Same contract as [`__gc_collect_incremental_start`]: the calling thread owns its stack.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect_incremental_start() {
    __gc_collect_incremental_start()
}

/// `__twig_gc_collect_incremental_step(budget)` → [`__gc_collect_incremental_step`]. Advances
/// the in-progress incremental mark by up to `budget` objects; returns `1` when marking is
/// complete, `0` otherwise.
///
/// # Safety
/// Same contract as [`__gc_collect_incremental_step`]: called between a `start` and its
/// `finish`; single mutator.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect_incremental_step(budget: i64) -> i64 {
    __gc_collect_incremental_step(budget)
}

/// `__twig_gc_collect_incremental_finish()` → [`__gc_collect_incremental_finish`]. Sweeps the
/// unreachable objects and ends the incremental cycle; returns the count reclaimed.
///
/// # Safety
/// Same contract as [`__gc_collect_incremental_finish`]: called after marking is complete.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __twig_gc_collect_incremental_finish() -> i64 {
    __gc_collect_incremental_finish()
}

/// `__twig_gc_live_bytes()` → [`__gc_live_bytes`].
#[no_mangle]
pub extern "C" fn __twig_gc_live_bytes() -> i64 {
    __gc_live_bytes()
}

/// `__twig_gc_collection_count()` → [`__gc_collection_count`].
#[no_mangle]
pub extern "C" fn __twig_gc_collection_count() -> i64 {
    __gc_collection_count()
}

/// `__twig_gc_stackmap_count()` → [`crate::__gc_stackmap_count`]. Number of functions
/// whose stack maps are currently registered — a diagnostic the native `gc_stackmap_count`
/// builtin exposes so a program can confirm `__gc_init_stackmaps` ran.
#[no_mangle]
pub extern "C" fn __twig_gc_stackmap_count() -> i64 {
    crate::__gc_stackmap_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::__gc_reset;

    /// The twig-named aliases drive the same underlying collector as the `__gc_*`
    /// ABI: alloc through `__twig_gc_alloc`, observe it via both accessors,
    /// collect through the throttled + full entry points.
    #[test]
    fn twig_aliases_forward_to_gc_core() {
        // Serialise against the other HEAP-touching tests (shared process-wide heap).
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();

        // Allocate through the twig alias; it must return a real, writable pointer
        // and be reflected in both twig-named accessors.
        let p = __twig_gc_alloc(16);
        assert!(p != 0);
        unsafe { *(p as *mut i64) = 0x7161 };
        assert_eq!(__twig_gc_live_bytes(), 16);
        assert_eq!(__twig_gc_collection_count(), 0);

        // Below the 1 MiB threshold, the safepoint is a no-op (throttled).
        unsafe { __twig_gc_safepoint() };
        assert_eq!(__twig_gc_collection_count(), 0);

        // A full collect runs a cycle; `p` is stack-rooted so it survives.
        unsafe { __twig_gc_collect() };
        assert_eq!(__twig_gc_collection_count(), 1);
        assert_eq!(unsafe { *(p as *const i64) }, 0x7161);
        core::hint::black_box(p);

        __gc_reset();
    }

    /// `__twig_gc_alloc_pair` returns a movable 2-word pair cell: an object it
    /// references via a `{0,8}` slot is retained through it and reclaimed once the
    /// slot is cleared — proving the two fields are traced precisely (a kind-0
    /// block would pin conservatively, not trace).
    #[test]
    fn twig_alloc_pair_is_a_traced_movable_pair() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();

        // A pair cell, plus a child object referenced only from the pair's field 0.
        let pair = __twig_gc_alloc_pair();
        let child = __twig_gc_alloc(16);
        assert!(pair != 0 && child != 0);
        // The allocator returns a raw (untagged) 16-aligned pointer.
        assert_eq!(pair & 0b1111, 0, "pair payload must be 16-aligned (low bits clear)");
        unsafe {
            *(pair as *mut i64) = child; // field 0 = child (a reference)
            *((pair as usize + 8) as *mut i64) = 0; // field 1 = null
        }

        // Root only the pair: the child survives via field 0 → 32 live bytes.
        let roots = [pair];
        let freed = unsafe { crate::__gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed, 0, "child retained via the pair's traced field 0");
        assert_eq!(__twig_gc_live_bytes(), 32);

        // Clear field 0 → the child is reclaimed, proving the slot was traced.
        unsafe { *(pair as *mut i64) = 0 };
        let freed2 = unsafe { crate::__gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed2, 1, "clearing field 0 reclaims the child (the slot is traced, not pinned)");

        __gc_reset();
    }

    /// `__twig_gc_register_ref_array_kind` forwards to the ref-array kind registration: an array
    /// allocated under the returned kind has its reference **tail** traced, so an element stored
    /// in a tail slot is retained via the array and reclaimed once that slot is cleared.
    #[test]
    fn twig_register_ref_array_kind_alias_forwards() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();

        // Register a pure reference array (no fixed fields, tail from offset 0) via the twig alias.
        let kind = unsafe { __twig_gc_register_ref_array_kind(core::ptr::null(), 0, 0) };
        assert!(kind >= 1, "kind ids are 1-based");

        let elem = __twig_gc_alloc(16);
        let arr = crate::__gc_alloc_kind(16, kind as u16); // a 2-slot array of the registered kind
        assert!(elem != 0 && arr != 0);
        unsafe {
            *(arr as *mut i64) = elem; // arr[0] = elem (a reference in the tail)
            *((arr as usize + 8) as *mut i64) = 0; // arr[1] = null
        }

        // Root only the array: `elem` survives via the tail → 32 live bytes.
        let roots = [arr];
        let freed = unsafe { crate::__gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed, 0, "the element is retained via the array's reference tail");
        assert_eq!(__twig_gc_live_bytes(), 32);

        // Clear the tail slot → the element is reclaimed, proving the tail was traced.
        unsafe { *(arr as *mut i64) = 0 };
        let freed2 = unsafe { crate::__gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed2, 1, "clearing the tail slot reclaims the element");

        __gc_reset();
    }

    /// The increment-C aliases forward to the precise/observability entry points:
    /// `__twig_gc_collect_precise` runs a (stack-rooted) collection and returns a
    /// count, and `__twig_gc_stackmap_count` reports the stack-map registry size.
    #[test]
    fn twig_precise_and_count_aliases_forward() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        crate::__gc_stackmap_reset();

        // No functions registered yet → the count alias reads zero.
        assert_eq!(__twig_gc_stackmap_count(), 0);

        // A stack-rooted allocation survives a precise collect (no maps registered →
        // the walk degrades to a conservative scan, which roots `p` on this stack).
        let p = __twig_gc_alloc(16);
        assert!(p != 0);
        unsafe { *(p as *mut i64) = 0x5150 };
        let _freed = unsafe { __twig_gc_collect_precise() };
        assert_eq!(unsafe { *(p as *const i64) }, 0x5150, "precise collect kept the live root");
        core::hint::black_box(p);

        __gc_reset();
    }
}
