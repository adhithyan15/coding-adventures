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
}
