#include "unix_history.h"
#include <stdint.h>

bool uh_allocate(uh_table table, size_t inode, size_t *fd) {
    if (!inode) return false;
    for (size_t slot = 0; slot < table.capacity; ++slot) {
        if (table.slots[slot].inode == 0) {
            table.slots[slot] = (uh_descriptor){inode, 0};
            *fd = slot;
            return true;
        }
    }
    return false;
}

bool uh_descriptor_at(uh_table table, size_t fd, uh_descriptor *slot) {
    if (fd >= table.capacity || !table.slots[fd].inode) return false;
    *slot = table.slots[fd];
    return true;
}

bool uh_advance(uh_table table, size_t fd, size_t bytes) {
    uh_descriptor slot;
    if (!uh_descriptor_at(table, fd, &slot) || bytes > SIZE_MAX - slot.offset) return false;
    table.slots[fd].offset += bytes;
    return true;
}

bool uh_release(uh_table table, size_t fd) {
    uh_descriptor slot;
    if (!uh_descriptor_at(table, fd, &slot)) return false;
    table.slots[fd] = (uh_descriptor){0, 0};
    return true;
}
