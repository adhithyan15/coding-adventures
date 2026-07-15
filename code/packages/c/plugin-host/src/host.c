/*
 * host.c — the plugin host, implemented on os-platform's dynlib primitive.
 * ===========================================================================
 *
 * This is OS-agnostic: every OS-specific detail of loading a library and
 * resolving a symbol is already handled by os_platform/dynlib (dlopen/dlsym on
 * POSIX, LoadLibrary/GetProcAddress on Windows). So there is one host.c for all
 * platforms — no per-OS backend and no #ifdef. It just knows the plugin ABI.
 *
 * The resolved symbol is a void*; it is converted to the entry function pointer
 * with memcpy — the portable, warning-free idiom dynlib documents for turning an
 * object pointer into a function pointer.
 */
#include "plugin_host/host.h"

#include "os_platform/dynlib.h"

#include <stdlib.h>
#include <string.h>

struct osp_plugin {
    osp_dynlib *lib;
    osp_plugin_entry_fn entry;
};

osp_status osp_plugin_open(const char *path, osp_plugin **out) {
    osp_plugin *p;
    osp_dynlib *lib;
    void *sym;
    osp_status st;

    if (path == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp_dynlib_open(&lib, path);
    if (st != OSP_OK) {
        return st;
    }
    st = osp_dynlib_symbol(lib, OSP_PLUGIN_ENTRY_NAME, &sym);
    if (st != OSP_OK) {
        osp_dynlib_close(lib);
        return st;
    }
    p = (osp_plugin *)malloc(sizeof(*p));
    if (p == NULL) {
        osp_dynlib_close(lib);
        return OSP_ERR_NOMEM;
    }
    p->lib = lib;
    /* void* symbol address -> entry function pointer, bit-for-bit. */
    memcpy(&p->entry, &sym, sizeof(p->entry));
    *out = p;
    return OSP_OK;
}

osp_status osp_plugin_call(osp_plugin *p, int arg, int *out) {
    if (p == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    *out = p->entry(arg);
    return OSP_OK;
}

osp_status osp_plugin_close(osp_plugin *p) {
    osp_status st;
    if (p == NULL) {
        return OSP_ERR_INVAL;
    }
    st = osp_dynlib_close(p->lib);
    free(p);
    return st;
}
