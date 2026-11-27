#ifndef UNIX_HISTORY_H
#define UNIX_HISTORY_H

#include <stdbool.h>
#include <stddef.h>

typedef enum { UH_OK, UH_INVALID, UH_MISSING, UH_NOT_DIR, UH_IS_DIR, UH_FULL, UH_BAD_FD } uh_error;
typedef struct { const char *name; size_t inode; } uh_entry;
typedef struct {
    const uh_entry *entries;
    size_t entry_count;
    size_t parent;
    const char *data;
    size_t length;
} uh_node;
typedef struct { const uh_node *nodes; size_t count; } uh_image;
typedef struct { size_t inode, offset; } uh_descriptor;
typedef struct { uh_descriptor *slots; size_t capacity; } uh_table;
typedef struct { uh_image image; uh_table table; } uh_kernel;
typedef struct { const char *data; size_t count; } uh_bytes;

extern const uh_image uh_demo_image;

const char *uh_error_name(uh_error error);
const uh_node *uh_node_at(uh_image image, size_t inode);
uh_error uh_resolve(uh_image image, const char *path, size_t length, size_t *inode);
uh_error uh_read_node(uh_image image, size_t inode, size_t offset, size_t count, uh_bytes *bytes);

bool uh_allocate(uh_table table, size_t inode, size_t *fd);
bool uh_descriptor_at(uh_table table, size_t fd, uh_descriptor *slot);
bool uh_advance(uh_table table, size_t fd, size_t bytes);
bool uh_release(uh_table table, size_t fd);

uh_error uh_open(uh_kernel kernel, const char *path, size_t *fd, size_t *inode);
uh_error uh_read(uh_kernel kernel, size_t fd, size_t count, uh_descriptor *before, uh_bytes *bytes);
uh_error uh_close(uh_kernel kernel, size_t fd);
uh_error uh_list(uh_kernel kernel, const char *path, size_t *inode, const uh_node **directory);

#endif
