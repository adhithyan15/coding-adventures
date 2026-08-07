/*
 * jit_windows.c — the Win32 backend of os_platform/jit.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use jit_posix.c via
 * the shared `BUILD`). Windows has no W^X toggle: reserve+commit the region
 * read-write, memcpy the code, then VirtualProtect it to PAGE_EXECUTE_READ and
 * FlushInstructionCache. No #ifdefs — the build chose this file. Links kernel32.
 */
#include "os_platform/jit.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdlib.h>
#include <string.h>

struct osp_jit {
    void *base;
    size_t cap;
    size_t mapped;
    size_t len;
    int committed;
};

static size_t osp__round_page(size_t n) {
    SYSTEM_INFO si;
    size_t p;
    GetSystemInfo(&si);
    p = (si.dwPageSize != 0) ? (size_t)si.dwPageSize : 4096u;
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
     * would make `mapped` wrap below `capacity`). */
    if (mapped < capacity) {
        free(j);
        return OSP_ERR_OS;
    }
    base = VirtualAlloc(NULL, mapped, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if (base == NULL) {
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
    if (len > j->cap - j->len) {
        return OSP_ERR_INVAL;
    }
    memcpy((unsigned char *)j->base + j->len, code, len);
    j->len += len;
    return OSP_OK;
}

osp_status osp_jit_commit(osp_jit *j) {
    DWORD old_prot;
    if (j == NULL || j->committed) {
        return OSP_ERR_INVAL;
    }
    if (!VirtualProtect(j->base, j->mapped, PAGE_EXECUTE_READ, &old_prot)) {
        return OSP_ERR_OS;
    }
    if (!FlushInstructionCache(GetCurrentProcess(), j->base, j->len)) {
        return OSP_ERR_OS;
    }
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
    if (!VirtualFree(j->base, 0, MEM_RELEASE)) {
        st = OSP_ERR_OS;
    }
    free(j);
    return st;
}
