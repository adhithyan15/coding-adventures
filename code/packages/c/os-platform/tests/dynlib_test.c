/*
 * dynlib_test.c — load/resolve/close tests for os_platform/dynlib, per OS.
 * ===========================================================================
 *
 * We load a library guaranteed to be present on the platform, resolve a
 * well-known no-argument function from it, call that function through the
 * resolved address, and check the result is sane. This proves the whole chain:
 * open -> symbol -> a usable code address -> close. Which library and symbol
 * exist is inherently OS-specific, so (only in this test — never in the library
 * backends) a #ifdef selects them:
 *
 *      OS       library            symbol (returns a positive id)
 *      ───────  ─────────────────  ──────────────────────────────
 *      Linux    libc.so.6          getpid
 *      macOS    libSystem.dylib    getpid
 *      Windows  kernel32.dll       GetCurrentProcessId
 *
 * The resolved void* is turned into a callable function pointer with memcpy —
 * the portable, warning-free idiom (a direct cast trips MSVC /W4 /WX). Error and
 * NULL-argument paths are checked too, and they are the same on every OS.
 */
#include "iso_test.h"

#include "os_platform/dynlib.h"

#include <stddef.h> /* NULL */
#include <string.h> /* memcpy */

#ifdef _WIN32
#define OSP_TEST_LIB "kernel32.dll"
#define OSP_TEST_SYM "GetCurrentProcessId"
typedef unsigned long (*osp_id_fn)(void); /* DWORD GetCurrentProcessId(void) */
#elif defined(__APPLE__)
#define OSP_TEST_LIB "libSystem.dylib"
#define OSP_TEST_SYM "getpid"
typedef int (*osp_id_fn)(void); /* pid_t getpid(void) */
#else
#define OSP_TEST_LIB "libc.so.6"
#define OSP_TEST_SYM "getpid"
typedef int (*osp_id_fn)(void);
#endif

int main(void) {
    osp_dynlib *lib = NULL;
    osp_dynlib *bad = NULL;
    void *sym = NULL;
    osp_id_fn fn;

    /* ── open → symbol → call → close ───────────────────────────────────── */
    ISO_CHECK(osp_dynlib_open(&lib, OSP_TEST_LIB) == OSP_OK);
    if (lib != NULL) {
        ISO_CHECK(osp_dynlib_symbol(lib, OSP_TEST_SYM, &sym) == OSP_OK);
        ISO_CHECK_MSG(sym != NULL, "resolved symbol address must be non-NULL");

        /* void* -> function pointer, bit-for-bit (warning-free everywhere). */
        memcpy(&fn, &sym, sizeof(fn));
        ISO_CHECK_MSG(fn() > 0, "calling the resolved function should yield a positive id");

        /* a missing symbol must fail cleanly */
        ISO_CHECK(osp_dynlib_symbol(lib, "osp_no_such_symbol_zzz", &sym) == OSP_ERR_OS);

        ISO_CHECK(osp_dynlib_close(lib) == OSP_OK);
    } else {
        ISO_CHECK_MSG(0, "osp_dynlib_open returned OK but a NULL handle");
    }

    /* ── error + NULL-argument paths (identical on every OS) ─────────────── */
    ISO_CHECK(osp_dynlib_open(&bad, "osp_no_such_library_zzz.xyz") == OSP_ERR_OS);
    ISO_CHECK(osp_dynlib_open(NULL, OSP_TEST_LIB) == OSP_ERR_INVAL);
    ISO_CHECK(osp_dynlib_open(&lib, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_dynlib_symbol(NULL, OSP_TEST_SYM, &sym) == OSP_ERR_INVAL);
    ISO_CHECK(osp_dynlib_close(NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
