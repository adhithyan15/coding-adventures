//! # Conservative C-stack scan — the argument-less `__gc_collect`
//!
//! [`__gc_collect_roots`](crate) and [`__gc_collect_region`](crate) both need the
//! caller to *hand the collector its roots* — a slice of words, or a span of raw
//! memory. But the native-AOT backend emits **argument-less** collection points:
//! `__twig_gc_collect()` at an explicit collect, `__twig_gc_safepoint()` at every
//! call site. There is no root list at those points — the live references are
//! wherever the machine happens to be keeping them: some in memory on the call
//! stack, some still in callee-saved registers. This module is the piece that
//! finds *all* of them and roots from them, with **no help from the caller**.
//!
//! It is the pure-Rust replacement for `twig-aot/runtime/twig_gc.c`'s `gc_mark`
//! (the `setjmp` + stack-base scan). Same conservative technique, no C:
//!
//! 1. **Spill callee-saved registers to the stack.** A live pointer may sit only
//!    in a callee-saved register that no frame above us has spilled (the frame is
//!    still running and keeping it in-register). The ABI guarantees callee-saved
//!    registers are preserved across our call, so at the instant we run they
//!    *still* hold the mutator's values. We copy them into a stack buffer with a
//!    tiny `asm!` block — the exact job `setjmp` does by saving them into a
//!    `jmp_buf`. `asm!` without `options(nomem)` is a compiler memory barrier, so
//!    the spill is not reordered or elided.
//!
//! 2. **Read the current stack pointer.** The same `asm!` returns `sp` — the
//!    lowest address of live stack. Everything the mutator can still reach lives
//!    between `sp` and the top of the stack.
//!
//! 3. **Find the thread's stack base** (the high end, since every supported
//!    target grows the stack downward) via the platform thread API — declared
//!    here as bare `extern` bindings to the system libraries, no third-party
//!    crate: `pthread_get_stackaddr_np` (macOS), `pthread_getattr_np` +
//!    `pthread_attr_getstack` (Linux), `GetCurrentThreadStackLimits` (Windows).
//!
//! 4. **Hand `[sp, base)` to [`FlatHeap::collect_region`].** That span contains
//!    the spill buffer (it lives in this frame, above `sp`) and every caller
//!    frame, so it conservatively roots from all of them.
//!
//! ## Why this is safe to run with no explicit roots
//!
//! The failure mode to fear is a collect that *misses* a live reference and frees
//! it → use-after-free. Missing a root requires a live pointer to be neither on
//! the scanned stack span nor in a spilled register — impossible here: registers
//! are spilled in step 1 and the whole live stack is scanned in step 4. False
//! *positives* (a stack integer that looks like a pointer) only retain a dead
//! object one extra cycle — the defining, intended imprecision of a conservative
//! scan, never unsound.
//!
//! ## Supported targets
//!
//! The native-AOT columns are aarch64 (macOS) and x86_64 (Linux, Windows); the
//! crate's host tests also run on exactly those. Each is implemented precisely.
//! Any other `(arch, os)` is a hard `compile_error!` rather than a silent unsafe
//! fallback — an unsound "free everything" default is worse than not compiling.

use crate::with_heap;

// ── 1 + 2. Register spill + stack-pointer read ─────────────────────────────
//
// `spill_and_sp(buf)` stores every callee-saved register that could hold a
// managed pointer — **both integer and floating-point/SIMD** — into `buf`, then
// returns the current stack pointer. `buf` must hold at least [`SPILL_SLOTS`]
// words. The register list is ABI-specific.
//
// **Why the FP/SIMD registers too.** A managed reference is normally kept in an
// integer register or on the stack, so the callee-saved *integer* set is the
// obvious thing to spill. But a runtime that **NaN-boxes** its values keeps them
// as `f64`s, and a compiler may legitimately hold a boxed reference in a
// callee-saved FP register (`d8`–`d15` on AArch64; `xmm6`–`xmm15` on Win64) across
// a call. If such a reference were the *only* live copy at a safepoint/alloc, a
// scan that spilled only integer registers would miss it and free the object —
// a use-after-free. `twig_gc.c`'s `setjmp` saved those FP registers on exactly
// these ABIs; we match that. (System V x86-64 has **no** callee-saved xmm — all
// are caller-saved, so the mutator has already spilled any live one to the stack
// before the call, and there is nothing extra to save there.) A false positive
// from a stale FP register only retains a dead object one cycle — the intended
// conservative imprecision, never unsound.
//
// The buffer pointer and the SP output are pinned to caller-saved scratch
// registers (`x8`/`x9`, `r10`/`r11`) so the register allocator can never place
// them on top of a register we are trying to read — those scratch registers are
// not in any saved-register list below, so overwriting them loses nothing.

