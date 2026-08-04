/*
 * fs_posix.c — the POSIX backend of os_platform/fs (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * fs_windows.c via `BUILD_windows`). No OS #ifdefs — the build chose this file.
 * Uses only libc (stat/open/read/write/opendir), so no extra library is linked;
 * _POSIX_C_SOURCE (from the BUILD) exposes the declarations.
 *
 * The two subtleties worth calling out:
 *   - mtime is reported in SECONDS scaled to nanoseconds. The sub-second field
 *     spells differently across systems (st_mtim on Linux, st_mtimespec on
 *     macOS), and this file must compile on both, so it uses the portable
 *     st_mtime (whole seconds) rather than an #ifdef per OS.
 *   - read()/write() can transfer fewer bytes than asked and can be interrupted
 *     by a signal (EINTR); both are looped to completion.
 */
#include "os_platform/fs.h"

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define OSP_NS_PER_SEC 1000000000LL

osp_status osp_fs_stat(const char *path, osp_file_info *out) {
    struct stat st;
    if (path == NULL || out == NULL) {
        return OSP_ERR_INVAL;
    }
    if (stat(path, &st) != 0) {
        return OSP_ERR_OS;
    }
    out->is_dir = S_ISDIR(st.st_mode) ? 1 : 0;
    out->is_regular = S_ISREG(st.st_mode) ? 1 : 0;
    out->size = (st.st_size > 0) ? (uint64_t)st.st_size : 0;
    out->mtime_unix_ns = (int64_t)st.st_mtime * OSP_NS_PER_SEC;
    return OSP_OK;
}

int osp_fs_exists(const char *path) {
    struct stat st;
    if (path == NULL) {
        return 0;
    }
    return (stat(path, &st) == 0) ? 1 : 0;
}

osp_status osp_fs_read_file(const char *path, unsigned char **out_data,
                            size_t *out_len) {
    int fd;
    struct stat st;
    size_t len;
    size_t off;
    unsigned char *buf;

    if (path == NULL || out_data == NULL || out_len == NULL) {
        return OSP_ERR_INVAL;
    }
    fd = open(path, O_RDONLY);
    if (fd < 0) {
        return OSP_ERR_OS;
    }
    /* fstat the OPEN descriptor (not the path) so the size we allocate matches
     * the file we will actually read — no TOCTOU gap between two path lookups. */
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode) || st.st_size < 0) {
        close(fd);
        return OSP_ERR_OS;
    }
    len = (size_t)st.st_size;
    /* On a 32-bit size_t a large file would truncate; reject rather than
     * under-allocate. Also guard the len+1 terminator against wraparound. */
    if ((off_t)len != st.st_size || len + 1 == 0) {
        close(fd);
        return OSP_ERR_NOMEM;
    }
    buf = (unsigned char *)malloc(len + 1);
    if (buf == NULL) {
        close(fd);
        return OSP_ERR_NOMEM;
    }
    off = 0;
    while (off < len) {
        ssize_t n = read(fd, buf + off, len - off);
        if (n < 0) {
            if (errno == EINTR) {
                continue; /* interrupted before any byte — retry */
            }
            free(buf);
            close(fd);
            return OSP_ERR_OS;
        }
        if (n == 0) {
            break; /* EOF: the file shrank since fstat; return what we read */
        }
        off += (size_t)n;
    }
    close(fd);
    buf[off] = '\0';
    *out_data = buf;
    *out_len = off;
    return OSP_OK;
}

osp_status osp_fs_write_file(const char *path, const unsigned char *data,
                             size_t len) {
    int fd;
    size_t off;

    if (path == NULL || (data == NULL && len > 0)) {
        return OSP_ERR_INVAL;
    }
    fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        return OSP_ERR_OS;
    }
    off = 0;
    while (off < len) {
        ssize_t n = write(fd, data + off, len - off);
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
            close(fd);
            return OSP_ERR_OS;
        }
        off += (size_t)n;
    }
    /* close() can surface a deferred write error (e.g. ENOSPC), so check it. */
    if (close(fd) != 0) {
        return OSP_ERR_OS;
    }
    return OSP_OK;
}

osp_status osp_fs_list_dir(const char *path, osp_dir_cb cb, void *user) {
    DIR *d;
    struct dirent *ent;

    if (path == NULL || cb == NULL) {
        return OSP_ERR_INVAL;
    }
    d = opendir(path);
    if (d == NULL) {
        return OSP_ERR_OS;
    }
    while ((ent = readdir(d)) != NULL) {
        if (strcmp(ent->d_name, ".") == 0 || strcmp(ent->d_name, "..") == 0) {
            continue;
        }
        cb(ent->d_name, user);
    }
    closedir(d);
    return OSP_OK;
}
