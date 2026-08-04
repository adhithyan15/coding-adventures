/*
 * dynlib_posix.c — the POSIX backend of os_platform/dynlib (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * dynlib_windows.c via `BUILD_windows`). No OS #ifdefs — the build chose this
 * file. Links `-ldl` on Linux (the BUILD adds it there only; macOS has dlopen in
 * libc and ships no libdl).
 *
 * The one subtlety is in symbol lookup: dlsym returns NULL both when a symbol is
 * missing AND when a symbol legitimately has the value NULL. POSIX's way to tell
 * them apart is to clear dlerror() first, then, if dlsym returned NULL, check
 * whether dlerror() now reports a message. We honour that so a real (but
 * NULL-valued) symbol is not misreported as missing.
 */
#include "os_platform/dynlib.h"

#include <dlfcn.h>
#include <stdlib.h>

struct osp_dynlib {
    void *handle;
};

osp_status osp_dynlib_open(osp_dynlib **out, const char *path) {
    struct osp_dynlib *lib;
    void *h;

    if (out == NULL || path == NULL) {
        return OSP_ERR_INVAL;
    }
    /* RTLD_NOW: resolve every symbol immediately, so a broken library fails here
     * rather than at first use. RTLD_LOCAL: do not leak its symbols into the
     * global namespace. */
    h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (h == NULL) {
        return OSP_ERR_OS;
    }
    lib = (struct osp_dynlib *)malloc(sizeof(*lib));
    if (lib == NULL) {
        dlclose(h);
        return OSP_ERR_NOMEM;
    }
    lib->handle = h;
    *out = lib;
    return OSP_OK;
}

osp_status osp_dynlib_symbol(osp_dynlib *lib, const char *name, void **out_sym) {
    void *sym;

    if (lib == NULL || name == NULL || out_sym == NULL) {
        return OSP_ERR_INVAL;
    }
    dlerror(); /* clear any stale error before the lookup */
    sym = dlsym(lib->handle, name);
    if (sym == NULL && dlerror() != NULL) {
        /* NULL AND an error message => genuinely not found. (NULL with no error
         * means the symbol exists and its value is NULL — still a success.) */
        return OSP_ERR_OS;
    }
    *out_sym = sym;
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
    if (dlclose(lib->handle) != 0) {
        st = OSP_ERR_OS;
    }
    free(lib);
    return st;
}