/// Size of the spill buffer, in words: the largest callee-saved register set
/// (integer **and** FP) across supported ABIs. Counts:
///
/// | ABI            | callee-saved integer | callee-saved FP | total words |
/// |----------------|----------------------|-----------------|-------------|
/// | AArch64 AAPCS  | x19–x28 = 10         | d8–d15 = 8      | **18**      |
/// | Win64          | rbx,rbp,rdi,rsi,r12–r15 = 8 | xmm6–xmm15 = 10 | **18** |
/// | System V x86-64| rbx,rbp,r12–r15 = 6 | none            | 6           |
///
/// 18 words (144 bytes) is the max, so that is the buffer size every
/// `spill_and_sp` below must fit into. Getting this too small is an
/// out-of-bounds stack write on **every** collect that passes tests by UB luck
/// (see the git history of this constant); ABIs that spill fewer registers leave
/// the extra slots zero (read as null candidates, ignored).
const SPILL_SLOTS: usize = 18;

// The `spill_and_sp` asm blocks write FIXED BYTE offsets (up to byte 143), but the
// buffer is sized as `SPILL_SLOTS` *words*. Those agree only when a word is 8
// bytes. On a 32-bit-pointer variant of a supported arch (e.g. aarch64 `arm64_32`,
// x86-64 `x32`) the buffer would be `18 * 4 = 72` bytes while the asm still writes
// 144 — a stack overflow. Every native-AOT target is LP64/LLP64, so this holds;
// the assert turns any future non-LP64 target into a build error, not an OOB.
const _: () = assert!(
    core::mem::size_of::<usize>() == 8,
    "spill buffer offsets assume an 8-byte usize (LP64/LLP64); a 32-bit-pointer \
     target would write past the SPILL_SLOTS-word buffer",
);

#[cfg(target_arch = "aarch64")]
#[inline(never)]
unsafe fn spill_and_sp(buf: *mut usize) -> usize {
    // AAPCS64 callee-saved registers: integer x19–x28 (10, bytes 0..80) then
    // floating-point d8–d15 (8, bytes 80..144). x29 = frame pointer, x30 = link
    // register; neither carries a mutator GC value. 18 words total = SPILL_SLOTS.
    // `stp d,d` stores a 64-bit FP pair; a NaN-boxed reference is the full 64-bit
    // value, so the low-64 `d` view captures it.
    let sp: usize;
    core::arch::asm!(
        "stp x19, x20, [x8, #0]",
        "stp x21, x22, [x8, #16]",
        "stp x23, x24, [x8, #32]",
        "stp x25, x26, [x8, #48]",
        "stp x27, x28, [x8, #64]",
        "stp d8,  d9,  [x8, #80]",
        "stp d10, d11, [x8, #96]",
        "stp d12, d13, [x8, #112]",
        "stp d14, d15, [x8, #128]",
        "mov x9, sp",
        in("x8") buf,
        out("x9") sp,
        // No options(nomem): the stores through x8 must be visible as memory
        // writes so the compiler materialises `buf` and does not reorder past it.
        options(nostack),
    );
    sp
}

#[cfg(all(target_arch = "x86_64", not(target_os = "windows")))]
#[inline(never)]
unsafe fn spill_and_sp(buf: *mut usize) -> usize {
    // System V AMD64 callee-saved integer registers: rbx, rbp, r12–r15 (6).
    let sp: usize;
    core::arch::asm!(
        "mov [r10 + 0],  rbx",
        "mov [r10 + 8],  rbp",
        "mov [r10 + 16], r12",
        "mov [r10 + 24], r13",
        "mov [r10 + 32], r14",
        "mov [r10 + 40], r15",
        "mov r11, rsp",
        in("r10") buf,
        out("r11") sp,
        options(nostack, preserves_flags),
    );
    sp
}

