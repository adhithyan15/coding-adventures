/*
 * dynlib_windows.c — the Win32 backend of os_platform/dynlib.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use dynlib_posix.c
 * via the shared `BUILD`). No OS #ifdefs — the build chose this file. Uses only
 * kernel32.
 *
 * GetProcAddress returns a FARPROC (a function pointer), but our API hands the
 * caller a `void *`. A direct function-pointer -> object-pointer cast is exactly
 * what MSVC flags as C4054 (and it would become an error under /WX), so we copy
 * the bits with memcpy instead — well-defined here because a code address and a
 * data pointer are the same width on every Windows ABI, and warning-free on every
 * compiler.
 */
#include "os_platform/dynlib.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdlib.h>
#include <string.h> /* memcpy */

struct osp_dynlib {
    HMODULE handle;
};

osp_status osp_dynlib_open(osp_dynlib **out, const char *path) {
    struct osp_dynlib *lib;
    HMODULE h;

    if (out == NULL || path == NULL) {
        return OSP_ERR_INVAL;
    }
    h = LoadLibraryA(path);
    if (h == NULL) {
        return OSP_ERR_OS;
    }
    lib = (struct osp_dynlib *)malloc(sizeof(*lib));
    if (lib == NULL) {
        FreeLibrary(h);
        return OSP_ERR_NOMEM;
    }
    lib->handle = h;
    *out = lib;
    return OSP_OK;
}

osp_status osp_dynlib_symbol(osp_dynlib *lib, const char *name, void **out_sym) {
    FARPROC sym;

    if (lib == NULL || name == NULL || out_sym == NULL) {
        return OSP_ERR_INVAL;
    }
    sym = GetProcAddress(lib->handle, name);
    if (sym == NULL) {
        return OSP_ERR_OS;
    }
    /* FARPROC (function pointer) -> void *, bit-for-bit, to avoid the C4054
     * function-to-data pointer cast diagnostic. Same width on every Win32 ABI. */
    memcpy(out_sym, &sym, sizeof(sym));
    return OSP_OK;
}

osp_status osp_dynlib_close(osp_dynlib *lib) {
    osp_status st = OSP_OK;
    if (lib == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Free the wrapper unconditionally: the contract says close frees the
     * handle, so a caller will not retry after a failure — keeping the struct
     * would leak it. We still report an unload failure via the status. */
    if (!FreeLibrary(lib->handle)) {
        st = OSP_ERR_OS;
    }
    free(lib);
    return st;
}
