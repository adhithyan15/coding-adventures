/*
 * mmap_windows.c — the Win32 backend of os_platform/mmap.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use mmap_posix.c
 * via the shared `BUILD`). No OS #ifdefs — the build chose this file. Uses only
 * kernel32.
 *
 * VirtualAlloc reserves and commits pages in one call; the protection is one of
 * the PAGE_* constants (which, unlike POSIX's orthogonal PROT_ bits, are a fixed
 * matrix — so we map our bitmask onto the right constant explicitly).
 */
#include "os_platform/mmap.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdlib.h>

struct osp_mapping {
    void *base;
    size_t len;
};

/* Map our OSP_PROT_* bitmask onto the Win32 PAGE_* matrix. */
static DWORD osp__prot_to_win(int prot) {
    int w = prot & OSP_PROT_WRITE;
    int r = prot & OSP_PROT_READ;
    if (prot & OSP_PROT_EXEC) {
        if (w) {
            return PAGE_EXECUTE_READWRITE;
        }
        if (r) {
            return PAGE_EXECUTE_READ;
        }
        return PAGE_EXECUTE;
    }
    if (w) {
        return PAGE_READWRITE;
    }
    if (r) {
        return PAGE_READONLY;
    }
    return PAGE_NOACCESS;
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
    base = VirtualAlloc(NULL, len, MEM_RESERVE | MEM_COMMIT,
                        osp__prot_to_win(prot));
    if (base == NULL) {
        free(m);
        return OSP_ERR_OS;
    }
    m->base = base;
    m->len = len;
    *out = m;
    return OSP_OK;
}

osp_status osp_map_protect(osp_mapping *m, int prot) {
    DWORD old_prot;
    if (m == NULL) {
        return OSP_ERR_INVAL;
    }
    /* VirtualProtect requires a non-NULL out-parameter for the old protection,
     * even when we do not need the value. */
    if (!VirtualProtect(m->base, m->len, osp__prot_to_win(prot), &old_prot)) {
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
    /* MEM_RELEASE frees the whole reservation; its dwSize argument must be 0.
     * Free the wrapper unconditionally (see the POSIX backend for why). */
    if (!VirtualFree(m->base, 0, MEM_RELEASE)) {
        st = OSP_ERR_OS;
    }
    free(m);
    return st;
}
