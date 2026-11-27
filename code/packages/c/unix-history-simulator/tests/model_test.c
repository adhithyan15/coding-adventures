#include "unix_history.h"
#include <assert.h>
#include <string.h>

int main(void) {
    uh_descriptor slots[2] = {{0}};
    uh_table table = {slots, 2};
    uh_descriptor other;
    size_t fd;
    assert(uh_allocate(table, 2, &fd) && fd == 0);
    assert(uh_allocate(table, 2, &fd) && fd == 1);
    assert(!uh_allocate(table, 2, &fd));
    assert(uh_advance(table, 0, 4));
    assert(uh_descriptor_at(table, 1, &other) && other.offset == 0);
    assert(!uh_release(table, 2));
    assert(uh_release(table, 0));
    assert(uh_allocate(table, 3, &fd) && fd == 0 && slots[0].offset == 0);
    const char *path = "//etc/../etc/./notes";
    size_t inode;
    assert(uh_resolve(uh_demo_image, path, strlen(path), &inode) == UH_OK && inode == 5);
    assert(uh_resolve(uh_demo_image, "/hello/..", 9, &inode) == UH_NOT_DIR);
    assert(uh_resolve(uh_demo_image, "a\0b", 3, &inode) == UH_INVALID);

    uh_descriptor kernel_slots[2] = {{0}};
    uh_kernel kernel = {uh_demo_image, {kernel_slots, 2}};
    size_t kernel_fd;
    assert(uh_open(kernel, "/hello", &kernel_fd, &inode) == UH_OK);
    assert(kernel_fd == 0 && inode == 2);
    uh_descriptor before;
    uh_bytes bytes;
    assert(uh_read(kernel, kernel_fd, 5, &before, &bytes) == UH_OK);
    assert(before.offset == 0 && bytes.count == 5 && memcmp(bytes.data, "Hello", 5) == 0);
    assert(kernel_slots[kernel_fd].offset == 5);
    assert(uh_close(kernel, kernel_fd) == UH_OK);
    assert(uh_read(kernel, kernel_fd, 1, &before, &bytes) == UH_BAD_FD);
    return 0;
}
