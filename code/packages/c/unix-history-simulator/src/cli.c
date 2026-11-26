#include "unix_history.h"
#include <stdio.h>
#include <string.h>

static void quote(const char *text) {
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)text; *p; ++p) {
        if (*p < 32 || *p == '"' || *p == '\\') printf("\\u%04x", (unsigned)*p);
        else putchar(*p);
    }
    putchar('"');
}

static void path_event(const char *call, const char *path) {
    printf("{\"call\":\"%s\",\"path\":", call);
    quote(path);
}

static void event_error(char *output, size_t capacity, uh_error error) {
    const char *name = uh_error_name(error);
    printf(",\"error\":\"%s\"}", name);
    size_t end = strlen(output);
    snprintf(output + end, capacity - end, "error: %s\n", name);
}

/* The CLI uses only the small fixed demo image. The streaming kernel API
   above does not impose the demo's console buffer limit. */
static void cat(uh_kernel kernel, const char *path, char *output, size_t capacity) {
    size_t fd, inode;
    uh_error error = uh_open(kernel, path, &fd, &inode);
    path_event("open", path);
    if (error != UH_OK) {
        event_error(output, capacity, error);
        return;
    }
    printf(",\"fd\":%zu,\"inode\":%zu,\"offset\":0}", fd, inode);
    size_t written = 0;
    for (;;) {
        uh_descriptor before;
        uh_bytes bytes;
        error = uh_read(kernel, fd, 8, &before, &bytes);
        printf(",{\"call\":\"read\",\"fd\":%zu", fd);
        if (error != UH_OK) {
            event_error(output, capacity, error);
            break;
        }
        printf(",\"inode\":%zu,\"offset\":%zu,\"bytes\":%zu,\"nextOffset\":%zu}",
               before.inode, before.offset, bytes.count, before.offset + bytes.count);
        if (!bytes.count) break;
        if (bytes.count >= capacity - written) break;
        memcpy(output + written, bytes.data, bytes.count);
        written += bytes.count;
        output[written] = '\0';
    }
    error = uh_close(kernel, fd);
    printf(",{\"call\":\"close\",\"fd\":%zu", fd);
    if (error != UH_OK) event_error(output, capacity, error);
    else putchar('}');
}

static void list(uh_kernel kernel, const char *path, char *output, size_t capacity) {
    size_t inode;
    const uh_node *node;
    uh_error error = uh_list(kernel, path, &inode, &node);
    path_event("list", path);
    if (error != UH_OK) {
        event_error(output, capacity, error);
        return;
    }
    printf(",\"inode\":%zu,\"names\":[", inode);
    for (size_t i = 0; i < node->entry_count; ++i) {
        const char *name = node->entries[i].name;
        if (i) putchar(',');
        quote(name);
        size_t end = strlen(output);
        snprintf(output + end, capacity - end, "%s\n", name);
    }
    printf("]}");
}

int main(int argc, char **argv) {
    char default_command[] = "cat /hello";
    if (argc > 2) {
        fputs("usage: unix-history \"COMMAND\"\n", stderr);
        return 2;
    }
    char *line = argc == 2 ? argv[1] : default_command;
    const char *delimiters = " \t\n\r\f\v";
    char *name = strtok(line, delimiters);
    char *path = strtok(NULL, delimiters);
    char *extra = strtok(NULL, delimiters);
    uh_descriptor slots[4] = {{0}};
    uh_kernel kernel = {uh_demo_image, {slots, 4}};
    char output[128] = {0};

    printf("{\"events\":[");
    if (name && strcmp(name, "cat") == 0 && path && !extra) cat(kernel, path, output, sizeof output);
    else if (name && strcmp(name, "ls") == 0 && !extra) list(kernel, path ? path : "/", output, sizeof output);
    else if (name) snprintf(output, sizeof output, "usage: cat PATH or ls [PATH]\n");
    printf("],\"output\":");
    quote(output);
    puts("}");
    return 0;
}
