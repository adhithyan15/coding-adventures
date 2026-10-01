#define _DARWIN_C_SOURCE 1
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <libproc.h>
#include <sandbox.h>
#include <CommonCrypto/CommonDigest.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/event.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define READY_FD 3

static volatile sig_atomic_t sandbox_child = -1;

static void forward_signal(int signal_number) {
    if (sandbox_child > 0) {
        kill((pid_t)sandbox_child, signal_number == SIGUSR2 ? SIGKILL : signal_number);
    }
}

static void watch_supervisor(pid_t supervisor, pid_t plugin) {
    for (int fd = 0; fd < 1024; fd++) close(fd);
    int queue = kqueue();
    if (queue < 0) {
        kill(-plugin, SIGKILL);
        _exit(1);
    }
    struct kevent changes[2];
    EV_SET(&changes[0], supervisor, EVFILT_PROC, EV_ADD | EV_ENABLE, NOTE_EXIT, 0, NULL);
    EV_SET(&changes[1], plugin, EVFILT_PROC, EV_ADD | EV_ENABLE, NOTE_EXIT, 0, NULL);
    if (kevent(queue, changes, 2, NULL, 0, NULL) != 0) {
        kill(-plugin, SIGKILL);
        _exit(1);
    }
    for (;;) {
        struct kevent event;
        if (kevent(queue, NULL, 0, &event, 1, NULL) < 0) {
            if (errno == EINTR) continue;
            kill(-plugin, SIGKILL);
            _exit(1);
        }
        if ((pid_t)event.ident == supervisor) kill(-plugin, SIGKILL);
        _exit(0);
    }
}

static const char *argument(int argc, char **argv, const char *name) {
    size_t length = strlen(name);
    for (int index = 1; index < argc; index++) {
        if (strncmp(argv[index], name, length) == 0 && argv[index][length] == '=') {
            return argv[index] + length + 1;
        }
    }
    return NULL;
}

static int parse_limit(const char *text, rlim_t *value) {
    char *end = NULL;
    errno = 0;
    unsigned long long parsed = strtoull(text == NULL ? "" : text, &end, 10);
    if (errno != 0 || end == text || end == NULL || *end != '\0' || parsed == 0) return -1;
    *value = (rlim_t)parsed;
    return (unsigned long long)*value == parsed ? 0 : -1;
}

static int apply_limit(int resource, rlim_t value, const char *label) {
    struct rlimit current;
    if (getrlimit(resource, &current) != 0) {
        perror(label);
        return -1;
    }
    if (current.rlim_max != RLIM_INFINITY && value > current.rlim_max) value = current.rlim_max;
    struct rlimit limit = { value, value };
    if (setrlimit(resource, &limit) != 0) {
        perror(label);
        return -1;
    }
    return 0;
}

static char *escape_literal(const char *value) {
    size_t length = 0;
    for (const unsigned char *cursor = (const unsigned char *)value; *cursor; cursor++) {
        if (*cursor < 0x20 || *cursor == 0x7f) return NULL;
        length += (*cursor == '\\' || *cursor == '"') ? 2 : 1;
    }
    char *escaped = calloc(length + 1, 1);
    if (escaped == NULL) return NULL;
    char *out = escaped;
    for (const char *cursor = value; *cursor; cursor++) {
        if (*cursor == '\\' || *cursor == '"') *out++ = '\\';
        *out++ = *cursor;
    }
    return escaped;
}

static char *ancestor_literals(const char *value) {
    char path[PATH_MAX];
    if (strlen(value) >= sizeof(path)) return NULL;
    strcpy(path, value);
    char *rules = calloc(PATH_MAX * 8, 1);
    if (rules == NULL) return NULL;
    size_t used = 0;
    for (char *cursor = path + 1;; cursor++) {
        if (*cursor != '/' && *cursor != '\0') continue;
        char saved = *cursor;
        *cursor = '\0';
        const char *literal = path[1] == '\0' ? "/" : path;
        char *escaped = escape_literal(literal);
        if (escaped == NULL) {
            free(rules);
            return NULL;
        }
        int added = snprintf(rules + used, PATH_MAX * 8 - used, " (literal \"%s\")", escaped);
        free(escaped);
        if (added <= 0 || (size_t)added >= PATH_MAX * 8 - used) {
            free(rules);
            return NULL;
        }
        used += (size_t)added;
        *cursor = saved;
        if (saved == '\0') break;
    }
    if (strstr(rules, "(literal \"/\")") == NULL) {
        memmove(rules + 14, rules, used + 1);
        memcpy(rules, " (literal \"/\")", 14);
    }
    return rules;
}

