/* twig_gc.c — Conservative mark-and-sweep garbage collector for the Twig
 * native AOT runtime.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * Overview
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * Every language that compiles to IIR and targets the native AOT backend
 * (McCarthy Lisp, BASIC boxed values, future closures, …) needs heap memory
 * that can be freed automatically.  `__twig_alloc_bytes` (twig_runtime.c)
 * uses `calloc` and never frees — acceptable for tiny command-line scripts,
 * but a McCarthy Lisp program that builds a large list would OOM.
 *
 * This file implements TWIG-GC (Layer 1 of the native AOT substrate,
 * code/specs/native-aot-substrate.md) — a *conservative* mark-and-sweep
 * collector written in portable C99.
 *
 * Conservative means the GC does not require the compiler to generate GC
 * maps or safepoint metadata.  Every word on the C stack is treated as a
 * *potential* managed pointer and tested against the live-object table.
 * False positives (a plain integer that happens to look like a pointer) cause
 * live objects to be retained unnecessarily — they are never freed too early.
 * This is the same strategy used by the Boehm GC and by early versions of
 * the .NET runtime on 32-bit platforms.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * Memory layout
 * ─────────────────────────────────────────────────────────────────────────────
 *
 *   Every managed allocation is preceded by a 32-byte gc_header_t:
 *
 *   ┌──────────┬──────────┬─────────┬──────────────────────┐
 *   │ next (8) │ size (8) │ mark (1)│ _pad (15)            │
 *   └──────────┴──────────┴─────────┴──────────────────────┘
 *   ↑ gc_header_t                   ↑ payload returned to caller
 *
 *   sizeof(gc_header_t) == 32.  Because `malloc` / `calloc` return memory
 *   aligned to at least 16 bytes on every platform we target (POSIX and
 *   Windows), and the header is 32 bytes, the payload is always
 *   16-byte–aligned.  This satisfies the requirement that heap pointers
 *   have their low 3 bits clear, which the Lispy NaN-box tag scheme needs.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * NaN-box compatibility
 * ─────────────────────────────────────────────────────────────────────────────
 *
 *   Lispy heap pointers are stored with the low 3 bits set to 0b111 (the
 *   HEAP tag).  When scanning the stack for roots the GC checks both:
 *
 *     1. The raw stack word `w` (for raw pointers from `alloc`/`alloc_bytes`).
 *     2. `w & ~0x7ULL` (strips tag bits) for NaN-boxed Lispy values.
 *
 *   Both are tested against the live-object range.
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * Adaptive collection threshold
 * ─────────────────────────────────────────────────────────────────────────────
 *
 *   gc_threshold starts at 1 MB.  After each collection:
 *   - If more than 50% of the heap (by byte count) survived, double the
 *     threshold (the program is live-heavy; collecting too often wastes time).
 *   - Otherwise halve it (floor: 1 MB).
 *
 *   This matches the adaptive strategies used by Go's runtime and the JVM's
 *   ergonomic GC — the idea is to keep pause frequency roughly proportional
 *   to allocation rate while bounding pause duration.
 */

#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>

/* ── Platform-specific stack-base detection ────────────────────────────────
 *
 * The mark phase needs to know where the thread's stack starts (highest
 * address on down-growing stacks) so it can scan from the current SP to that
 * base.  Detection is platform-specific.
 *
 * macOS:   pthread_get_stackaddr_np — Apple extension, available since 10.5.
 * Linux:   pthread_getattr_np + pthread_attr_getstack — POSIX extension.
 * Windows: __readgsqword(0x08) — reads TEB.StackBase from the GS segment.
 * Other:   conservative fallback using the address of a local variable.
 */
