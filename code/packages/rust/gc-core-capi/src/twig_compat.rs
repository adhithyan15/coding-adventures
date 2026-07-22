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

use crate::stack_scan::{__gc_collect, __gc_collect_precise, __gc_safepoint};
use crate::{__gc_alloc, __gc_collection_count, __gc_live_bytes};

/// `__twig_gc_alloc(n)` → [`__gc_alloc`]. Called by the emitted code and by
/// `dynval_runtime.c` for every heap allocation.
#[no_mangle]
pub extern "C" fn __twig_gc_alloc(n: i64) -> i64 {
    __gc_alloc(n)
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
