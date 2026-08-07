/*
 * mmap_posix.c — the POSIX backend of os_platform/mmap (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * mmap_windows.c via `BUILD_windows`). No OS #ifdefs — the build chose this
 * file. Uses only libc (mmap/mprotect/munmap).
 *
 * MAP_ANONYMOUS is not strict POSIX, so on glibc it is gated behind
 * _DEFAULT_SOURCE, which the BUILD defines for this primitive (harmless on
 * macOS, whose headers expose it unconditionally).
 */
#include "os_platform/mmap.h"

#include <stdlib.h>
#include <sys/mman.h>

struct osp_mapping {
    void *base;
    size_t len;
};

/* Translate our OSP_PROT_* bitmask into the OS's PROT_* bitmask. */
static int osp__prot_to_posix(int prot) {
    int p = 0;
    if (prot & OSP_PROT_READ) {
        p |= PROT_READ;
    }
    if (prot & OSP_PROT_WRITE) {
        p |= PROT_WRITE;
    }
    if (prot & OSP_PROT_EXEC) {
        p |= PROT_EXEC;
    }
    return (p == 0) ? PROT_NONE : p;
}

osp_status osp_map_anon(osp_mapping **out, size_t len, int prot) {
    struct osp_mapping *m;
    void *base;

    if (out == NULL || len == 0) {
        return OSP_ERR_INVAL;
    }
    m = (struct osp_mapping *)malloc(sizeof(*m));
    if (m == NULL) {
        return OSP_ERR_NOMEM;
    }
    /* MAP_PRIVATE | MAP_ANONYMOUS with fd -1: fresh, zero-filled, copy-on-write
     * pages backed by no file. */
    base = mmap(NULL, len, osp__prot_to_posix(prot),
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base == MAP_FAILED) {
        free(m);
        return OSP_ERR_OS;
    }
    m->base = base;
    m->len = len;
    *out = m;
    return OSP_OK;
}

osp_status osp_map_protect(osp_mapping *m, int prot) {
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    if (mprotect(m->base, m->len, osp__prot_to_posix(prot)) != 0) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

void *osp_map_base(const osp_mapping *m) {
    return (m != NULL) ? m->base : NULL;
}

size_t osp_map_size(const osp_mapping *m) {
    return (m != NULL) ? m->len : 0;
}

osp_status osp_map_unmap(osp_mapping *m) {
    osp_status st = OSP_OK;
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Free the wrapper unconditionally (the contract says unmap frees it, so a
     * caller will not retry); report an OS failure via the status. */
    if (munmap(m->base, m->len) != 0) {
        st = OSP_ERR_OS;
    }
    free(m);
    return st;
}