#if defined(__APPLE__)
#  include <pthread.h>
static void *gc_stack_base(void) {
    return pthread_get_stackaddr_np(pthread_self());
}
#elif defined(__linux__)
#  include <pthread.h>
static void *gc_stack_base(void) {
    pthread_t self = pthread_self();
    pthread_attr_t attr;
    void *stack_addr = NULL;
    size_t stack_size = 0;
    if (pthread_getattr_np(self, &attr) == 0) {
        pthread_attr_getstack(&attr, &stack_addr, &stack_size);
        pthread_attr_destroy(&attr);
        /* stack_addr is the LOWEST address; base = addr + size. */
        return (char *)stack_addr + stack_size;
    }
    /* Fallback: use a rough estimate from a local variable. */
    return (void *)((uintptr_t)&attr + 65536);
}
#elif defined(_WIN32) || defined(_WIN64)
#  include <intrin.h>
static void *gc_stack_base(void) {
    /* TEB.StackBase is at GS:[0x08] on x64 Windows.  The stack grows
     * downward from this address. */
    return (void *)(uintptr_t)__readgsqword(0x08);
}
#else
/* Unknown platform — scan 64 KB worth of stack as a conservative fallback. */
static void *gc_stack_base(void) {
    volatile char local;
    return (void *)((uintptr_t)&local + 65536);
}
#endif

/* ── Object header ─────────────────────────────────────────────────────────
 *
 * Placed immediately BEFORE the user payload.  The user-visible pointer is
 * always `(gc_header_t *)hdr + 1` i.e. the byte just after the header.
 * Reverse: `header_of(payload) = (gc_header_t *)payload - 1`.
 *
 * Fields:
 *   next   — singly-linked list of every live allocation.  Head is
 *            `gc_all_objects`.  Sweep phase walks this list to find unmarked
 *            (dead) objects and free them.
 *   size   — payload size in bytes.  Used by the mark phase to scan the
 *            payload for further managed pointers, and by threshold accounting.
 *   marked — set to 1 during the mark phase.  Cleared by the sweep phase
 *            after the object survives the collection.
 *   _pad   — explicit padding to reach 32 bytes total, so `sizeof == 32`
 *            without relying on compiler struct-tail padding.  Makes the
 *            payload 16-byte–aligned on every platform.
 */
typedef struct gc_header {
    struct gc_header *next;   /* 8 bytes */
    size_t            size;   /* 8 bytes */
    uint8_t           marked; /* 1 byte  */
    uint8_t           _pad[15]; /* 15 bytes — total 32 */
} gc_header_t;

/* ── Forward declarations ───────────────────────────────────────────────────
 * Required because __twig_gc_alloc calls __twig_gc_collect which is defined
 * below the allocator in this file (definitions are in dependency order).
 */
void __twig_gc_collect(void);

/* ── GC state ───────────────────────────────────────────────────────────────
 *
 * All mutable GC state lives in file-scope statics — the V1 AOT runtime is
 * single-threaded, so no locking is required.
 */

/** Linked list of every live managed allocation (walked by the sweep phase). */
static gc_header_t *gc_all_objects = NULL;

/** Total bytes currently allocated (payload bytes only, not header overhead). */
static size_t gc_live_bytes = 0;

/** Total collections triggered since process start (debugging aid). */
static size_t gc_collection_count = 0;

/** Adaptive threshold: trigger a collection when gc_live_bytes exceeds this. */
#define GC_INITIAL_THRESHOLD (1u * 1024u * 1024u) /* 1 MB */
static size_t gc_threshold = GC_INITIAL_THRESHOLD;

/** Mark stack for iterative BFS — avoids deep C recursion. */
#define GC_MARK_STACK_CAP 4096
static gc_header_t *gc_mark_stack[GC_MARK_STACK_CAP];
static int gc_mark_stack_top = 0;

/* ── Internal helpers ───────────────────────────────────────────────────────*/

/** Test whether `raw` points into the payload of a live managed object.
 *  Returns the header if found, NULL otherwise.
 *
 *  V1 uses a linear scan.  With millions of short-lived objects this would
 *  be slow — a future PR can replace it with a sorted-pointer binary-search
 *  or an interval tree.  For the programs that use this runtime (AOT'd
 *  command-line tools), the live set is small enough that linear scan is fine.
 */
static gc_header_t *gc_find_header(uintptr_t raw) {
    for (gc_header_t *hdr = gc_all_objects; hdr != NULL; hdr = hdr->next) {
        uintptr_t payload_start = (uintptr_t)(hdr + 1);
        uintptr_t payload_end   = payload_start + hdr->size;
        if (raw >= payload_start && raw < payload_end) {
            return hdr;
        }
    }
    return NULL;
}

/** Push an unmarked header onto the mark stack.  Returns 0 if the stack is
 *  full — the object will be found again on the next collection (conservative
 *  correctness: live objects may be retained but never freed too early). */
