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

// ── 2b. Current frame pointer (for the precise walk) ────────────────────────
//
// `current_fp()` returns this frame's **frame pointer** (`x29` on AArch64, `rbp`
// on x86-64) — the anchor the precise stack walk unwinds from. It MUST be
// `#[inline(always)]`: inlined into the `#[inline(never)]` collect entry, it reads
// that entry's own frame pointer; as a separate call it would read its *own* frame
// and mislead the walk.
//
// **Frame-pointer dependency.** This reads whatever the register holds; it is a
// valid chain anchor only if the compiler maintains a frame pointer for the collect
// entry. That is guaranteed on `aarch64-apple-darwin` (the Apple ABI mandates the
// `x29` chain) and holds under Rust's current x86-64-host defaults, but is *not*
// enforced by this crate's build. While **no stack maps are registered** a bogus
// anchor is still safe: `build_precise_roots` then classifies every frame
// conservatively and its regions tile all of `[sp, base)`, so precise collection
// degrades to exactly `__gc_collect`. Once maps ARE registered, a valid frame
// pointer becomes load-bearing for safety (a garbage anchor whose `[fp+8]` aliased a
// stale return address into a mapped function could exclude a live span) — so the
// backend-record-emission rung must build this crate with `-Cforce-frame-pointers`
// (or rely on the aarch64 ABI guarantee). Tracked as a prerequisite of that rung.

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn current_fp() -> usize {
    let fp: usize;
    core::arch::asm!("mov {}, x29", out(reg) fp, options(nomem, nostack, preserves_flags));
    fp
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn current_fp() -> usize {
    let fp: usize;
    core::arch::asm!("mov {}, rbp", out(reg) fp, options(nomem, nostack, preserves_flags));
    fp
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

/// A **minor** (young-generation-only) collection rooted at this thread's live
/// stack + callee-saved registers — the generational analogue of [`__gc_collect`].
/// Reclaims only young garbage and never scans or frees the old generation
/// (old→young pointers are reached through the remembered set the
/// [`__gc_write_barrier`](crate::__gc_write_barrier) populates). Returns objects
/// freed. Identical stack-discovery to [`__gc_collect`]; only the underlying cycle
/// differs ([`gc_core::FlatHeap::collect_minor_region`]).
///
/// # Safety
///
/// Same contract as [`__gc_collect`]: the calling thread must own its stack, and
/// every old→young store must have gone through the write barrier.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_minor() -> i64 {
    let mut regs = [0usize; SPILL_SLOTS];
    let sp = spill_and_sp(regs.as_mut_ptr());
    let base = stack_base();
    let freed = if base != 0 && sp < base && base - sp <= MAX_STACK_SCAN {
        with_heap(|h| h.collect_minor_region(sp as *const u8, base - sp).freed as i64)
    } else {
        0
    };
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

/// A full collection rooted **precisely** at this thread's stack: the argument-less
/// entry that gives the precise-root machinery a real machine stack to walk.
///
/// Where [`__gc_collect`] scans the whole `[sp, base)` span *conservatively*, this
/// captures the current frame pointer and hands it, plus `sp`/`base`, to
/// [`crate::precise_walk::build_precise_roots`], which unwinds the frame-pointer
/// chain into **precise slots** for stack-mapped frames (registered via
/// [`__gc_register_stackmap`](crate::__gc_register_stackmap)) and **conservative
/// regions** for the rest, then collects both in one [`gc_core::FlatHeap::collect_mixed`]
/// cycle. It is to `collect_mixed` what `__gc_collect` is to `collect_region`.
///
/// Callee-saved registers are spilled and handed to the collector as an explicit
/// conservative region: a reference live *only* in a callee-saved register is named
/// by no stack map yet (that needs a per-safepoint `callee_saved_mask`, a later
/// rung), so it must be scanned, exactly as `__gc_collect` scans the spill. The
/// buffer also lives in this frame — inside the walk's `[sp, fp)` collector region —
/// so the explicit region is belt-and-suspenders against an absent/garbage frame
/// pointer.
///
/// **Precision is opportunistic and safe:** with no stack maps registered, every
/// frame resolves to a conservative region whose spans tile all of `[sp, base)`, so
/// this degrades exactly to `__gc_collect` — safe even if the captured frame pointer
/// is garbage. As backends register maps, matching frames shed their floating
/// garbage; at that point a *valid* frame pointer becomes load-bearing for safety
/// (see [`current_fp`]), so the map-emitting rung must build this crate with frame
/// pointers (guaranteed by ABI on the aarch64 primary target). An unwalkable stack
/// still degrades to a conservative scan; if the stack base cannot be established,
/// it collects **nothing** this cycle rather than risk freeing a live object — the
/// same bias-to-leak as `__gc_collect`.
///
/// # Safety
///
/// Same contract as [`__gc_collect`]: sound to call from any thread that owns its
/// stack (the single-threaded native runtime always does); it must not run while
/// another thread mutates the same heap.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_precise() -> i64 {
    // Spill callee-saved registers into a stack buffer (in this frame), then SP.
    let mut regs = [0usize; SPILL_SLOTS];
    let sp = spill_and_sp(regs.as_mut_ptr());
    // Capture this frame's frame pointer — the walk's unwind anchor. `current_fp`
    // is `#[inline(always)]`, so this reads *this* frame's fp, not a callee's.
    let fp = current_fp();
    let base = stack_base();

    // Only collect when the stack range is trustworthy. As in `__gc_collect`, a
    // failed base detection (base == 0) or an absurd span means we cannot enumerate
    // all roots — collecting then could free a live object, so we leak this cycle.
    let freed = if base != 0 && sp < base && base - sp <= MAX_STACK_SCAN {
        let mut slots: Vec<usize> = Vec::new();
        let mut regions: Vec<(*const u8, usize)> = Vec::new();
        // Walk the frame-pointer chain into precise slots + conservative regions.
        crate::precise_walk::build_precise_roots(fp, sp, base, &mut slots, &mut regions);
        // Always scan the spilled callee-saved registers, independent of the walk.
        regions.push((
            regs.as_ptr() as *const u8,
            SPILL_SLOTS * core::mem::size_of::<usize>(),
        ));
        with_heap(|h| h.collect_mixed(&slots, &regions).freed as i64)
    } else {
        0
    };

    // Keep `regs` materialised across the collect: its address is what makes the
    // spilled registers part of the scanned roots.
    core::hint::black_box(&regs);
    freed
}

/// **Begin** an incremental (bounded-pause) collection rooted precisely at this thread's
/// stack — the first of the three-call cooperative cycle (spec
/// `AOT00-T4-incremental-collector.md` §6). Captures the precise roots **once**, via the
/// *same* frame-pointer walk as [`__gc_collect_precise`] (precise slots for stack-mapped
/// frames, conservative regions for the rest, plus the spilled callee-saved registers), and
/// shades them grey. The mutator then runs *between* [`__gc_collect_incremental_step`] calls;
/// its reference stores are caught by [`__gc_write_barrier`]'s incremental shading (the
/// Dijkstra insertion barrier). With no stack maps registered the walk tiles the stack
/// conservatively, exactly as `__gc_collect_precise` degrades.
///
/// **Root-snapshot contract (spec §6).** Roots are captured here; a reference that becomes
/// reachable only *after* this call is retained iff it passed through a barriered store or is
/// still reachable from this snapshot. The driver must not pop a frame holding the sole
/// reference to a white object without that reference having been stored. The gc-core mark
/// re-reads neither the snapshotted slots nor regions (it drains only the grey worklist), so
/// the fact that they point into *this* frame is safe.
///
/// **Untrustworthy stack ⇒ no-op cycle.** If the stack range can't be trusted (base
/// undetectable, or an absurd span), no phase is entered — [`__gc_collect_incremental_step`]
/// then reports "done" immediately and [`__gc_collect_incremental_finish`] reclaims nothing,
/// so no live object is ever freed (the same bias-to-leak as `__gc_collect_precise`).
///
/// # Safety
/// Same contract as [`__gc_collect_precise`]: the calling thread owns its stack; single
/// mutator; **no other collection runs between `start` and `finish`** (enforced by gc-core's
/// mixing guard). The driver loops `step` to "done" before calling `finish`.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_incremental_start() {
    // Spill callee-saved registers into a stack buffer (in this frame), then SP.
    let mut regs = [0usize; SPILL_SLOTS];
    let sp = spill_and_sp(regs.as_mut_ptr());
    let fp = current_fp();
    let base = stack_base();

    if base != 0 && sp < base && base - sp <= MAX_STACK_SCAN {
        let mut slots: Vec<usize> = Vec::new();
        let mut regions: Vec<(*const u8, usize)> = Vec::new();
        // Walk the frame-pointer chain into precise slots + conservative regions.
        crate::precise_walk::build_precise_roots(fp, sp, base, &mut slots, &mut regions);
        // Always scan the spilled callee-saved registers (a ref live only in one is named by
        // no stack map). Valid here because `incremental_start` greys these roots NOW, while
        // `regs` and the walked frames are still live.
        regions.push((
            regs.as_ptr() as *const u8,
            SPILL_SLOTS * core::mem::size_of::<usize>(),
        ));
        with_heap(|h| h.incremental_start(&slots, &regions));
    }
    // else: untrustworthy stack → don't enter a phase (see the bias-to-leak note above).

    core::hint::black_box(&regs);
}

