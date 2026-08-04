/*
 * os_platform/mmap.h — anonymous virtual memory with protection control.
 * ===========================================================================
 *
 * The sixth os-platform primitive. `malloc` gives you bytes, but it cannot give
 * you a *whole page range with a chosen protection* — read-only data, guard
 * pages, or (with EXEC) memory you can execute, the substrate a JIT emits into.
 * That is the OS's job:
 *
 *      operation           macOS / Linux    Windows
 *      ──────────────────  ───────────────  ─────────────────────
 *      reserve+commit      mmap             VirtualAlloc
 *      change protection   mprotect         VirtualProtect
 *      release             munmap           VirtualFree
 *
 * MODEL. osp_map_anon reserves and commits `len` bytes of fresh, zero-filled,
 * page-aligned anonymous memory with an initial protection, returning an opaque
 * handle. osp_map_protect changes the protection later (e.g. write code as RW,
 * then flip to R+X). osp_map_base / osp_map_size expose the region;
 * osp_map_unmap releases it and frees the handle. No mapping leaks.
 *
 * PROTECTION is a bitmask of OSP_PROT_* below. A classic W^X sequence maps RW,
 * writes, then re-protects to READ|EXEC.
 *
 * EXECUTABLE MEMORY — SCOPE NOTE. The EXEC bit is plumbed through to
 * PROT_EXEC / PAGE_EXECUTE_* so JIT consumers can request it, and it works
 * directly on Linux and Windows (map RW, emit, re-protect to R+X, run). Apple
 * Silicon's hardened runtime additionally requires MAP_JIT plus a per-thread
 * write-protect toggle and an instruction-cache flush. That full protocol — and
 * a cross-architecture emit-and-call test — now lives in the sibling `jit`
 * primitive (os_platform/jit.h); this header's own tests deliberately stay with
 * anonymous read/write memory and protection changes, which are identical on
 * every OS.
 *
 * BUILD. Compiled by platform-harness; the POSIX backend needs _DEFAULT_SOURCE
 * (added by the BUILD) so MAP_ANONYMOUS is visible on glibc, and links no extra
 * library. Windows uses only kernel32.
 */
#ifndef OS_PLATFORM_MMAP_H
#define OS_PLATFORM_MMAP_H

#include <stddef.h> /* size_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Protection bits — combine with bitwise OR. */
enum {
    OSP_PROT_NONE = 0,  /* no access                    */
    OSP_PROT_READ = 1,  /* readable                     */
    OSP_PROT_WRITE = 2, /* writable                     */
    OSP_PROT_EXEC = 4   /* executable (for JIT/codegen) */
};

/* Opaque mapping handle. Created by osp_map_anon, freed by osp_map_unmap. */
typedef struct osp_mapping osp_mapping;

/*
 * osp_map_anon — reserve+commit `len` bytes of anonymous, zero-filled memory
 * with protection `prot` (a bitmask of OSP_PROT_*). `len` is rounded up to a
 * page by the OS; the usable region is at least `len` bytes. Writes a handle
 * through *out. Returns OSP_ERR_INVAL (out NULL or len 0), OSP_ERR_NOMEM on
 * handle allocation failure, OSP_ERR_OS if the OS mapping call fails.
 */
osp_status osp_map_anon(osp_mapping **out, size_t len, int prot);

/*
 * osp_map_protect — change the protection of the whole mapping to `prot`.
 * Returns OSP_ERR_INVAL if m is NULL, OSP_ERR_OS on failure.
 */
osp_status osp_map_protect(osp_mapping *m, int prot);

/* Base address of the mapping (page-aligned), or NULL if m is NULL. */
void *osp_map_base(const osp_mapping *m);

/* The requested length in bytes, or 0 if m is NULL. */
size_t osp_map_size(const osp_mapping *m);

/*
 * osp_map_unmap — release the mapping and free the handle. Returns OSP_ERR_INVAL
 * if m is NULL, OSP_ERR_OS if the OS release call fails (the handle is freed
 * either way, matching the "unmap frees the handle" contract).
 */
osp_status osp_map_unmap(osp_mapping *m);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_MMAP_H */