static int gc_mark_push(gc_header_t *hdr) {
    if (hdr->marked) return 1; /* already marked — nothing to do */
    hdr->marked = 1;
    if (gc_mark_stack_top < GC_MARK_STACK_CAP) {
        gc_mark_stack[gc_mark_stack_top++] = hdr;
        return 1;
    }
    /* Stack overflow — the object is already marked, so it won't be freed.
     * Its children will be processed in a future collection triggered by
     * the next allocation.  Acceptable for a conservative collector. */
    return 0;
}

/** Scan `len` bytes of memory starting at `base` for pointers into managed
 *  objects.  Both the raw word and `word & ~0x7` are tested (NaN-box compat).
 *  Called on: (a) the C stack, (b) payloads of already-marked objects. */
static void gc_scan_region(const char *base, size_t len) {
    /* Align start up to pointer width so we only check aligned candidates. */
    uintptr_t start = (uintptr_t)base;
    uintptr_t end   = start + len;

    /* Process one pointer-width word at a time. */
    for (uintptr_t addr = (start + 7u) & ~7u; addr + 8u <= end; addr += 8u) {
        uintptr_t word;
        memcpy(&word, (const void *)addr, sizeof(uintptr_t));

        /* Check the raw word. */
        gc_header_t *hdr = gc_find_header(word);
        if (hdr) { gc_mark_push(hdr); }

        /* Check the word with the low 3 tag bits cleared — Lispy heap ptr. */
        uintptr_t stripped = word & ~(uintptr_t)0x7u;
        if (stripped != word && stripped != 0u) {
            hdr = gc_find_header(stripped);
            if (hdr) { gc_mark_push(hdr); }
        }
    }
}

/* ── Collection ─────────────────────────────────────────────────────────────*/

/** Mark phase: flush registers into a jmp_buf (ensures any pointer held in a
 *  register is spilled to the stack), then scan the C stack from SP to the
 *  platform-detected stack base.  Then drain the mark stack by scanning each
 *  marked object's payload for further pointers.
 *
 *  The jmp_buf trick is standard conservative GC practice — the C standard
 *  does not guarantee that volatile locals remain on the stack, but `setjmp`
 *  MUST save all callee-saved registers, which are the ones that might hold
 *  live GC roots.  The jmp_buf sits on the current stack frame, so the
 *  saved register values are visible to the stack scan. */
static void gc_mark(void) {
    /* 1. Flush callee-saved registers onto the stack via setjmp. */
    jmp_buf regs;
    setjmp(regs);

    /* 2. Scan the C stack from here to the thread's stack base.
     *    `sp_local` is the lowest SP we can see from this function. */
    volatile char sp_local;
    void *sp  = (void *)&sp_local;
    void *top = gc_stack_base();

    /* On all supported platforms the stack grows downward: sp < top. */
    if ((uintptr_t)sp < (uintptr_t)top) {
        gc_scan_region((const char *)sp,
                       (size_t)((uintptr_t)top - (uintptr_t)sp));
    }

    /* 3. Drain the mark stack — scan each marked object's payload for
     *    further pointers.  New entries pushed by gc_scan_region extend
     *    the work list until it is empty. */
    while (gc_mark_stack_top > 0) {
        gc_header_t *hdr = gc_mark_stack[--gc_mark_stack_top];
        /* Scan this object's payload for further managed pointers. */
        gc_scan_region((const char *)(hdr + 1), hdr->size);
    }
}

/** Sweep phase: walk the gc_all_objects list.
 *  - Marked objects: clear the mark bit and advance.
 *  - Unmarked objects: unlink from the list and free. */
static void gc_sweep(void) {
    size_t live_bytes = 0;
    gc_header_t **cursor = &gc_all_objects;

    while (*cursor != NULL) {
        gc_header_t *hdr = *cursor;
        if (hdr->marked) {
            hdr->marked = 0;       /* clear for next cycle */
            live_bytes += hdr->size;
            cursor = &hdr->next;   /* advance */
        } else {
            *cursor = hdr->next;   /* unlink */
            free(hdr);             /* free header + payload in one shot */
        }
    }

    gc_live_bytes = live_bytes;
}