static char *runtime_read_rules(int argc, char **argv) {
    const char *prefix = "--runtime-read-path=";
    size_t prefix_length = strlen(prefix);
    size_t capacity = PATH_MAX * 16;
    char *rules = calloc(capacity, 1);
    if (rules == NULL) return NULL;
    size_t used = 0;
    int count = 0;
    for (int index = 1; index < argc; index++) {
        if (strncmp(argv[index], prefix, prefix_length) != 0) continue;
        const char *path = argv[index] + prefix_length;
        char canonical[PATH_MAX];
        if (++count > 32 || path[0] != '/' || realpath(path, canonical) == NULL) {
            free(rules);
            return NULL;
        }
        char *requested = escape_literal(path);
        char *resolved = escape_literal(canonical);
        if (requested == NULL || resolved == NULL) {
            free(requested);
            free(resolved);
            free(rules);
            return NULL;
        }
        int added = snprintf(
            rules + used,
            capacity - used,
            " (subpath \"%s\") (subpath \"%s\")",
            requested,
            resolved
        );
        free(requested);
        free(resolved);
        if (added <= 0 || (size_t)added >= capacity - used) {
            free(rules);
            return NULL;
        }
        used += (size_t)added;
    }
    return rules;
}

static char *profile_for(
    int argc,
    char **argv,
    const char *working_directory,
    const char *runtime,
    const char *runtime_root_path,
    const char *entry,
    const char *schema
) {
    char *cwd = escape_literal(working_directory);
    char *binary = escape_literal(runtime);
    char *runtime_root = escape_literal(runtime_root_path);
    char *snapshot_entry = escape_literal(entry);
    char *snapshot_schema = schema[0] == '\0' ? NULL : escape_literal(schema);
    char *cwd_ancestors = ancestor_literals(working_directory);
    char *runtime_ancestors = ancestor_literals(runtime);
    char *extra_runtime_rules = runtime_read_rules(argc, argv);
    if (cwd == NULL || binary == NULL || runtime_root == NULL || snapshot_entry == NULL
            || (schema[0] != '\0' && snapshot_schema == NULL)
            || cwd_ancestors == NULL || runtime_ancestors == NULL || extra_runtime_rules == NULL) {
        free(cwd);
        free(binary);
        free(runtime_root);
        free(snapshot_entry);
        free(snapshot_schema);
        free(cwd_ancestors);
        free(runtime_ancestors);
        free(extra_runtime_rules);
        return NULL;
    }
    const char *format =
        "(version 1)\n"
        "(deny default)\n"
        "(import \"system.sb\")\n"
        "(deny file-read* (literal \"/etc/passwd\") (literal \"/private/etc/passwd\"))\n"
        "(deny file-write* (literal \"%s\")%s)\n"
        "(allow process-exec (literal \"%s\"))\n"
        "(deny process-fork)\n"
        "(deny network*)\n"
        "(allow sysctl-read)\n"
        "(allow mach-lookup (global-name \"com.apple.system.opendirectoryd.libinfo\"))\n"
        "(allow file-read-metadata%s%s)\n"
        "(allow file-read* (literal \"%s\") (subpath \"%s\") (subpath \"%s\")%s"
        " (subpath \"/usr/lib\") (subpath \"/System/Library\")"
        " (subpath \"/Library/Apple/System/Library\"))\n"
        "(allow file-write* (subpath \"%s\"))\n";
    char schema_rule[PATH_MAX + 32] = {0};
    if (snapshot_schema != NULL) snprintf(schema_rule, sizeof(schema_rule), " (literal \"%s\")", snapshot_schema);
    size_t size = strlen(format) + strlen(binary) * 2 + strlen(cwd) * 2 + strlen(runtime_root)
        + strlen(snapshot_entry) + strlen(schema_rule)
        + strlen(extra_runtime_rules)
        + strlen(cwd_ancestors) + strlen(runtime_ancestors) + 128;
    char *profile = malloc(size);
    if (profile != NULL) snprintf(
        profile, size, format, snapshot_entry, schema_rule, binary,
        cwd_ancestors, runtime_ancestors, binary, cwd, runtime_root,
        extra_runtime_rules, cwd
    );
    free(cwd);
    free(binary);
    free(runtime_root);
    free(snapshot_entry);
    free(snapshot_schema);
    free(cwd_ancestors);
    free(runtime_ancestors);
    free(extra_runtime_rules);
    return profile;
}

