#include "unix_history.h"
#include <string.h>

static const uh_entry root[] = {{"hello", 2}, {"empty", 3}, {"etc", 4}};
static const uh_entry etc[] = {{"notes", 5}};
static const char hello[] = "Hello, Unix!\n";
static const char notes[] = "A directory maps a name to an inode.\n"
                            "An open descriptor tracks a byte offset.\n";
static const uh_node nodes[] = {
    {root, 3, 1, NULL, 0}, {NULL, 0, 0, hello, sizeof hello - 1},
    {NULL, 0, 0, "", 0}, {etc, 1, 1, NULL, 0},
    {NULL, 0, 0, notes, sizeof notes - 1}
};
const uh_image uh_demo_image = {nodes, sizeof nodes / sizeof nodes[0]};

const uh_node *uh_node_at(uh_image image, size_t inode) {
    return inode > 0 && inode <= image.count ? &image.nodes[inode - 1] : NULL;
}

static bool equal_word(const char *text, size_t length, const char *word) {
    return strlen(word) == length && memcmp(text, word, length) == 0;
}

static size_t find_entry(const uh_node *node, const char *text, size_t length) {
    for (size_t i = 0; i < node->entry_count; ++i) {
        if (equal_word(text, length, node->entries[i].name)) return node->entries[i].inode;
    }
    return 0;
}

/* Walk the actual directory entries before interpreting a later "..". */
uh_error uh_resolve(uh_image image, const char *path, size_t length, size_t *inode) {
    if (length == 0 || memchr(path, '\0', length)) return UH_INVALID;

    size_t current = 1;
    size_t position = 0;
    while (position < length) {
        if (path[position] == '/') {
            ++position;
            continue;
        }
        const uh_node *node = uh_node_at(image, current);
        if (!node) return UH_MISSING;
        if (node->data) return UH_NOT_DIR;

        size_t start = position;
        while (position < length && path[position] != '/') ++position;
        size_t width = position - start;
        const char *component = path + start;
        if (equal_word(component, width, "..")) current = node->parent;
        else if (!equal_word(component, width, ".")) current = find_entry(node, component, width);
        if (!current) return UH_MISSING;
    }
    const uh_node *node = uh_node_at(image, current);
    if (!node) return UH_MISSING;
    if (path[length - 1] == '/' && node->data) return UH_NOT_DIR;
    *inode = current;
    return UH_OK;
}

uh_error uh_read_node(uh_image image, size_t inode, size_t offset, size_t count, uh_bytes *bytes) {
    const uh_node *node = uh_node_at(image, inode);
    if (!node) return UH_MISSING;
    if (!node->data) return UH_IS_DIR;
    size_t start = offset < node->length ? offset : node->length;
    size_t remaining = node->length - start;
    *bytes = (uh_bytes){node->data + start, count < remaining ? count : remaining};
    return UH_OK;
}