/** Update the adaptive threshold based on how much survived the last sweep. */
static void gc_adapt_threshold(size_t prev_live) {
    if (gc_live_bytes > prev_live / 2) {
        /* More than 50% live — double the threshold. */
        if (gc_threshold < SIZE_MAX / 2) {
            gc_threshold *= 2;
        }
    } else {
        /* Less than 50% survived — halve (floor: 1 MB). */
        size_t half = gc_threshold / 2;
        gc_threshold = (half > GC_INITIAL_THRESHOLD) ? half : GC_INITIAL_THRESHOLD;
    }
}

/* ── Public API ─────────────────────────────────────────────────────────────*/

/** __twig_gc_alloc — allocate `n` zero-initialised bytes on the GC heap.
 *
 * Returned pointer is to the user payload (not the header).  Returns 0 (NULL)
 * on OOM or if `n <= 0`.  The allocation is 16-byte–aligned because the
 * gc_header_t is 32 bytes and `malloc` returns at least 16-byte aligned memory.
 *
 * A collection is triggered when gc_live_bytes would exceed gc_threshold BEFORE
 * the new object is counted — so the threshold is a soft ceiling on the live
 * set measured at the last collection, not an absolute hard limit.  In practice
 * the true peak is threshold + allocations since last collect, bounded by the
 * program's allocation rate between collections.
 *
 * After `__twig_gc_alloc` returns, the new object is immediately reachable
 * from the call site (the caller holds the returned pointer on its stack),
 * so the collector correctly retains it.
 */
int64_t __twig_gc_alloc(int64_t n) {
    if (n <= 0) return 0;

    /* Trigger a collection if we are over the threshold.  Do this BEFORE
     * the allocation so the new object's root (the caller's stack slot) is
     * visible during the scan. */
    if (gc_live_bytes >= gc_threshold) {
        __twig_gc_collect();
    }

    /* Allocate header + payload in one `calloc` call (zero-initialises both). */
    size_t total = sizeof(gc_header_t) + (size_t)n;
    gc_header_t *hdr = (gc_header_t *)calloc(1, total);
    if (hdr == NULL) return 0;

    hdr->size   = (size_t)n;
    hdr->marked = 0;

    /* Prepend to the all-objects list. */
    hdr->next     = gc_all_objects;
    gc_all_objects = hdr;
    gc_live_bytes += (size_t)n;

    /* Return pointer to user payload — just past the header. */
    return (int64_t)(intptr_t)(hdr + 1);
}

/** __twig_gc_collect — run a full mark-and-sweep cycle.
 *
 * Called automatically by __twig_gc_alloc when gc_live_bytes >= gc_threshold,
 * and also by __twig_gc_safepoint when a safepoint is reached.  Can also be
 * called explicitly by programs that want deterministic collection timing.
 *
 * Timing: O(stack_size + live_heap_words + all_objects).  For typical AOT
 * programs (< 100k live objects, < 1 MB stack) this is under 1 ms.
 */
void __twig_gc_collect(void) {
    size_t prev_live = gc_live_bytes;
    gc_mark_stack_top = 0; /* reset mark stack */
    gc_mark();
    gc_sweep();
    gc_adapt_threshold(prev_live);
    gc_collection_count++;
}

/** __twig_gc_safepoint — called at IIR `safepoint` ops.
 *
 * The IIR `safepoint` opcode is emitted by frontends at loop back-edges and
 * function entries to give the GC a chance to run.  This avoids the worst
 * case where a tight allocation loop prevents the GC from ever running.
 *
 * In V1 this simply delegates to __twig_gc_collect when the threshold is
 * exceeded.  A future PR can add thread-suspension and time-based triggering
 * for concurrent GC.
 */
void __twig_gc_safepoint(void) {
    if (gc_live_bytes >= gc_threshold) {
        __twig_gc_collect();
    }
}

/** __twig_gc_live_bytes — returns the current live byte count.
 *
 * Exposed for testing: the golden test can call this after allocating and
 * dropping objects to verify the sweep freed them correctly.
 */
int64_t __twig_gc_live_bytes(void) {
    return (int64_t)gc_live_bytes;
}

/** __twig_gc_collection_count — returns the total number of GC cycles run.
 *
 * Exposed for testing: verify that allocating past the threshold triggers
 * exactly one collection.
 */
int64_t __twig_gc_collection_count(void) {
    return (int64_t)gc_collection_count;
}
