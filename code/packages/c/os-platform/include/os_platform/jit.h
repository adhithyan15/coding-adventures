/*
 * os_platform/jit.h — emit machine code at run time and call it.
 * ===========================================================================
 *
 * The seventh os-platform primitive, and the follow-up promised in mmap.h's
 * scope note. `mmap` plumbs an EXEC bit, but on a modern OS you cannot simply
 * map a page RWX, scribble instructions, and jump in: W^X (write-xor-execute) is
 * enforced, and each OS has its own protocol for the RW → RX transition a JIT
 * needs. `jit` encapsulates exactly that protocol so a code generator can:
 *
 *      alloc → write bytes → commit → call the entry
 *
 *      step        macOS (Apple Silicon)         Linux                Windows
 *      ─────────   ───────────────────────────   ──────────────────   ─────────────────────
 *      alloc       mmap MAP_JIT (RWX)            mmap RW              VirtualAlloc RW
 *      write       pthread_jit_write_protect(0)  memcpy               memcpy
 *                  → memcpy → …_protect(1)
 *      commit      sys_icache_invalidate         mprotect RX +        VirtualProtect RX +
 *                                                __clear_cache        FlushInstructionCache
 *
 * The Apple Silicon path is the hard one: hardened W^X means the page toggles
 * per-thread between writable and executable, and the instruction cache must be
 * flushed before the freshly written code is fetched. On x86 the i-cache is
 * coherent so the flush is a no-op, but arm64 (Linux and macOS) genuinely needs
 * it — omitting it runs stale bytes.
 *
 * MODEL. osp_jit_alloc reserves a JIT buffer with a byte capacity (writable to
 * begin with). osp_jit_write appends machine code (repeatable up to capacity).
 * osp_jit_commit flips the buffer to read+execute and flushes the i-cache; after
 * commit the code is callable and no more writes are allowed. osp_jit_entry
 * returns the callable address (NULL until committed). osp_jit_free releases it.
 *
 * THREADING. A single osp_jit buffer must be built (alloc → write → commit) by
 * one thread: the Apple write-protect toggle is per-thread state, so interleaving
 * writes to the same buffer across threads is a caller error. Once committed, the
 * read-only code is of course safe to call from any thread.
 *
 * CALLING the entry: cast it to the function-pointer type your emitted code
 * implements. The portable, warning-free cast is via memcpy (an object pointer
 * and a function pointer need not be the same size in ISO C, though they are on
 * every OS here) — see the test for the idiom.
 *
 * BUILD. Compiled by platform-harness; the POSIX backend needs _DEFAULT_SOURCE /
 * _DARWIN_C_SOURCE (added by the BUILD) so MAP_ANONYMOUS / MAP_JIT are visible,
 * and links no extra library (the Apple JIT calls live in libSystem). Windows
 * uses only kernel32.
 */
#ifndef OS_PLATFORM_JIT_H
#define OS_PLATFORM_JIT_H

#include <stddef.h> /* size_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque JIT buffer. Created by osp_jit_alloc, freed by osp_jit_free. */
typedef struct osp_jit osp_jit;

/*
 * osp_jit_alloc — reserve a JIT buffer able to hold at least `capacity` bytes of
 * machine code, initially writable. Writes a handle through *out. Returns
 * OSP_ERR_INVAL (out NULL or capacity 0), OSP_ERR_NOMEM (handle allocation), or
 * OSP_ERR_OS (the OS mapping call failed).
 */
osp_status osp_jit_alloc(osp_jit **out, size_t capacity);

/*
 * osp_jit_write — append `len` bytes of machine code to the buffer. May be
 * called repeatedly before commit; the total may not exceed the requested
 * capacity. Handles the Apple-Silicon write-protect toggle internally. Returns
 * OSP_ERR_INVAL (j or code NULL, already committed, or would exceed capacity).
 */
osp_status osp_jit_write(osp_jit *j, const void *code, size_t len);

/*
 * osp_jit_commit — flip the buffer to read+execute and flush the instruction
 * cache so the CPU sees the freshly written code. After commit the entry is
 * callable and further writes are rejected. Returns OSP_ERR_INVAL (j NULL or
 * already committed) or OSP_ERR_OS (the protection change failed).
 */
osp_status osp_jit_commit(osp_jit *j);

/*
 * osp_jit_entry — the callable base address of the committed code, or NULL if j
 * is NULL or the buffer has not been committed yet.
 */
void *osp_jit_entry(const osp_jit *j);

/*
 * osp_jit_free — release the buffer and free the handle. Returns OSP_ERR_INVAL
 * if j is NULL, OSP_ERR_OS if the OS release call fails (the handle is freed
 * either way, matching the other primitives' "free releases the handle").
 */
osp_status osp_jit_free(osp_jit *j);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_JIT_H */