#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
#[inline(never)]
unsafe fn spill_and_sp(buf: *mut usize) -> usize {
    // Win64 callee-saved registers: integer rbx,rbp,rdi,rsi,r12–r15 (8, bytes
    // 0..64) then FP xmm6–xmm15 (10, bytes 64..144). `movsd` stores the low 64
    // bits of each xmm — a NaN-boxed reference is a scalar `f64` living there, so
    // the low-64 view captures it (the high lanes never hold a mutator reference).
    // 18 words total = SPILL_SLOTS.
    let sp: usize;
    core::arch::asm!(
        "mov [r10 + 0],  rbx",
        "mov [r10 + 8],  rbp",
        "mov [r10 + 16], rdi",
        "mov [r10 + 24], rsi",
        "mov [r10 + 32], r12",
        "mov [r10 + 40], r13",
        "mov [r10 + 48], r14",
        "mov [r10 + 56], r15",
        "movsd [r10 + 64],  xmm6",
        "movsd [r10 + 72],  xmm7",
        "movsd [r10 + 80],  xmm8",
        "movsd [r10 + 88],  xmm9",
        "movsd [r10 + 96],  xmm10",
        "movsd [r10 + 104], xmm11",
        "movsd [r10 + 112], xmm12",
        "movsd [r10 + 120], xmm13",
        "movsd [r10 + 128], xmm14",
        "movsd [r10 + 136], xmm15",
        "mov r11, rsp",
        in("r10") buf,
        out("r11") sp,
        options(nostack, preserves_flags),
    );
    sp
}

// ── 3. Thread stack base (the high end of the down-growing stack) ───────────

