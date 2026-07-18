//! # `gc-core-capi` — C ABI for `gc-core`'s flat-native heap
//!
//! This crate exposes [`gc_core::FlatHeap`] — the real-memory mark-and-sweep
//! collector — over a stable **C ABI**, compiled to a static archive
//! (`libgc_core_capi.a`).  It is the concrete realisation of LANG16's
//! `gc_runtime_<target>.a` companion: the archive a native-AOT executable links
//! so its emitted `alloc` / `field_*` / `safepoint` ops resolve to a real
//! garbage collector.  See `AOT00-T1-precise-gc.md` (§3.1, §11) and
//! `LANG16-gc-core.md`.
//!
//! It **supersedes `twig-aot/runtime/twig_gc.c`** — the same flat mark-sweep, but
//! one generic Rust collector shared by every native consumer instead of a
//! Twig-specific C fork.  It now covers the whole conservative collector: explicit
//! roots, a raw region ([`__gc_collect_region`]), and the argument-less
//! conservative C-stack scan ([`__gc_collect`], see [`stack_scan`]) — all pure
//! Rust.  (Wiring `twig-aot` to link this archive and retiring `twig_gc.c` is the
//! next PR.)
//!
//! ## The exported ABI
//!
//! | Symbol | Meaning |
//! |---|---|
//! | `__gc_alloc(n)` | allocate `n` zeroed bytes; returns a real pointer (as `int64`), or `0` on failure/`n<=0` |
//! | `__gc_alloc_kind(n, kind)` | as above, tagging the object with a `HeapKind` id (for later precise tracing) |
//! | `__gc_collect_roots(roots, count)` | mark from `count` root words at `roots`, sweep; returns objects freed |
//! | `__gc_collect_region(base, len)` | mark from every candidate pointer in a raw region, sweep; returns objects freed |
//! | `__gc_collect()` | conservative collect rooted at this thread's live stack + callee-saved registers; returns objects freed |
//! | `__gc_safepoint()` | paced collect — runs `__gc_collect` only when live bytes reach the adaptive threshold; returns objects freed |
//! | `__gc_live_bytes()` | live payload bytes |
//! | `__gc_collection_count()` | collections run so far |
//! | `__gc_reset()` | drop the whole heap (frees everything); mainly for tests / process teardown |
//! | `__gc_register_stackmap(...)` | register a function's stack maps (code range + per-safepoint records) for precise-root resolution |
//! | `__gc_stackmap_count()` | number of functions registered |
//! | `__gc_stackmap_reset()` | drop all registered stack maps (tests / teardown) |
//!
//! ## State & threading
//!
//! The collector is a single process-wide instance behind a `Mutex` (the native
//! AOT runtime is single-threaded, matching `twig_gc.c`; the mutex simply makes
//! the `static` sound in Rust's model and costs nothing uncontended).  Pointers
//! returned by `__gc_alloc` stay valid until a `__gc_collect_roots` that does not
//! root them, or `__gc_reset`.

use gc_core::FlatHeap;
use std::sync::Mutex;

/// Conservative C-stack scan — the argument-less `__gc_collect` that roots from
/// this thread's live stack + callee-saved registers (the drop-in for
/// `twig_gc.c`'s `__twig_gc_collect`). See [`stack_scan`].
mod stack_scan;

/// `__twig_gc_*` ABI aliases so the AOT-emitted code and `dynval_runtime.c`,
/// which reference the names `twig_gc.c` used, link against this collector.
/// See [`twig_compat`].
mod twig_compat;

/// Code-address → live-reference stack-map registry — the lookup the precise
/// stack walker consults to turn a return address into the [`gc_core::StackMapRecord`]
/// live there. Populated by [`__gc_register_stackmap`]. See [`stackmap_registry`].
mod stackmap_registry;

/// The one process-wide heap.  `None` until the first allocation (lazy init);
/// `__gc_reset` puts it back to `None`, running `FlatHeap`'s `Drop` to free
/// every outstanding block.
static HEAP: Mutex<Option<FlatHeap>> = Mutex::new(None);