/// **Advance** an in-progress incremental mark by up to `budget` objects — the bounded-pause
/// primitive (spec §6). Returns `1` when marking is complete (the caller should then call
/// [`__gc_collect_incremental_finish`]), `0` if more steps remain. A negative `budget` is
/// treated as `0`. If no incremental phase is in progress (e.g. an untrustworthy-stack
/// `start`, or a spurious call), returns `1` ("done") without touching the heap — so the
/// driver's `step`-to-done loop terminates and a no-op cycle stays safe.
///
/// # Safety
/// Single-threaded; no other collection runs mid-cycle. Called only after
/// [`__gc_collect_incremental_start`].
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_incremental_step(budget: i64) -> i64 {
    let budget = if budget < 0 { 0 } else { budget as usize };
    with_heap(|h| {
        if !h.incremental_in_progress() {
            return 1; // no phase → nothing to mark → "done"
        }
        if h.incremental_step(budget) {
            1
        } else {
            0
        }
    })
}

/// **Finish** an in-progress incremental cycle: sweep the unreachable (white) objects and end
/// the phase (spec §6). Returns the number of objects reclaimed. Marking must be complete
/// (drive [`__gc_collect_incremental_step`] to `1` first). If no phase is in progress, returns
/// `0` without sweeping — so an untrustworthy-stack cycle reclaims nothing.
///
/// # Safety
/// Single-threaded; no other collection mid-cycle; called only after marking is complete.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_incremental_finish() -> i64 {
    with_heap(|h| {
        if !h.incremental_in_progress() {
            return 0; // no phase → nothing collected (no sweep)
        }
        h.incremental_finish().freed as i64
    })
}

