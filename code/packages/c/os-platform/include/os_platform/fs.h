/*
 * os_platform/fs.h — filesystem metadata, whole-file I/O, and directory listing.
 * ===========================================================================
 *
 * The third os-platform primitive. Enumerating a directory and reading file
 * metadata are bucket B: ISO C's <stdio.h> can open a file *by name* but cannot
 * list a directory, tell a file from a directory, or report a size or
 * modification time. Those need the OS — dirent + stat on POSIX, the FindFirstFile
 * / GetFileAttributesEx family on Windows:
 *
 *      operation          macOS / Linux            Windows
 *      ─────────────────  ───────────────────────  ────────────────────────────
 *      metadata           stat()                   GetFileAttributesEx
 *      whole-file read     open/fstat/read          CreateFile/ReadFile
 *      whole-file write    open/write               CreateFile/WriteFile
 *      list a directory    opendir/readdir          FindFirstFile/FindNextFile
 *
 * Whole-file read/write are included here (rather than left to <stdio.h>) so a
 * caller gets one convenient, size-exact, binary-safe call per direction with
 * uniform os_platform error handling — the shape the deferred Rust crates want.
 *
 * PATHS are NUL-terminated C strings in the OS's native encoding (UTF-8 on
 * macOS/Linux; the ANSI code page on Windows — adequate for the ASCII paths the
 * tests and downstream ports use). No path is ever constructed from untrusted
 * concatenation inside this library.
 *
 * BUILD. Compiled by platform-harness. The POSIX backend needs _POSIX_C_SOURCE
 * (from the BUILD) and links no extra library; the Windows backend uses only
 * kernel32. Per-OS source selection is done by the BUILD file.
 */
#ifndef OS_PLATFORM_FS_H
#define OS_PLATFORM_FS_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t, int64_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Metadata about one filesystem entry. */
typedef struct {
    int is_dir;             /* 1 if a directory                              */
    int is_regular;         /* 1 if a regular file (not a directory)         */
    uint64_t size;          /* size in bytes (meaningful for regular files)  */
    int64_t mtime_unix_ns;  /* last-modified time, ns since the UNIX epoch
                             * (second-resolution on POSIX — see fs_posix.c)  */
} osp_file_info;

/*
 * osp_fs_stat — fill *out with metadata for the entry at `path`.
 * Returns OSP_ERR_INVAL if path or out is NULL, OSP_ERR_OS if the entry does not
 * exist or cannot be queried.
 */
osp_status osp_fs_stat(const char *path, osp_file_info *out);

/*
 * osp_fs_exists — 1 if something exists at `path`, else 0 (also 0 if path is
 * NULL). A convenience predicate over osp_fs_stat.
 */
int osp_fs_exists(const char *path);

/*
 * osp_fs_read_file — read the entire file into a freshly malloc'd buffer.
 *
 * On success *out_data points to `*out_len` bytes plus a trailing '\0' (so text
 * files can be used as C strings; the terminator is NOT counted in *out_len).
 * The read is binary-safe — embedded NULs are preserved. The caller must free()
 * *out_data. Returns OSP_ERR_INVAL (NULL args), OSP_ERR_NOMEM (allocation or a
 * file too large for size_t), OSP_ERR_OS (open/read failure or not a regular
 * file).
 */
osp_status osp_fs_read_file(const char *path, unsigned char **out_data,
                            size_t *out_len);

/*
 * osp_fs_write_file — write `len` bytes from `data` to `path`, creating the file
 * or truncating an existing one. `data` may be NULL only if `len` is 0.
 * Returns OSP_ERR_INVAL (NULL path, or NULL data with len > 0), OSP_ERR_OS.
 */
osp_status osp_fs_write_file(const char *path, const unsigned char *data,
                             size_t len);

/*
 * osp_fs_list_dir — call `cb(name, user)` once per entry in directory `path`,
 * skipping "." and "..". `name` is the bare entry name (not a full path) and is
 * valid only for the duration of the callback. Enumeration order is unspecified.
 * Returns OSP_ERR_INVAL (NULL path or cb), OSP_ERR_OS (not a directory / cannot
 * open).
 */
typedef void (*osp_dir_cb)(const char *name, void *user);
osp_status osp_fs_list_dir(const char *path, osp_dir_cb cb, void *user);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_FS_H */