/// macOS: `pthread_get_stackaddr_np` returns the base (highest address) directly.
#[cfg(target_os = "macos")]
unsafe fn stack_base() -> usize {
    // Opaque `pthread_t` handled as a raw pointer for FFI; we never dereference it.
    extern "C" {
        fn pthread_self() -> *mut core::ffi::c_void;
        fn pthread_get_stackaddr_np(thread: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }
    pthread_get_stackaddr_np(pthread_self()) as usize
}

/// Linux: `pthread_getattr_np` fills a `pthread_attr_t`, from which
/// `pthread_attr_getstack` yields the *lowest* address + size; base = low + size.
#[cfg(target_os = "linux")]
unsafe fn stack_base() -> usize {
    // `pthread_attr_t` is an opaque fixed-size blob: 56 bytes on x86_64 glibc,
    // 64 on aarch64. An 8-word (64-byte), 16-aligned buffer covers both. We only
    // ever pass its address to the pthread calls, never interpret its contents.
    #[repr(C, align(16))]
    struct PthreadAttr([u64; 8]);

    extern "C" {
        fn pthread_self() -> usize;
        fn pthread_getattr_np(thread: usize, attr: *mut PthreadAttr) -> i32;
        fn pthread_attr_getstack(
            attr: *const PthreadAttr,
            stackaddr: *mut *mut core::ffi::c_void,
            stacksize: *mut usize,
        ) -> i32;
        fn pthread_attr_destroy(attr: *mut PthreadAttr) -> i32;
    }

    let mut attr = PthreadAttr([0; 8]);
    let mut addr: *mut core::ffi::c_void = core::ptr::null_mut();
    let mut size: usize = 0;
    if pthread_getattr_np(pthread_self(), &mut attr) != 0 {
        return 0; // caller treats 0 base as "no scan"; never frees live objects
    }
    let ok = pthread_attr_getstack(&attr, &mut addr, &mut size) == 0;
    pthread_attr_destroy(&mut attr);
    if !ok {
        return 0;
    }
    (addr as usize).wrapping_add(size)
}

/// Windows: `GetCurrentThreadStackLimits` returns [low, high]; base = high.
#[cfg(target_os = "windows")]
unsafe fn stack_base() -> usize {
    extern "system" {
        fn GetCurrentThreadStackLimits(low: *mut usize, high: *mut usize);
    }
    let mut low: usize = 0;
    let mut high: usize = 0;
    GetCurrentThreadStackLimits(&mut low, &mut high);
    high
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!(
    "gc-core-capi conservative stack scan supports only macOS, Linux, and Windows \
     (the native-AOT host targets); no unsound fallback is provided"
);

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
compile_error!(
    "gc-core-capi conservative stack scan supports only aarch64 and x86_64 \
     (the native-AOT code-generation targets)"
);

// ── 4. The exported entry point ────────────────────────────────────────────

/// Run a full conservative collection rooted at **this thread's live C stack and
/// callee-saved registers** — no caller-supplied roots.
///
/// This is the drop-in for `twig_gc.c`'s argument-less `__twig_gc_collect`: the
/// native backend calls it at explicit collects and (via the safepoint wrapper)
/// at call sites, where the only roots are whatever the machine is holding.
/// Returns the number of objects reclaimed.
///
/// `#[inline(never)]` keeps this frame — and the spill buffer inside it — *below*
/// every mutator frame, so the scanned span `[sp, base)` covers them all.
///
/// # Safety
///
/// Sound to call from any thread that owns its stack (the single-threaded native
/// runtime always does). It reads this thread's stack and the global heap; it
/// must not run while another thread mutates the same heap.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect() -> i64 {
    // Spill callee-saved registers into a stack buffer, then capture SP.
    let mut regs = [0usize; SPILL_SLOTS];
    let sp = spill_and_sp(regs.as_mut_ptr());

    let base = stack_base();

    // Every supported target grows the stack downward, so a valid scan needs
    // sp < base. If stack-base detection failed (base == 0) or anything looks
    // wrong, scan nothing — freeing on an empty root set would be a
    // use-after-free, so we bias to leaking this cycle instead.
    let freed = if base != 0 && sp < base && base - sp <= MAX_STACK_SCAN {
        with_heap(|h| h.collect_region(sp as *const u8, base - sp).freed as i64)
    } else {
        0
    };

    // Keep `regs` materialised across the scan: its address is what makes the
    // spilled register values part of `[sp, base)`. Without this the optimiser
    // could reuse the slot before `collect_region` reads it.
    core::hint::black_box(&regs);
    freed
}

/// A **paced** collect: run [`__gc_collect`] only if the heap has reached its
/// adaptive threshold ([`gc_core::FlatHeap::should_collect`]); otherwise do
/// nothing. Returns objects freed (`0` if no collection ran).
///
/// This is the drop-in for `twig_gc.c`'s `__twig_gc_safepoint`. The native
/// backend emits a `safepoint` op at loop back-edges and function entries — cheap,
/// frequent checkpoints. Collecting at *every* one would be ruinous; instead each
/// safepoint asks "are we over the threshold yet?" and collects only then, so GC
/// cost stays proportional to allocation, and a tight allocation loop can never
/// starve the collector (the twig_gc.c comment's original motivation).
///
/// # Safety
///
/// Same contract as [`__gc_collect`]: the calling thread must own its stack.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_safepoint() -> i64 {
    if with_heap(|h| h.should_collect()) {
        __gc_collect()
    } else {
        0
    }
}

