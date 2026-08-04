/*
 * os_platform/dynlib.h — load a shared library, resolve a symbol, close it.
 * ===========================================================================
 *
 * The fifth os-platform primitive, and the foundation for plugins and FFI. ISO C
 * has no notion of loading code at run time; it is entirely the OS's domain:
 *
 *      step            macOS / Linux    Windows
 *      ──────────────  ───────────────  ─────────────────
 *      load a library  dlopen           LoadLibrary
 *      resolve symbol  dlsym            GetProcAddress
 *      unload          dlclose          FreeLibrary
 *
 * WHY THIS LIVES ON platform-harness, NOT iso-harness. Resolving a symbol yields
 * an address that the caller will usually invoke as a FUNCTION, but dlsym returns
 * `void *` (an object pointer). Converting between object and function pointers
 * is not defined by strict ISO C — which is exactly the kind of thing
 * platform-harness allows (it keeps -Wall -Wextra -Werror but drops
 * -pedantic-errors). POSIX itself guarantees the round trip works.
 *
 * CALLING A RESOLVED SYMBOL. `osp_dynlib_symbol` writes the address into a
 * `void *`. To call it, convert that to the right function-pointer type. The
 * portable, warning-free way (clean even under MSVC /W4 /WX, which rejects a
 * direct object<->function pointer cast) is to copy the bits:
 *
 *      void *addr;
 *      osp_dynlib_symbol(lib, "some_func", &addr);
 *      int (*fn)(int);
 *      memcpy(&fn, &addr, sizeof fn);   // object* -> function*, bit-for-bit
 *      int r = fn(7);
 *
 * MODEL. An opaque handle from osp_dynlib_open is used with osp_dynlib_symbol and
 * released by osp_dynlib_close (which frees the handle and drops the library's
 * reference count). No OS handle leaks.
 *
 * BUILD. Compiled by platform-harness. On Linux the POSIX backend links `-ldl`
 * (the BUILD adds it on Linux only — macOS has dlopen in libc and no libdl);
 * Windows uses only kernel32.
 */
#ifndef OS_PLATFORM_DYNLIB_H
#define OS_PLATFORM_DYNLIB_H

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque loaded-library handle. Created by osp_dynlib_open, freed by
 * osp_dynlib_close. Do not copy or free it yourself. */
typedef struct osp_dynlib osp_dynlib;

/*
 * osp_dynlib_open — load the shared library at `path`.
 * Writes a handle through *out on success. Returns OSP_ERR_INVAL if out or path
 * is NULL, OSP_ERR_NOMEM on an allocation failure, OSP_ERR_OS if the library
 * cannot be loaded.
 */
osp_status osp_dynlib_open(osp_dynlib **out, const char *path);

/*
 * osp_dynlib_symbol — resolve `name` in `lib`, writing its address to *out_sym.
 * See the header comment for how to invoke a function symbol. Returns
 * OSP_ERR_INVAL if any argument is NULL, OSP_ERR_OS if the symbol is not found.
 */
osp_status osp_dynlib_symbol(osp_dynlib *lib, const char *name, void **out_sym);

/*
 * osp_dynlib_close — unload `lib` and free the handle. Returns OSP_ERR_INVAL if
 * lib is NULL, OSP_ERR_OS if the OS unload call fails. Any symbol addresses
 * previously resolved from `lib` are invalid after this returns.
 */
osp_status osp_dynlib_close(osp_dynlib *lib);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_DYNLIB_H */
