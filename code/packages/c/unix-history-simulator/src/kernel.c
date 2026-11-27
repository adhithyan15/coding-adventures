#include "unix_history.h"
#include <string.h>

const char *uh_error_name(uh_error error) {
    switch (error) {
        case UH_OK: return "";
        case UH_INVALID: return "EINVAL";
        case UH_MISSING: return "ENOENT";
        case UH_NOT_DIR: return "ENOTDIR";
        case UH_IS_DIR: return "EISDIR";
        case UH_FULL: return "EMFILE";
        case UH_BAD_FD: return "EBADF";
    }
    return "EINVAL";
}

uh_error uh_open(uh_kernel kernel, const char *path, size_t *fd, size_t *inode) {
    uh_error error = uh_resolve(kernel.image, path, strlen(path), inode);
    if (error != UH_OK) return error;
    if (!uh_node_at(kernel.image, *inode)->data) return UH_IS_DIR;
    return uh_allocate(kernel.table, *inode, fd) ? UH_OK : UH_FULL;
}

uh_error uh_read(uh_kernel kernel, size_t fd, size_t count, uh_descriptor *before, uh_bytes *bytes) {
    if (!uh_descriptor_at(kernel.table, fd, before)) return UH_BAD_FD;
    uh_error error = uh_read_node(kernel.image, before->inode, before->offset, count, bytes);
    if (error != UH_OK) return error;
    return uh_advance(kernel.table, fd, bytes->count) ? UH_OK : UH_INVALID;
}

uh_error uh_close(uh_kernel kernel, size_t fd) {
    return uh_release(kernel.table, fd) ? UH_OK : UH_BAD_FD;
}

uh_error uh_list(uh_kernel kernel, const char *path, size_t *inode, const uh_node **directory) {
    uh_error error = uh_resolve(kernel.image, path, strlen(path), inode);
    if (error != UH_OK) return error;
    *directory = uh_node_at(kernel.image, *inode);
    return (*directory)->data ? UH_NOT_DIR : UH_OK;
}
