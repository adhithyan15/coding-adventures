/*
 * fs_windows.c — the Win32 backend of os_platform/fs.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use fs_posix.c via
 * the shared `BUILD`). No OS #ifdefs — the build chose this file. Uses only
 * kernel32 (linked by default).
 *
 * We call the ...A (ANSI) entry points explicitly (CreateFileA, FindFirstFileA,
 * …) so the char* paths from the shared header map straight through without
 * defining UNICODE. Reads and writes are chunked because ReadFile/WriteFile take
 * a 32-bit DWORD count, while our sizes are size_t. The modification time is a
 * FILETIME (100-ns ticks since 1601) converted to UNIX nanoseconds exactly as in
 * the clock backend.
 */
#include "os_platform/fs.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdlib.h>
#include <string.h>

/* 100-ns ticks between the 1601 FILETIME epoch and the 1970 UNIX epoch. */
#define OSP_FILETIME_UNIX_EPOCH_DELTA 116444736000000000ULL
/* Per-call transfer cap: a comfortably-large chunk that fits a DWORD. */
#define OSP_IO_CHUNK 0x10000000UL /* 256 MiB */

static int64_t osp__filetime_to_unix_ns(FILETIME ft) {
    uint64_t ticks = ((uint64_t)ft.dwHighDateTime << 32) |
                     (uint64_t)ft.dwLowDateTime;
    return (int64_t)(ticks - OSP_FILETIME_UNIX_EPOCH_DELTA) * 100;
}

osp_status osp_fs_stat(const char *path, osp_file_info *out) {
    WIN32_FILE_ATTRIBUTE_DATA fad;
    int is_dir;
    if (path == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    if (!GetFileAttributesExA(path, GetFileExInfoStandard, &fad)) {
        return OSP_ERR_OS;
    }
    is_dir = (fad.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
    out->is_dir = is_dir;
    out->is_regular = is_dir ? 0 : 1; /* anything that is not a directory */
    out->size = ((uint64_t)fad.nFileSizeHigh << 32) | (uint64_t)fad.nFileSizeLow;
    out->mtime_unix_ns = osp__filetime_to_unix_ns(fad.ftLastWriteTime);
    return OSP_OK;
}

int osp_fs_exists(const char *path) {
    if (path == NULL) {
        return 0;
    }
    return (GetFileAttributesA(path) != INVALID_FILE_ATTRIBUTES) ? 1 : 0;
}

osp_status osp_fs_read_file(const char *path, unsigned char **out_data,
                            size_t *out_len) {
    HANDLE h;
    LARGE_INTEGER sz;
    uint64_t u;
    size_t len;
    size_t off;
    unsigned char *buf;

    if (path == NULL || out_data == NULL || out_len == NULL) {
        return OSP_ERR_INVAL;
    }
    h = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL, NULL);
    if (h == INVALID_HANDLE_VALUE) {
        return OSP_ERR_OS;
    }
    if (!GetFileSizeEx(h, &sz) || sz.QuadPart < 0) {
        CloseHandle(h);
        return OSP_ERR_OS;
    }
    u = (uint64_t)sz.QuadPart;
    len = (size_t)u;
    /* Reject a file too large for size_t (32-bit builds), and guard the +1. */
    if ((uint64_t)len != u || len + 1 == 0) {
        CloseHandle(h);
        return OSP_ERR_NOMEM;
    }
    buf = (unsigned char *)malloc(len + 1);
    if (buf == NULL) {
        CloseHandle(h);
        return OSP_ERR_NOMEM;
    }
    off = 0;
    while (off < len) {
        size_t remain = len - off;
        DWORD want = (remain > OSP_IO_CHUNK) ? (DWORD)OSP_IO_CHUNK
                                             : (DWORD)remain;
        DWORD got = 0;
        if (!ReadFile(h, buf + off, want, &got, NULL)) {
            free(buf);
            CloseHandle(h);
            return OSP_ERR_OS;
        }
        if (got == 0) {
            break; /* EOF earlier than the reported size */
        }
        off += got;
    }
    CloseHandle(h);
    buf[off] = '\0';
    *out_data = buf;
    *out_len = off;
    return OSP_OK;
}

osp_status osp_fs_write_file(const char *path, const unsigned char *data,
                             size_t len) {
    HANDLE h;
    size_t off;

    if (path == NULL || (data == NULL && len > 0)) {
        return OSP_ERR_INVAL;
    }
    h = CreateFileA(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS,
                    FILE_ATTRIBUTE_NORMAL, NULL);
    if (h == INVALID_HANDLE_VALUE) {
        return OSP_ERR_OS;
    }
    off = 0;
    while (off < len) {
        size_t remain = len - off;
        DWORD want = (remain > OSP_IO_CHUNK) ? (DWORD)OSP_IO_CHUNK
                                             : (DWORD)remain;
        DWORD wrote = 0;
        if (!WriteFile(h, data + off, want, &wrote, NULL)) {
            CloseHandle(h);
            return OSP_ERR_OS;
        }
        off += wrote;
    }
    if (!CloseHandle(h)) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

osp_status osp_fs_list_dir(const char *path, osp_dir_cb cb, void *user) {
    size_t plen;
    char *pattern;
    size_t k;
    WIN32_FIND_DATAA fd;
    HANDLE hf;

    if (path == NULL || cb == NULL) {
        return OSP_ERR_INVAL;
    }
    /* Build the search pattern "<path>\*" (FindFirstFile needs a wildcard),
     * inserting a separator only if the path lacks a trailing one. Worst case
     * adds '\' + '*' + '\0' = 3 bytes. */
    plen = strlen(path);
    pattern = (char *)malloc(plen + 3);
    if (pattern == NULL) {
        return OSP_ERR_NOMEM;
    }
    memcpy(pattern, path, plen);
    k = plen;
    if (plen > 0 && pattern[plen - 1] != '\\' && pattern[plen - 1] != '/') {
        pattern[k++] = '\\';
    }
    pattern[k++] = '*';
    pattern[k] = '\0';

    hf = FindFirstFileA(pattern, &fd);
    free(pattern);
    if (hf == INVALID_HANDLE_VALUE) {
        /* An empty match set is reported as ERROR_FILE_NOT_FOUND, which for a
         * valid (but entry-less) directory is success, not an error. */
        return (GetLastError() == ERROR_FILE_NOT_FOUND) ? OSP_OK : OSP_ERR_OS;
    }
    do {
        if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0) {
            continue;
        }
        cb(fd.cFileName, user);
    } while (FindNextFileA(hf, &fd));
    FindClose(hf);
    return OSP_OK;
}