static int write_all(int fd, const char *bytes, size_t length) {
    while (length > 0) {
        ssize_t written = write(fd, bytes, length);
        if (written < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        bytes += written;
        length -= (size_t)written;
    }
    return 0;
}

static uint64_t monotonic_milliseconds(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return UINT64_MAX;
    return (uint64_t)now.tv_sec * 1000 + (uint64_t)now.tv_nsec / 1000000;
}

static int verify_sha256_file(const char *path, const char *expected) {
    if (expected == NULL || strncmp(expected, "sha256:", 7) != 0 || strlen(expected) != 71) return -1;
    int fd = open(path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -1;
    struct stat status;
    CC_SHA256_CTX context;
    unsigned char digest[CC_SHA256_DIGEST_LENGTH];
    unsigned char buffer[16384];
    int result = -1;
    if (fstat(fd, &status) != 0 || !S_ISREG(status.st_mode) || CC_SHA256_Init(&context) != 1) goto done;
    for (;;) {
        ssize_t count = read(fd, buffer, sizeof(buffer));
        if (count < 0) {
            if (errno == EINTR) continue;
            goto done;
        }
        if (count == 0) break;
        if (CC_SHA256_Update(&context, buffer, (CC_LONG)count) != 1) goto done;
    }
    if (CC_SHA256_Final(digest, &context) != 1) goto done;
    char actual[72] = "sha256:";
    for (size_t index = 0; index < sizeof(digest); index++) {
        snprintf(actual + 7 + index * 2, 3, "%02x", digest[index]);
    }
    result = strcmp(actual, expected) == 0 ? 0 : -1;
done:
    close(fd);
    return result;
}

int main(int argc, char **argv) {
    const char *provider = argument(argc, argv, "--provider");
    const char *manifest_hash = argument(argc, argv, "--manifest-hash");
    const char *schema_hash = argument(argc, argv, "--schema-hash");
    const char *entry_hash = argument(argc, argv, "--entry-hash");
    const char *memory_text = argument(argc, argv, "--memory-bytes");
    const char *cpu_text = argument(argc, argv, "--cpu-ms");
    const char *wall_text = argument(argc, argv, "--wall-clock-ms");
    const char *fd_text = argument(argc, argv, "--fd-limit");
    const char *working_directory = argument(argc, argv, "--working-directory");
    const char *runtime = argument(argc, argv, "--runtime");
    const char *runtime_root = argument(argc, argv, "--runtime-root");
    const char *entry = argument(argc, argv, "--entry");
    const char *schema = argument(argc, argv, "--schema");
    const char *stage = argument(argc, argv, "--stage");
    if (provider == NULL || strcmp(provider, "forme-macos-v1") != 0
            || manifest_hash == NULL || schema_hash == NULL || entry_hash == NULL
            || working_directory == NULL || runtime == NULL || runtime_root == NULL
            || entry == NULL || schema == NULL || stage == NULL) {
        fputs("invalid launcher arguments\n", stderr);
        return 64;
    }

    char canonical_cwd[PATH_MAX];
    char canonical_entry[PATH_MAX];
    char canonical_runtime[PATH_MAX];
    char canonical_runtime_root[PATH_MAX];
    char canonical_schema[PATH_MAX] = {0};
    if (realpath(working_directory, canonical_cwd) == NULL
            || realpath(entry, canonical_entry) == NULL
            || realpath(runtime, canonical_runtime) == NULL
            || realpath(runtime_root, canonical_runtime_root) == NULL
            || strncmp(canonical_entry, canonical_cwd, strlen(canonical_cwd)) != 0
            || canonical_entry[strlen(canonical_cwd)] != '/'
            || strncmp(canonical_runtime, canonical_runtime_root, strlen(canonical_runtime_root)) != 0
            || canonical_runtime[strlen(canonical_runtime_root)] != '/') {
        fputs("snapshot containment failed\n", stderr);
        return 65;
    }
    if (strcmp(schema, "-") != 0
            && (realpath(schema, canonical_schema) == NULL
                || strncmp(canonical_schema, canonical_cwd, strlen(canonical_cwd)) != 0
                || canonical_schema[strlen(canonical_cwd)] != '/')) {
        fputs("schema containment failed\n", stderr);
        return 65;
    }

    rlim_t memory = 0, cpu_ms = 0, wall_ms = 0, descriptors = 0;
    if (parse_limit(memory_text, &memory) != 0
            || parse_limit(cpu_text, &cpu_ms) != 0
            || parse_limit(wall_text, &wall_ms) != 0
            || parse_limit(fd_text, &descriptors) != 0) {
        fputs("invalid resource limits\n", stderr);
        return 66;
    }
    int installed[2];
    if (pipe(installed) != 0
            || fcntl(installed[0], F_SETFD, FD_CLOEXEC) != 0
            || fcntl(installed[1], F_SETFD, FD_CLOEXEC) != 0) {
        perror("sandbox readiness pipe");
        return 67;
    }
    pid_t child = fork();
    if (child < 0) {
        perror("initial sandbox fork");
        return 68;
    }
    if (child == 0) {
        close(installed[0]);
        close(READY_FD);
        close(4); /* host-to-supervisor control is Windows-only */
        // Become a session leader before untrusted code starts. With process
        // creation denied, the plugin cannot leave this identity or create a
        // descendant in another session; the supervisor remains its signal
        // and reap authority.
        if (setsid() < 0) _exit(69);
        pid_t plugin = getpid();
        pid_t supervisor = getppid();
        pid_t watcher = fork();
        if (watcher < 0) _exit(69);
        if (watcher == 0) watch_supervisor(supervisor, plugin);
        rlim_t cpu_seconds = (cpu_ms + 999) / 1000;
        if (apply_limit(RLIMIT_CPU, cpu_seconds, "setrlimit cpu") != 0
                || apply_limit(RLIMIT_NOFILE, descriptors, "setrlimit descriptors") != 0) {
            _exit(69);
        }
        if (chdir(canonical_cwd) != 0) {
            perror("chdir");
            _exit(70);
        }
        char *profile = profile_for(
            argc, argv, canonical_cwd, canonical_runtime, canonical_runtime_root,
            canonical_entry, canonical_schema
        );
        char *sandbox_error = NULL;
        if (profile == NULL || sandbox_init(profile, 0, &sandbox_error) != 0) {
            fprintf(stderr, "sandbox_init: %s\n", sandbox_error == NULL ? "failed" : sandbox_error);
            if (sandbox_error != NULL) sandbox_free_error(sandbox_error);
            free(profile);
            _exit(71);
        }
        free(profile);
        if (verify_sha256_file(canonical_entry, entry_hash) != 0) {
            fputs("staged entry identity changed\n", stderr);
            _exit(72);
        }
        if (strcmp(schema_hash, "-") != 0
                && verify_sha256_file(canonical_schema, schema_hash) != 0) {
            fputs("staged schema identity changed\n", stderr);
            _exit(72);
        }
        if (write_all(installed[1], "1", 1) != 0) _exit(72);
        execl(canonical_runtime, canonical_runtime, canonical_entry, stage, schema_hash, (char *)NULL);
        write_all(installed[1], "0", 1);
        perror("exec runtime");
        _exit(73);
    }

    sandbox_child = child;
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = forward_signal;
    sigemptyset(&action.sa_mask);
    for (int signal_number = 1; signal_number < NSIG; signal_number++) {
        if (signal_number != SIGKILL && signal_number != SIGSTOP && signal_number != SIGCHLD) {
            sigaction(signal_number, &action, NULL);
        }
    }
    close(installed[1]);
    char installed_byte = 0;
    if (read(installed[0], &installed_byte, 1) != 1 || installed_byte != '1'
            || read(installed[0], &installed_byte, 1) != 0) {
        close(installed[0]);
        waitpid(child, NULL, 0);
        fputs("child did not install sandbox\n", stderr);
        return 74;
    }
    close(installed[0]);
    if (fcntl(READY_FD, F_SETFD, FD_CLOEXEC) != 0) {
        perror("readiness fcntl");
        kill(child, SIGKILL);
        waitpid(child, NULL, 0);
        return 75;
    }
    char readiness[2048];
    int readiness_length = snprintf(
        readiness,
        sizeof(readiness),
        "{\"protocol\":1,\"provider\":\"forme-macos-v1\","
        "\"manifestHash\":\"%s\",\"configSchemaHash\":%s%s%s,"
        "\"entryHash\":\"%s\"}\n",
        manifest_hash,
        strcmp(schema_hash, "-") == 0 ? "" : "\"",
        strcmp(schema_hash, "-") == 0 ? "null" : schema_hash,
        strcmp(schema_hash, "-") == 0 ? "" : "\"",
        entry_hash
    );
    if (readiness_length <= 0 || (size_t)readiness_length >= sizeof(readiness)
            || write_all(READY_FD, readiness, (size_t)readiness_length) != 0
            || close(READY_FD) != 0) {
        perror("readiness write");
        kill(child, SIGKILL);
        waitpid(child, NULL, 0);
        return 76;
    }

    int status = 0;
    uint64_t started = monotonic_milliseconds();
    if (started == UINT64_MAX) {
        kill(child, SIGKILL);
        waitpid(child, &status, 0);
        return 77;
    }
    for (;;) {
        pid_t waited = waitpid(child, &status, WNOHANG);
        if (waited == child) break;
        if (waited < 0 && errno != EINTR) {
            perror("waitpid");
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            return 77;
        }
        uint64_t now = monotonic_milliseconds();
        if (now == UINT64_MAX || now - started > (uint64_t)wall_ms) {
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            return 124;
        }
        struct proc_taskinfo task;
        int bytes = proc_pidinfo(child, PROC_PIDTASKINFO, 0, &task, sizeof(task));
        if (bytes != sizeof(task)) {
            for (int retry = 0; retry < 10; retry++) {
                waited = waitpid(child, &status, WNOHANG);
                if (waited == child) goto child_exited;
                usleep(1000);
            }
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            return 77;
        }
        if (task.pti_resident_size > (uint64_t)memory) {
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            return 137;
        }
        usleep(10000);
    }
child_exited:
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 78;
}