/// Serialises tests that touch the single process-wide `HEAP`.
///
/// `cargo test` runs unit tests on parallel threads; two tests that each
/// `__gc_reset` and mutate `HEAP` would otherwise interleave nondeterministically.
/// Every `HEAP`-touching test — here and in [`stack_scan`] — takes this lock first,
/// making them mutually exclusive regardless of the runner's thread count. A
/// poisoned lock is recovered (a panicking test must not wedge the rest).
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` against the (lazily created) global heap.
///
/// A poisoned lock (a prior panic while holding it) is recovered with
/// `into_inner` rather than propagated — a GC must not become unusable because
/// some unrelated code panicked; the heap's own invariants are upheld by its
/// methods, not by panic-freedom of callers.
fn with_heap<R>(f: impl FnOnce(&mut FlatHeap) -> R) -> R {
    let mut guard = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    let heap = guard.get_or_insert_with(FlatHeap::new);
    f(heap)
}

/// Allocate `n` zeroed bytes on the GC heap; returns the payload pointer as an
/// `int64`, or `0` on `n <= 0`, overflow, or allocator failure.
#[no_mangle]
pub extern "C" fn __gc_alloc(n: i64) -> i64 {
    __gc_alloc_kind(n, 0)
}

/// As [`__gc_alloc`], tagging the object with `HeapKind` id `kind` (stored for
/// later precise interior tracing; `0` = opaque/conservative).
#[no_mangle]
pub extern "C" fn __gc_alloc_kind(n: i64, kind: u16) -> i64 {
    if n <= 0 {
        return 0;
    }
    // Paced collection: if the live set has reached the adaptive threshold, run a
    // conservative stack-scan collect BEFORE allocating — exactly where
    // `twig_gc.c`'s `__twig_gc_alloc` collects. Doing it *before* the allocation
    // means the new object does not exist yet, so it cannot be wrongly reclaimed;
    // every root that must survive is the caller's, already live on the stack this
    // scan walks. Below the 1 MiB threshold (host tests, light workloads) this is
    // never taken, so allocation stays a plain bump with no surprise scan.
    if with_heap(|h| h.should_collect()) {
        // SAFETY: `__gc_alloc*` is only ever called by the single-threaded native
        // runtime on a thread that owns its stack — `__gc_collect`'s contract.
        unsafe { stack_scan::__gc_collect() };
    }
    with_heap(|h| h.alloc(n as usize, kind) as i64)
}

/// Register a **reference-field map** for one class of object and return the
/// `kind` id (≥ 1) to pass to [`__gc_alloc_kind`]. Objects allocated with that id
/// are traced **precisely** — only the `count` byte offsets at `field_offsets`
/// (the object's `ref`-typed fields) are followed during marking, instead of
/// scanning every payload word conservatively.
///
/// This is the C-ABI seam a native runtime / language frontend uses to teach the
/// collector its object layouts (records, tuples, Ruby/Python/JS objects) so a
/// look-alike-pointer integer in a non-reference field can't pin a phantom child.
/// Registering nothing (`field_offsets` null or `count <= 0`) yields a kind whose
/// objects have no ref fields — traced as fully opaque.
///
/// # Safety
///
/// `field_offsets` must point to `count` readable `int64` words (or be null with
/// `count <= 0`). Negative offsets are ignored. Standard C-array contract.
#[no_mangle]
pub unsafe extern "C" fn __gc_register_kind(field_offsets: *const i64, count: i64) -> i64 {
    let offsets: Vec<usize> = if field_offsets.is_null() || count <= 0 {
        Vec::new()
    } else {
        // SAFETY: caller guarantees `field_offsets` covers `count` readable words.
        let slice = std::slice::from_raw_parts(field_offsets, count as usize);
        slice.iter().filter(|&&o| o >= 0).map(|&o| o as usize).collect()
    };
    with_heap(|h| h.register_kind(&offsets) as i64)
}

/// **Generational write barrier.** The native runtime calls this whenever it
/// stores a heap reference `child` into a field of heap object `parent` (both
/// payload addresses). If `parent` is **old**, it is recorded so a later
/// [`__gc_collect_minor`] scans it for the young objects it now references. O(1)
/// (see [`gc_core::FlatHeap::write_barrier`]); `child` is never dereferenced.
///
/// # Safety
///
/// `parent` must be a live GC-object payload on this heap (the store target
/// always is). A `parent < 32` (null / tiny) is ignored.
#[no_mangle]
pub unsafe extern "C" fn __gc_write_barrier(parent: i64, child: i64) {
    // SAFETY: `parent`/`child` are non-negative payload addresses per the contract.
    with_heap(|h| unsafe { h.write_barrier(parent as usize, child as usize) });
}

/// Mark from the `count` root words at `roots`, then sweep.  Returns the number
/// of objects reclaimed.  A null `roots` or `count <= 0` means "no roots" — a
/// full collection that frees everything not otherwise reachable (here, since
/// tracing starts only from `roots`, that is the whole heap).
///
/// # Safety
///
/// `roots` must point to `count` readable `int64` words (or be null with
/// `count <= 0`).  This is the standard C-array contract; the generated caller
/// (or a test) upholds it.
#[no_mangle]
pub unsafe extern "C" fn __gc_collect_roots(roots: *const i64, count: i64) -> i64 {
    let root_words: Vec<usize> = if roots.is_null() || count <= 0 {
        Vec::new()
    } else {
        // SAFETY: caller guarantees `roots` covers `count` readable i64 words.
        let slice = std::slice::from_raw_parts(roots, count as usize);
        slice.iter().map(|&w| w as usize).collect()
    };
    with_heap(|h| h.collect(&root_words).freed as i64)
}

/// Mark from every candidate pointer in the memory region `[base, base + len)`,
/// then sweep. Returns the number of objects reclaimed.
///
/// This is the region-scan primitive a native runtime uses to root from memory the
/// collector must scan itself — a block of spilled callee-saved registers, or the
/// machine call stack from the current stack pointer to the thread's stack base.
/// The argument-less native collect/safepoint entry points (a follow-up) discover
/// that stack range and hand it here.
///
/// # Safety
///
/// `base` must point to `len` readable bytes (or be null with `len == 0`). The
/// generated caller (or a test) upholds this.
#[no_mangle]
pub unsafe extern "C" fn __gc_collect_region(base: *const u8, len: i64) -> i64 {
    if base.is_null() || len <= 0 {
        // No region → no roots → free everything (matches `collect(&[])`).
        return with_heap(|h| h.collect(&[]).freed as i64);
    }
    // SAFETY: caller guarantees `[base, base+len)` is readable for `len` bytes.
    with_heap(|h| unsafe { h.collect_region(base, len as usize) }.freed as i64)
}

/// Current live payload bytes.
#[no_mangle]
pub extern "C" fn __gc_live_bytes() -> i64 {
    with_heap(|h| h.live_bytes() as i64)
}

/// Total collections run since process start (or last `__gc_reset`).
#[no_mangle]
pub extern "C" fn __gc_collection_count() -> i64 {
    with_heap(|h| h.collection_count() as i64)
}

/// Drop the entire heap, freeing every outstanding block, and reset counters.
/// Primarily for tests and deterministic process teardown.
#[no_mangle]
pub extern "C" fn __gc_reset() {
    let mut guard = HEAP.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None; // FlatHeap::drop frees all blocks
}

/// Register one compiled function's **stack maps** so the precise stack walker can
/// turn a return address inside it into the live-reference slots at that PC.
/// Returns the number of records stored (`> 0`), or `0` if rejected (see
/// [`stackmap_registry::register`] for the rejection rules).
///
/// A code generator calls this once per function at image start-up — before any
/// collection — passing the function's code range (`func_start`, `func_len`) and
/// its `num_records` safepoint records as **parallel flattened arrays**:
/// `pc_offsets[i]`, `frame_sizes[i]`, `callee_masks[i]`, `slot_counts[i]`, and one
/// concatenated `slots_flat` read record-by-record through the counts.
/// `frame_sizes` and `callee_masks` may be null (read as zero) for a first-cut
/// backend that spills every live reference to the stack.
///
/// This is the code-address analogue of [`__gc_register_kind`] (which registers an
/// object *layout*); together they are the two maps a precise collector needs — one
/// for stack roots, one for heap-object interiors.
///
/// # Safety
///
/// `pc_offsets` and `slot_counts` must each point to `num_records` readable words;
/// `frame_sizes` / `callee_masks` likewise or null; `slots_flat` must cover the sum
/// of the non-negative `slot_counts` (or be null if that sum is `0`). Standard
/// C-array contract, upheld by the generated caller.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn __gc_register_stackmap(
    func_start: u64,
    func_len: u64,
    num_records: i64,
    pc_offsets: *const u32,
    frame_sizes: *const u32,
    callee_masks: *const u16,
    slot_counts: *const i32,
    slots_flat: *const i32,
) -> i64 {
    // SAFETY: the array contract is forwarded verbatim to `register`.
    stackmap_registry::register(
        func_start,
        func_len,
        num_records,
        pc_offsets,
        frame_sizes,
        callee_masks,
        slot_counts,
        slots_flat,
    )
}

/// Number of functions currently registered via [`__gc_register_stackmap`].
/// Introspection for diagnostics and tests.
#[no_mangle]
pub extern "C" fn __gc_stackmap_count() -> i64 {
    stackmap_registry::count()
}

/// Drop every registered function's stack maps. Code maps normally live for the
/// whole process, so this is **not** run by [`__gc_reset`]; it exists for
/// deterministic test isolation and process teardown.
#[no_mangle]
pub extern "C" fn __gc_stackmap_reset() {
    stackmap_registry::reset()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exported functions share one process-wide `HEAP`, so the whole ABI
    // flow lives in a SINGLE test — cargo runs tests in parallel threads, and
    // interleaving two tests over the shared heap would be nondeterministic.
    #[test]
    fn c_abi_alloc_collect_flow() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        assert_eq!(__gc_live_bytes(), 0);
        assert_eq!(__gc_collection_count(), 0);

        // Allocate two real, distinct, writable blocks.
        let a = __gc_alloc(16);
        let b = __gc_alloc(16);
        assert!(a != 0 && b != 0 && a != b);
        assert_eq!(__gc_live_bytes(), 32);
        // The pointers are real memory: write and read back.
        unsafe {
            *(a as *mut i64) = 0x1122_3344_5566_7788u64 as i64;
            *(b as *mut i64) = -7;
            assert_eq!(*(a as *mut i64), 0x1122_3344_5566_7788u64 as i64);
            assert_eq!(*(b as *mut i64), -7);
        }

        // Collect rooting only `a`: `b` is reclaimed.
        let roots = [a];
        let freed = unsafe { __gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed, 1);
        assert_eq!(__gc_live_bytes(), 16);
        assert_eq!(__gc_collection_count(), 1);
        // `a` is still valid memory.
        unsafe {
            assert_eq!(*(a as *mut i64), 0x1122_3344_5566_7788u64 as i64);
        }

        // __gc_collect_region: root from a raw memory region (as a native runtime
        // would scan a register block / stack slice). The region names the two live
        // objects `a` and `c`; `d` has no candidate word in it, so only `d` is
        // reclaimed. (The region must name *every* live object — a real stack scan
        // sees all of them; here we list them explicitly.)
        let c = __gc_alloc(16);
        let d = __gc_alloc(16);
        assert_eq!(__gc_live_bytes(), 48); // a(16) + c(16) + d(16)
        let region: [i64; 4] = [0x1234, a, c, 99];
        let freed_r = unsafe {
            __gc_collect_region(
                region.as_ptr() as *const u8,
                std::mem::size_of_val(&region) as i64,
            )
        };
        assert_eq!(freed_r, 1, "d (no candidate in region) is freed");
        assert_eq!(__gc_live_bytes(), 32); // a + c survive
        let _ = d;

        // Collect with no roots frees the rest (a and c).
        let freed2 = unsafe { __gc_collect_roots(std::ptr::null(), 0) };
        assert_eq!(freed2, 2);
        assert_eq!(__gc_live_bytes(), 0);
        assert_eq!(__gc_collection_count(), 3);

        // Degenerate allocs fail to null.
        assert_eq!(__gc_alloc(0), 0);
        assert_eq!(__gc_alloc(-5), 0);

        // Reset drops everything and zeroes counters.
        __gc_alloc(64);
        assert_eq!(__gc_live_bytes(), 64);
        __gc_reset();
        assert_eq!(__gc_live_bytes(), 0);
        assert_eq!(__gc_collection_count(), 0);
    }

    /// `__gc_register_kind` + `__gc_alloc_kind` give **precise** interior tracing
    /// through the C ABI: a heap pointer in a non-reference field of a typed
    /// object is not followed, so its pointee is reclaimed.
    #[test]
    fn c_abi_register_kind_precise_trace() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();

        // A kind whose only ref field is at byte offset 0.
        let offsets = [0i64];
        let rec = unsafe { __gc_register_kind(offsets.as_ptr(), 1) };
        assert!(rec >= 1, "kind ids are 1-based");

        let target = __gc_alloc(16); // opaque pointee
        let container = __gc_alloc_kind(16, rec as u16);
        assert!(target != 0 && container != 0);
        unsafe {
            *(container as *mut i64) = 0; // field@0 (ref) = null
            *((container as usize + 8) as *mut i64) = target; // field@8 (non-ref) = phantom
        }
        assert_eq!(__gc_live_bytes(), 32);

        // Root only the container; precise tracing follows offset 0 only.
        let roots = [container];
        let freed = unsafe { __gc_collect_roots(roots.as_ptr(), 1) };
        assert_eq!(freed, 1, "the non-ref-field pointee is reclaimed precisely");
        assert_eq!(__gc_live_bytes(), 16, "the container survives");
        // Container memory is still valid.
        assert_eq!(unsafe { *(container as *const i64) }, 0);

        __gc_reset();
    }

    /// The generational C ABI is wired end-to-end: `__gc_write_barrier` records an
    /// old→young store and `__gc_collect_minor` (stack-rooted) runs a minor cycle
    /// that retains the barrier-reachable child while reclaiming young garbage.
    #[test]
    fn c_abi_generational_barrier_and_minor() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();

        // Allocate a parent and tenure it to the old generation with a full collect.
        let parent = __gc_alloc(16);
        assert!(parent != 0);
        let roots = [parent];
        let _ = unsafe { __gc_collect_roots(roots.as_ptr(), 1) };

        // Store a fresh young child into the (now old) parent, through the barrier.
        let child = __gc_alloc(16);
        assert!(child != 0);
        unsafe {
            *(parent as *mut i64) = child; // old → young store
            __gc_write_barrier(parent, child);
        }
        // Also allocate some young garbage that nothing references.
        let _ = __gc_alloc(16);

        // A minor collect must not crash and must keep the barrier-reachable child
        // (its bytes stay live) while reclaiming the young garbage.
        let _ = unsafe { stack_scan::__gc_collect_minor() };
        assert!(
            __gc_live_bytes() >= 32,
            "parent + child survive the minor cycle"
        );
        // `child` memory is still valid.
        assert_eq!(unsafe { *(child as *const i64) }, 0);
        core::hint::black_box((parent, child));

        __gc_reset();
    }
}
