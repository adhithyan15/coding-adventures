/*
 * jit_posix.c — the POSIX backend of os_platform/jit (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * jit_windows.c via `BUILD_windows`). The two POSIX platforms genuinely diverge
 * in how a JIT page is made executable, so this file has a single
 * `#if defined(__APPLE__)` split — the same convention mmap_posix.c uses for its
 * MAP_ANON feature-macro difference:
 *
 *   - Apple (esp. Apple Silicon): map RWX with MAP_JIT, then toggle the page
 *     writable/executable per-thread with pthread_jit_write_protect_np and flush
 *     the i-cache with sys_icache_invalidate. Hardened W^X requires this dance.
 *   - Linux: map RW, memcpy, mprotect to RX, then __builtin___clear_cache — a
 *     no-op on x86 (coherent i-cache) but a real flush on arm64.
 */
#include "os_platform/jit.h"

#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h> /* sysconf */

#if defined(__APPLE__)
#include <libkern/OSCacheControl.h> /* sys_icache_invalidate */
#include <pthread.h>                /* pthread_jit_write_protect_np */
#endif

struct osp_jit {
    void *base;    /* mapping base (page-aligned)           */
    size_t cap;    /* requested capacity (write limit)      */
    size_t mapped; /* page-rounded mapping length (unmap)   */
    size_t len;    /* bytes written so far                  */
    int committed; /* 1 once flipped to R+X                 */
};

/* Round `n` up to a whole page (never zero). */
static size_t osp__round_page(size_t n) {
    long pg = sysconf(_SC_PAGESIZE);
    size_t p = (pg > 0) ? (size_t)pg : 4096u;
    return ((n + p - 1) / p) * p;
}

osp_status osp_jit_alloc(osp_jit **out, size_t capacity) {
    struct osp_jit *j;
    void *base;
    size_t mapped;

    if (out == NULL || capacity == 0) {
        return OSP_ERR_INVAL;
    }
    j = (struct osp_jit *)malloc(sizeof(*j));
    if (j == NULL) {
        return OSP_ERR_NOMEM;
    }
    mapped = osp__round_page(capacity);
    /* Reject a capacity so large the page round-up overflowed size_t (which
     * would make `mapped` wrap below `capacity`) — explicit rather than relying
     * on a zero-length OS mapping failing. */
    if (mapped < capacity) {
        free(j);
        return OSP_ERR_OS;
    }
#if defined(__APPLE__)
    /* MAP_JIT: an RWX region whose writability is gated per-thread by the
     * write-protect toggle below. Required under the hardened runtime. */
    base = mmap(NULL, mapped, PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_PRIVATE | MAP_ANON | MAP_JIT, -1, 0);
#else
    /* Linux: start writable-only; flip to executable at commit. */
    base = mmap(NULL, mapped, PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
#endif
    if (base == MAP_FAILED) {
        free(j);
        return OSP_ERR_OS;
    }
    j->base = base;
    j->cap = capacity;
    j->mapped = mapped;
    j->len = 0;
    j->committed = 0;
    *out = j;
    return OSP_OK;
}

osp_status osp_jit_write(osp_jit *j, const void *code, size_t len) {
    if (j == NULL || code == NULL || j->committed) {
        return OSP_ERR_INVAL;
    }
    /* Overflow-safe capacity check: cap >= len is an invariant, so cap - len
     * never wraps. */
    if (len > j->cap - j->len) {
        return OSP_ERR_INVAL;
    }
#if defined(__APPLE__)
    pthread_jit_write_protect_np(0); /* make the JIT page writable */
#endif
    memcpy((unsigned char *)j->base + j->len, code, len);
#if defined(__APPLE__)
    pthread_jit_write_protect_np(1); /* back to executable */
#endif
    j->len += len;
    return OSP_OK;
}

osp_status osp_jit_commit(osp_jit *j) {
    if (j == NULL || j->committed) {
        return OSP_ERR_INVAL;
    }
#if defined(__APPLE__)
    /* Already RWX via MAP_JIT and already write-protected (executable) after the
     * last write; just flush the i-cache so the CPU fetches the new bytes. */
    pthread_jit_write_protect_np(1);
    sys_icache_invalidate(j->base, j->len);
#else
    if (mprotect(j->base, j->mapped, PROT_READ | PROT_EXEC) != 0) {
        return OSP_ERR_OS;
    }
    /* Flush the i-cache: a no-op on x86, a real flush on arm64 Linux. */
    __builtin___clear_cache((char *)j->base, (char *)j->base + j->len);
#endif
    j->committed = 1;
    return OSP_OK;
}

void *osp_jit_entry(const osp_jit *j) {
    return (j != NULL && j->committed) ? j->base : NULL;
}

osp_status osp_jit_free(osp_jit *j) {
    osp_status st = OSP_OK;
    if (j == NULL) {
        return OSP_ERR_INVAL;
    }
    if (munmap(j->base, j->mapped) != 0) {
        st = OSP_ERR_OS;
    }
    free(j);
    return st;
}
