#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/mount.h>
#include <sys/types.h>
#include <unistd.h>

static int denied(void) { return errno == EPERM || errno == EACCES || errno == ENOENT; }
int main(int argc, char **argv) {
    const char *probe = argc > 2 ? argv[2] : "";
    if (strcmp(probe, "filesystem") == 0) {
        int fd = open("/etc/passwd", O_RDONLY);
        if (fd >= 0) close(fd);
        return fd < 0 && denied() ? 0 : 70;
    }
    if (strcmp(probe, "network") == 0) {
        int fd = socket(AF_INET, SOCK_STREAM, 0);
        if (fd < 0) return denied() ? 0 : 71;
        close(fd);
        return 71;
    }
    if (strcmp(probe, "process") == 0) {
        pid_t child = fork();
        if (child == 0) _exit(72);
        return child < 0 && denied() ? 0 : 73;
    }
    if (strcmp(probe, "memory") == 0) {
        size_t size = (size_t)1024 * 1024 * 1024;
        volatile unsigned char *bytes = malloc(size);
        if (bytes == NULL) return 0;
        for (size_t offset = 0; offset < size; offset += 4096) bytes[offset] = 1;
        return 74;
    }
    if (strcmp(probe, "descriptors") == 0) {
        for (int index = 0; index < 4096; index++) {
            char path[64];
            snprintf(path, sizeof(path), "fd-%d", index);
            int fd = open(path, O_CREAT | O_WRONLY, 0600);
            if (fd < 0) return errno == EMFILE ? 0 : 75;
        }
        return 76;
    }
    if (strcmp(probe, "environment") == 0) {
        return getenv("FORME_SANDBOX_AMBIENT_SENTINEL") == NULL ? 0 : 78;
    }
    if (strcmp(probe, "cpu") == 0) {
        volatile unsigned long long value = 0;
        for (;;) value++;
    }
    if (strcmp(probe, "wall-clock") == 0) {
        sleep(10);
        return 79;
    }
    if (strcmp(probe, "mount") == 0) {
        int result = mount("tmpfs", ".", "tmpfs", 0, "size=1m");
        return result < 0 && denied() ? 0 : 80;
    }
    return 77;
}