/// Upper bound on how many bytes of stack a single conservative scan will walk
/// (256 MiB). A corrupt or absurd `base` (far above `sp`) would otherwise make
/// `collect_region` read hundreds of GB and appear to hang — an
/// algorithmic-complexity DoS. Real thread stacks are single-digit MiB, so this
/// ceiling never truncates a legitimate scan; it only fences off a bogus range.
const MAX_STACK_SCAN: usize = 256 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{__gc_alloc, __gc_collection_count, __gc_live_bytes, __gc_reset};
    use gc_core::FlatHeap;

    /// End-to-end: an object whose pointer is held in a **live stack local**
    /// survives an argument-less `__gc_collect` (the stack scan finds it), while
    /// an object with no live reference is reclaimed.
    ///
    /// `#[inline(never)]` on the helper guarantees `kept` really is a distinct
    /// stack slot in a frame above `__gc_collect`, not something the optimiser
    /// folded away.
    #[test]
    fn stack_scan_keeps_live_local_frees_dead() {
        // Serialise against every other HEAP-touching test (see crate::TEST_LOCK),
        // then reset so this case is order-independent within the binary.
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        run_stack_scan_case();
        __gc_reset();
    }

    /// `__gc_safepoint` is **throttled**: below the collection threshold it does
    /// nothing; at/over the threshold it runs a stack-scan collect and re-tunes.
    /// Also exercises `__gc_alloc`'s auto-collect under the same threshold.
    #[test]
    fn safepoint_throttles_then_collects_at_threshold() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        run_safepoint_case();
        __gc_reset();
    }

    #[inline(never)]
    fn run_safepoint_case() {
        use gc_core::flat_heap::INITIAL_THRESHOLD;

        // Under the threshold (a single tiny object): the safepoint is a no-op.
        let small = __gc_alloc(16);
        assert!(small != 0);
        assert_eq!(unsafe { __gc_safepoint() }, 0, "no collect below threshold");
        assert_eq!(__gc_collection_count(), 0, "throttled: no cycle ran");
        core::hint::black_box(small);

        // Push the live set to the 1 MiB threshold with one big block, held live on
        // the stack. Now the safepoint is due and must collect.
        let big = __gc_alloc(INITIAL_THRESHOLD as i64);
        assert!(big != 0);
        assert!(__gc_live_bytes() as usize >= INITIAL_THRESHOLD);
        unsafe { *(big as *mut i64) = 0xB16 };

        let _ = unsafe { __gc_safepoint() }; // over threshold → collects
        assert_eq!(__gc_collection_count(), 1, "safepoint collected at threshold");
        // `big` is stack-rooted, so it survives the conservative scan.
        assert_eq!(unsafe { *(big as *const i64) }, 0xB16, "live big block survives");
        core::hint::black_box(big);
    }

    #[inline(never)]
    fn run_stack_scan_case() {
        // A live pointer on the stack: `kept` holds a real heap address.
        let kept = __gc_alloc(16);
        assert!(kept != 0);
        // A dead object: nothing references `_dead` after this line, but its
        // returned address is deliberately dropped so no stack slot retains it.
        let _ = __gc_alloc(16);
        assert_eq!(__gc_live_bytes(), 32);

        // Write through `kept` so the compiler cannot discard the local.
        unsafe { *(kept as *mut i64) = 0x5eed };

        let freed = unsafe { __gc_collect() };

        // `kept` must survive; at least the dead object should be freed. (A
        // conservative scan may *retain* extra objects if a stray stack word
        // happens to look like a pointer, so we assert a lower bound on freeing
        // and that the live object is definitely still there.)
        assert!(freed >= 1, "the unreferenced object must be reclaimed");
        assert_eq!(
            unsafe { *(kept as *const i64) },
            0x5eed,
            "the stack-rooted object must survive and keep its value"
        );
        assert!(
            __gc_live_bytes() >= 16,
            "the live object's bytes remain accounted"
        );
        assert_eq!(__gc_collection_count(), 1);

        core::hint::black_box(kept);
    }

    /// `spill_and_sp` returns a plausible stack pointer (non-zero, and below the
    /// detected stack base on these down-growing targets) and does not crash.
    #[test]
    fn spill_and_sp_reports_a_sane_stack_pointer() {
        let mut buf = [0usize; SPILL_SLOTS];
        let sp = unsafe { spill_and_sp(buf.as_mut_ptr()) };
        assert!(sp != 0);
        let base = unsafe { stack_base() };
        if base != 0 {
            assert!(sp < base, "stack grows down: sp {sp:#x} < base {base:#x}");
            assert!(base - sp < MAX_STACK_SCAN, "current frame is near the base");
        }
        core::hint::black_box(&buf);
    }

    /// `FlatHeap` is the type the scan collects over — a direct sanity check that
    /// the module's import is the real collector, not a stray re-export.
    #[test]
    fn uses_real_flatheap() {
        let mut h = FlatHeap::new();
        let _ = h.alloc(8, 0);
        assert_eq!(h.object_count(), 1);
    }
}