/// A full **moving/compacting** collection rooted precisely at this thread's stack —
/// the argument-less entry that turns the precise-root machinery into a *relocating*
/// GC (spec `AOT00-T3-moving-collector.md` §5). It is to
/// [`gc_core::FlatHeap::collect_compacting`] exactly what [`__gc_collect_precise`] is to
/// `collect_mixed`: the *same* frame-pointer walk produces **precise slots** (for
/// stack-mapped frames) and **conservative regions** (unmapped frames + the spilled
/// callee-saved registers), and the very same `(slots, regions)` are handed to
/// `collect_compacting`, which evacuates the movable survivors into an arena and — this is
/// what makes moving safe — rewrites the pointers that named them, **including writing each
/// forwarded address back into its precise root slot** (a real stack location produced by
/// the walk). After it returns, the mutator's stack slots point at the relocated objects.
///
/// **Why precise roots are the safety precondition (and why this is always safe to call):**
/// only an object reachable *purely precisely* — named by a stack-map slot and by no
/// conservative region — is ever moved; anything a conservative region (an unmapped frame,
/// or a spilled register) can reach is **pinned** and stays put, because a conservative
/// holder's pointer cannot be found-and-rewritten. With **no** stack maps registered every
/// frame becomes a conservative region tiling all of `[sp, base)`, so nothing is movable and
/// this degrades to exactly [`__gc_collect_precise`] / `__gc_collect` — no relocation, no
/// risk, even if the captured frame pointer is garbage. As backends register maps a *valid*
/// frame pointer becomes load-bearing (see [`current_fp`]), identical to `__gc_collect_precise`.
/// A failed stack-base detection collects nothing this cycle (bias-to-leak), never freeing
/// or moving a live object.
///
/// Returns the number of objects reclaimed (genuinely-dead only; a relocated object is a
/// survivor, not a free — see `collect_compacting`).
///
/// # Safety
///
/// Same contract as [`__gc_collect_precise`]: sound to call from any thread that owns its
/// stack (the single-threaded native runtime always does); it must not run while another
/// thread mutates the same heap. The captured frame pointer must be valid once any stack map
/// is registered (guaranteed by ABI on the aarch64 primary target / frame-pointer builds).
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn __gc_collect_compacting() -> i64 {
    // Spill callee-saved registers into a stack buffer (in this frame), then SP.
    let mut regs = [0usize; SPILL_SLOTS];
    let sp = spill_and_sp(regs.as_mut_ptr());
    // Capture this frame's frame pointer — the walk's unwind anchor (see current_fp).
    let fp = current_fp();
    let base = stack_base();

    // Only collect when the stack range is trustworthy (identical gate to
    // __gc_collect_precise): a failed base detection or absurd span means we cannot
    // enumerate all roots, so we leak this cycle rather than move/free a live object.
    let freed = if base != 0 && sp < base && base - sp <= MAX_STACK_SCAN {
        let mut slots: Vec<usize> = Vec::new();
        let mut regions: Vec<(*const u8, usize)> = Vec::new();
        // Walk the frame-pointer chain into precise slots + conservative regions.
        crate::precise_walk::build_precise_roots(fp, sp, base, &mut slots, &mut regions);
        // Always scan the spilled callee-saved registers as a conservative region: a
        // reference live only in such a register is named by no stack map yet, so it must
        // pin (never move) — exactly as __gc_collect_precise scans it.
        regions.push((
            regs.as_ptr() as *const u8,
            SPILL_SLOTS * core::mem::size_of::<usize>(),
        ));
        with_heap(|h| h.collect_compacting(&slots, &regions).freed as i64)
    } else {
        0
    };

    // Keep `regs` materialised across the collect: its address is what makes the
    // spilled registers part of the scanned roots.
    core::hint::black_box(&regs);
    freed
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

    /// End-to-end smoke test for the argument-less `__gc_collect_precise`: an object
    /// held in a live stack local survives, a dead one is reclaimed, and the whole
    /// asm-capture → frame-pointer-walk → `collect_mixed` path runs without crashing.
    ///
    /// With no stack maps registered, every frame resolves conservatively, so this
    /// exercises the same safety guarantee as `__gc_collect` (the live local, sitting
    /// in a walked/tiled frame, is never dropped) while driving the precise plumbing.
    #[test]
    fn precise_collect_keeps_live_local_frees_dead() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        run_precise_collect_case();
        __gc_reset();
    }

    #[inline(never)]
    fn run_precise_collect_case() {
        let kept = __gc_alloc(16);
        assert!(kept != 0);
        let _ = __gc_alloc(16); // dead: no stack slot retains it
        assert_eq!(__gc_live_bytes(), 32);
        unsafe { *(kept as *mut i64) = 0x9a11 };

        let freed = unsafe { __gc_collect_precise() };

        assert!(freed >= 1, "the unreferenced object must be reclaimed");
        assert_eq!(
            unsafe { *(kept as *const i64) },
            0x9a11,
            "the stack-rooted object must survive the precise collect"
        );
        assert!(__gc_live_bytes() >= 16);
        assert_eq!(__gc_collection_count(), 1);
        core::hint::black_box(kept);
    }

    /// End-to-end smoke test for the argument-less **moving** entry
    /// `__gc_collect_compacting`: the same asm-capture → frame-pointer-walk path, now
    /// driving `collect_compacting`. With no stack maps registered every frame is
    /// conservative, so nothing is movable and this degrades to exactly
    /// `__gc_collect_precise` — the live stack local (conservatively pinned) survives with
    /// its value intact, and the dead object is reclaimed. Proves the C-ABI moving path runs
    /// on a real thread stack without crashing and never corrupts a live object.
    #[test]
    fn compacting_collect_keeps_live_local_frees_dead() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        run_compacting_collect_case();
        __gc_reset();
    }

    #[inline(never)]
    fn run_compacting_collect_case() {
        let kept = __gc_alloc(16);
        assert!(kept != 0);
        let _ = __gc_alloc(16); // dead: no stack slot retains it
        assert_eq!(__gc_live_bytes(), 32);
        unsafe { *(kept as *mut i64) = 0x5eed };

        let freed = unsafe { __gc_collect_compacting() };

        assert!(freed >= 1, "the unreferenced object must be reclaimed");
        assert_eq!(
            unsafe { *(kept as *const i64) },
            0x5eed,
            "the conservatively-pinned live local survives the compacting collect intact"
        );
        assert!(__gc_live_bytes() >= 16);
        assert_eq!(__gc_collection_count(), 1);
        core::hint::black_box(kept);
    }

    /// End-to-end smoke test for the **incremental** C-ABI cycle
    /// `__gc_collect_incremental_{start,step,finish}`: an object held in a live stack local
    /// survives the interruptible collection; a dead one is reclaimed. Drives the real
    /// three-call protocol (start → step-to-done → finish) on this thread's stack, with a
    /// deliberately small budget so the mark takes several steps. With no stack maps
    /// registered every frame is conservative, so the live local is pinned (kept) and the
    /// unreferenced object is swept — exactly matching a stop-the-world collect, just in
    /// bounded slices.
    #[test]
    fn incremental_collect_keeps_live_local_frees_dead() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        run_incremental_collect_case();
        __gc_reset();
    }

    #[inline(never)]
    fn run_incremental_collect_case() {
        let kept = __gc_alloc(16);
        assert!(kept != 0);
        let _ = __gc_alloc(16); // dead: no stack slot retains it
        assert_eq!(__gc_live_bytes(), 32);
        unsafe { *(kept as *mut i64) = 0x1ce5 };

        // start → step (budget 1, so several slices) to done → finish.
        unsafe { __gc_collect_incremental_start() };
        let mut steps = 0;
        while unsafe { __gc_collect_incremental_step(1) } == 0 {
            steps += 1;
            assert!(steps < 100_000, "incremental mark must converge");
        }
        let freed = unsafe { __gc_collect_incremental_finish() };

        assert!(freed >= 1, "the unreferenced object must be reclaimed");
        assert_eq!(
            unsafe { *(kept as *const i64) },
            0x1ce5,
            "the conservatively-pinned live local survives the incremental collect intact"
        );
        assert!(__gc_live_bytes() >= 16);
        assert_eq!(__gc_collection_count(), 1);
        core::hint::black_box(kept);
    }

    /// The incremental C-ABI protocol is safe even if no phase is in progress: `step` reports
    /// "done" and `finish` reclaims nothing (the no-op cycle an untrustworthy-stack `start`
    /// produces). Nothing is swept, so no live object could be lost.
    #[test]
    fn incremental_step_finish_are_safe_with_no_phase() {
        let _guard = crate::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        __gc_reset();
        let kept = __gc_alloc(16);
        assert!(kept != 0);
        // No `start` was called → no phase in progress.
        assert_eq!(unsafe { __gc_collect_incremental_step(64) }, 1, "no phase ⇒ done");
        assert_eq!(unsafe { __gc_collect_incremental_finish() }, 0, "no phase ⇒ nothing freed");
        assert!(__gc_live_bytes() >= 16, "the object was NOT swept without a real cycle");
        core::hint::black_box(kept);
        __gc_reset();
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
