#define _GNU_SOURCE 1
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <linux/audit.h>
#include <linux/capability.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include "sha256.h"
#include <linux/securebits.h>
#include <sched.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define READY_FD 3
#define DENIED (SECCOMP_RET_ERRNO | (EPERM & SECCOMP_RET_DATA))

static volatile sig_atomic_t sandbox_child = -1;
static char original_cgroup_procs[PATH_MAX];
static const char *root_setup_stage = "not-started";

static void forward_signal(int signal_number) {
    if (sandbox_child > 0) {
        kill((pid_t)sandbox_child, signal_number == SIGUSR2 ? SIGKILL : signal_number);
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
    int file = open(path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    int result = -1;
    struct stat status;
    if (file < 0 || fstat(file, &status) != 0 || !S_ISREG(status.st_mode)) goto done;
    sha256_ctx context;
    sha256_init(&context);
    unsigned char buffer[16384];
    ssize_t count;
    for (;;) {
        do { count = read(file, buffer, sizeof(buffer)); } while (count < 0 && errno == EINTR);
        if (count < 0) goto done;
        if (count == 0) break;
        sha256_update(&context, buffer, (size_t)count);
    }
    unsigned char digest[32];
    sha256_final(&context, digest);
    char actual[72] = "sha256:";
    for (size_t index = 0; index < sizeof(digest); index++) {
        snprintf(actual + 7 + index * 2, 3, "%02x", digest[index]);
    }
    result = strcmp(actual, expected) == 0 ? 0 : -1;
done:
    if (file >= 0) close(file);
    return result;
}

static int drop_namespace_root_authority(void) {
    unsigned long securebits = SECBIT_NOROOT | SECBIT_NOROOT_LOCKED
        | SECBIT_NO_SETUID_FIXUP | SECBIT_NO_SETUID_FIXUP_LOCKED;
    if (prctl(PR_SET_SECUREBITS, securebits, 0, 0, 0) != 0) return -1;
    struct __user_cap_header_struct header;
    struct __user_cap_data_struct data[2];
    memset(&header, 0, sizeof(header));
    memset(data, 0, sizeof(data));
    header.version = _LINUX_CAPABILITY_VERSION_3;
    header.pid = 0;
    return (int)syscall(SYS_capset, &header, data);
}

static int write_text(const char *path, const char *text) {
    int fd = open(path, O_WRONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    int result = write_all(fd, text, strlen(text));
    int saved = errno;
    close(fd);
    errno = saved;
    return result;
}

static int capture_original_cgroup(const char *canonical_base) {
    FILE *memberships = fopen("/proc/self/cgroup", "r");
    if (memberships == NULL) return -1;
    char line[PATH_MAX];
    char relative[PATH_MAX] = {0};
    while (fgets(line, sizeof(line), memberships) != NULL) {
        if (strncmp(line, "0::", 3) != 0) continue;
        char *newline = strchr(line + 3, '\n');
        if (newline != NULL) *newline = '\0';
        if (line[3] != '/' || strstr(line + 3, "..") != NULL
                || strlen(line + 3) >= sizeof(relative)) break;
        strcpy(relative, line + 3);
        break;
    }
    int saved = errno;
    fclose(memberships);
    errno = saved;
    if (relative[0] == '\0') {
        errno = EINVAL;
        return -1;
    }

    char current[PATH_MAX];
    char canonical_current[PATH_MAX];
    if (snprintf(current, sizeof(current), "/sys/fs/cgroup%s", relative) >= (int)sizeof(current)
            || realpath(current, canonical_current) == NULL) return -1;
    size_t base_length = strlen(canonical_base);
    if (strncmp(canonical_current, canonical_base, base_length) != 0
            || canonical_current[base_length] != '/') {
        errno = EPERM;
        return -1;
    }
    if (snprintf(original_cgroup_procs, sizeof(original_cgroup_procs),
            "%s/cgroup.procs", canonical_current) >= (int)sizeof(original_cgroup_procs)
            || access(original_cgroup_procs, W_OK) != 0) return -1;
    return 0;
}

static int setup_cgroup(const char *base, rlim_t memory, char *group, size_t group_size) {
    char canonical[PATH_MAX];
    if (base == NULL || base[0] != '/' || strstr(base, "..") != NULL || realpath(base, canonical) == NULL) {
        errno = EINVAL;
        return -1;
    }
    if (capture_original_cgroup(canonical) != 0) return -1;
    if (snprintf(group, group_size, "%s/plugin-%ld", canonical, (long)getpid()) >= (int)group_size
            || mkdir(group, 0700) != 0) return -1;
    char path[PATH_MAX];
    char value[64];
    snprintf(path, sizeof(path), "%s/memory.max", group);
    snprintf(value, sizeof(value), "%llu", (unsigned long long)memory);
    if (write_text(path, value) != 0) return -1;
    snprintf(path, sizeof(path), "%s/memory.swap.max", group);
    if (access(path, F_OK) == 0 && write_text(path, "0") != 0) return -1;
    snprintf(path, sizeof(path), "%s/pids.max", group);
    /* Threads count against pids.max; seccomp separately denies process clones. */
    if (write_text(path, "64") != 0) return -1;
    snprintf(path, sizeof(path), "%s/cpu.max", group);
    if (write_text(path, "100000 100000") != 0) return -1;
    snprintf(path, sizeof(path), "%s/cgroup.procs", group);
    snprintf(value, sizeof(value), "%ld", (long)getpid());
    return write_text(path, value);
}

static int cleanup_cgroup(const char *group) {
    if (group == NULL || *group == '\0') return 0;
    char value[64];
    snprintf(value, sizeof(value), "%ld", (long)getpid());
    int result = original_cgroup_procs[0] == '\0'
        ? -1
        : write_text(original_cgroup_procs, value);
    if (rmdir(group) != 0) result = -1;
    return result;
}

static int write_id_map(const char *name, unsigned long outside) {
    char path[64];
    char value[128];
    snprintf(path, sizeof(path), "/proc/self/%s", name);
    snprintf(value, sizeof(value), "0 %lu 1\n", outside);
    return write_text(path, value);
}

static int enter_namespaces(uid_t uid, gid_t gid) {
    if (unshare(CLONE_NEWUSER) != 0) return -1;
    if (write_text("/proc/self/setgroups", "deny\n") != 0
            || write_id_map("uid_map", (unsigned long)uid) != 0
            || write_id_map("gid_map", (unsigned long)gid) != 0
            || setresgid(0, 0, 0) != 0
            || setresuid(0, 0, 0) != 0) return -1;
    return unshare(CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWIPC | CLONE_NEWUTS | CLONE_NEWPID);
}

static int make_directory(const char *path, mode_t mode) {
    if (mkdir(path, mode) == 0 || errno == EEXIST) return 0;
    return -1;
}

static int make_parent_directories(char *path) {
    for (char *cursor = path + 1; *cursor; cursor++) {
        if (*cursor != '/') continue;
        *cursor = '\0';
        if (make_directory(path, 0755) != 0) return -1;
        *cursor = '/';
    }
    return 0;
}

static int bind_read_only(const char *source, const char *target) {
    struct stat status;
    if (stat(source, &status) != 0) return errno == ENOENT ? 0 : -1;
    char mutable_target[PATH_MAX];
    if (strlen(target) >= sizeof(mutable_target)) return -1;
    strcpy(mutable_target, target);
    if (make_parent_directories(mutable_target) != 0) return -1;
    if (S_ISDIR(status.st_mode)) {
        if (make_directory(target, 0755) != 0) return -1;
    } else {
        int fd = open(target, O_CREAT | O_RDONLY | O_CLOEXEC, 0400);
        if (fd < 0) return -1;
        close(fd);
    }
    unsigned long bind_flags = MS_BIND | (S_ISDIR(status.st_mode) ? MS_REC : 0);
    if (mount(source, target, NULL, bind_flags, NULL) != 0) return -1;
    return mount(NULL, target, NULL, MS_BIND | MS_REMOUNT | MS_RDONLY | MS_NOSUID | MS_NODEV, NULL);
}

static int setup_root(
    const char *working_directory,
    const char *runtime,
    const char *runtime_root,
    const char *entry,
    char *root,
    size_t root_size,
    char *sandbox_runtime,
    size_t runtime_size,
    char *sandbox_entry,
    size_t entry_size
) {
    root_setup_stage = "create-root-directory";
    char template[] = "/tmp/forme-sandbox-root-XXXXXX";
    char *created = mkdtemp(template);
    if (created == NULL || strlen(created) >= root_size) return -1;
    strcpy(root, created);
    root_setup_stage = "mount-private-tmpfs";
    if (mount("tmpfs", root, "tmpfs", MS_NOSUID | MS_NODEV, "mode=0755,size=32m") != 0) return -1;

    char target[PATH_MAX];
    snprintf(target, sizeof(target), "%s/work", root);
    root_setup_stage = "bind-plugin-work-directory";
    if (make_directory(target, 0700) != 0
            || mount(working_directory, target, NULL, MS_BIND | MS_REC, NULL) != 0
            || mount(NULL, target, NULL, MS_BIND | MS_REMOUNT | MS_NOSUID | MS_NODEV, NULL) != 0) return -1;
    char snapshot_source[PATH_MAX];
    if (snprintf(snapshot_source, sizeof(snapshot_source), "%s/.forme-snapshot", working_directory)
            >= (int)sizeof(snapshot_source)) return -1;
    snprintf(target, sizeof(target), "%s/work/.forme-snapshot", root);
    root_setup_stage = "remount-snapshot-read-only";
    if (bind_read_only(snapshot_source, target) != 0) return -1;
    snprintf(target, sizeof(target), "%s/proc", root);
    root_setup_stage = "create-proc-mountpoint";
    if (make_directory(target, 0555) != 0) return -1;
    snprintf(target, sizeof(target), "%s/dev", root);
    root_setup_stage = "create-device-mountpoint";
    if (make_directory(target, 0555) != 0) return -1;
    const char *libraries[] = { "/lib", "/lib64", "/usr/lib", "/usr/lib64", NULL };
    for (size_t index = 0; libraries[index] != NULL; index++) {
        snprintf(target, sizeof(target), "%s%s", root, libraries[index]);
        root_setup_stage = "bind-system-library-read-only";
        if (bind_read_only(libraries[index], target) != 0) return -1;
    }

    const char *relative_entry = entry + strlen(working_directory);
    if (*relative_entry != '/') return -1;
    if (snprintf(sandbox_entry, entry_size, "/work%s", relative_entry) >= (int)entry_size) return -1;
    if (strcmp(runtime, entry) == 0) {
        if (strlen(sandbox_entry) >= runtime_size) return -1;
        strcpy(sandbox_runtime, sandbox_entry);
    } else {
        const char *relative_runtime = runtime + strlen(runtime_root);
        snprintf(target, sizeof(target), "%s/runtime", root);
        root_setup_stage = "bind-runtime-read-only";
        if (bind_read_only(runtime_root, target) != 0
                || snprintf(sandbox_runtime, runtime_size, "/runtime%s", relative_runtime) >= (int)runtime_size) return -1;
    }
    return 0;
}

static int install_seccomp(void) {
#if defined(__x86_64__)
    const uint32_t architecture = AUDIT_ARCH_X86_64;
#elif defined(__aarch64__)
    const uint32_t architecture = AUDIT_ARCH_AARCH64;
#else
#error "forme-sandbox-linux supports x86_64 and aarch64"
#endif
#define LOAD_NR BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr))
#define ALLOW_NR(number) BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (number), 0, 1), BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW)
#define ERRNO_NR(number, error_number) BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, (number), 0, 1), BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | ((error_number) & SECCOMP_RET_DATA))
    struct sock_filter filter[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, architecture, 1, 0),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
        LOAD_NR,
#ifdef SYS_read
        ALLOW_NR(SYS_read),
#endif
#ifdef SYS_write
        ALLOW_NR(SYS_write),
#endif
#ifdef SYS_readv
        ALLOW_NR(SYS_readv),
#endif
#ifdef SYS_writev
        ALLOW_NR(SYS_writev),
#endif
#ifdef SYS_pread64
        ALLOW_NR(SYS_pread64),
#endif
#ifdef SYS_pwrite64
        ALLOW_NR(SYS_pwrite64),
#endif
#ifdef SYS_sendfile
        ALLOW_NR(SYS_sendfile),
#endif
#ifdef SYS_close
        ALLOW_NR(SYS_close),
#endif
#ifdef SYS_close_range
        ALLOW_NR(SYS_close_range),
#endif
#ifdef SYS_fstat
        ALLOW_NR(SYS_fstat),
#endif
#ifdef SYS_newfstatat
        ALLOW_NR(SYS_newfstatat),
#endif
#ifdef SYS_statx
        ALLOW_NR(SYS_statx),
#endif
#ifdef SYS_statfs
        ALLOW_NR(SYS_statfs),
#endif
#ifdef SYS_lseek
        ALLOW_NR(SYS_lseek),
#endif
#ifdef SYS_mmap
        ALLOW_NR(SYS_mmap),
#endif
#ifdef SYS_mprotect
        ALLOW_NR(SYS_mprotect),
#endif
#ifdef SYS_munmap
        ALLOW_NR(SYS_munmap),
#endif
#ifdef SYS_mremap
        ALLOW_NR(SYS_mremap),
#endif
#ifdef SYS_madvise
        ALLOW_NR(SYS_madvise),
#endif
#ifdef SYS_msync
        ALLOW_NR(SYS_msync),
#endif
#ifdef SYS_brk
        ALLOW_NR(SYS_brk),
#endif
#ifdef SYS_rt_sigaction
        ALLOW_NR(SYS_rt_sigaction),
#endif
#ifdef SYS_rt_sigprocmask
        ALLOW_NR(SYS_rt_sigprocmask),
#endif
#ifdef SYS_rt_sigreturn
        ALLOW_NR(SYS_rt_sigreturn),
#endif
#ifdef SYS_sigaltstack
        ALLOW_NR(SYS_sigaltstack),
#endif
#ifdef SYS_restart_syscall
        ALLOW_NR(SYS_restart_syscall),
#endif
#ifdef SYS_nanosleep
        ALLOW_NR(SYS_nanosleep),
#endif
#ifdef SYS_clock_nanosleep
        ALLOW_NR(SYS_clock_nanosleep),
#endif
#ifdef SYS_clock_gettime
        ALLOW_NR(SYS_clock_gettime),
#endif
#ifdef SYS_clock_getres
        ALLOW_NR(SYS_clock_getres),
#endif
#ifdef SYS_gettimeofday
        ALLOW_NR(SYS_gettimeofday),
#endif
#ifdef SYS_getitimer
        ALLOW_NR(SYS_getitimer),
#endif
#ifdef SYS_setitimer
        ALLOW_NR(SYS_setitimer),
#endif
#ifdef SYS_getpid
        ALLOW_NR(SYS_getpid),
#endif
#ifdef SYS_getppid
        ALLOW_NR(SYS_getppid),
#endif
#ifdef SYS_gettid
        ALLOW_NR(SYS_gettid),
#endif
#ifdef SYS_getuid
        ALLOW_NR(SYS_getuid),
#endif
#ifdef SYS_geteuid
        ALLOW_NR(SYS_geteuid),
#endif
#ifdef SYS_getgid
        ALLOW_NR(SYS_getgid),
#endif
#ifdef SYS_getegid
        ALLOW_NR(SYS_getegid),
#endif
#ifdef SYS_getpgrp
        ALLOW_NR(SYS_getpgrp),
#endif
#ifdef SYS_getsid
        ALLOW_NR(SYS_getsid),
#endif
#ifdef SYS_getcpu
        ALLOW_NR(SYS_getcpu),
#endif
#ifdef SYS_getrusage
        ALLOW_NR(SYS_getrusage),
#endif
#ifdef SYS_tgkill
        ALLOW_NR(SYS_tgkill),
#endif
#ifdef SYS_kill
        ALLOW_NR(SYS_kill),
#endif
#ifdef SYS_exit
        ALLOW_NR(SYS_exit),
#endif
#ifdef SYS_exit_group
        ALLOW_NR(SYS_exit_group),
#endif
#ifdef SYS_arch_prctl
        ALLOW_NR(SYS_arch_prctl),
#endif
#ifdef SYS_set_tid_address
        ALLOW_NR(SYS_set_tid_address),
#endif
#ifdef SYS_set_robust_list
        ALLOW_NR(SYS_set_robust_list),
#endif
#ifdef SYS_rseq
        ALLOW_NR(SYS_rseq),
#endif
#ifdef SYS_futex
        ALLOW_NR(SYS_futex),
#endif
#ifdef SYS_futex_waitv
        ALLOW_NR(SYS_futex_waitv),
#endif
#ifdef SYS_open
        ALLOW_NR(SYS_open),
#endif
#ifdef SYS_openat
        ALLOW_NR(SYS_openat),
#endif
#ifdef SYS_access
        ALLOW_NR(SYS_access),
#endif
#ifdef SYS_faccessat
        ALLOW_NR(SYS_faccessat),
#endif
#ifdef SYS_faccessat2
        ALLOW_NR(SYS_faccessat2),
#endif
#ifdef SYS_readlink
        ALLOW_NR(SYS_readlink),
#endif
#ifdef SYS_readlinkat
        ALLOW_NR(SYS_readlinkat),
#endif
#ifdef SYS_getcwd
        ALLOW_NR(SYS_getcwd),
#endif
#ifdef SYS_getdents64
        ALLOW_NR(SYS_getdents64),
#endif
#ifdef SYS_fcntl
        ALLOW_NR(SYS_fcntl),
#endif
#ifdef SYS_fsync
        ALLOW_NR(SYS_fsync),
#endif
#ifdef SYS_fdatasync
        ALLOW_NR(SYS_fdatasync),
#endif
#ifdef SYS_ftruncate
        ALLOW_NR(SYS_ftruncate),
#endif
#ifdef SYS_truncate
        ALLOW_NR(SYS_truncate),
#endif
#ifdef SYS_mkdir
        ALLOW_NR(SYS_mkdir),
#endif
#ifdef SYS_mkdirat
        ALLOW_NR(SYS_mkdirat),
#endif
#ifdef SYS_unlink
        ALLOW_NR(SYS_unlink),
#endif
#ifdef SYS_unlinkat
        ALLOW_NR(SYS_unlinkat),
#endif
#ifdef SYS_rename
        ALLOW_NR(SYS_rename),
#endif
#ifdef SYS_renameat
        ALLOW_NR(SYS_renameat),
#endif
#ifdef SYS_renameat2
        ALLOW_NR(SYS_renameat2),
#endif
#ifdef SYS_chmod
        ALLOW_NR(SYS_chmod),
#endif
#ifdef SYS_fchmod
        ALLOW_NR(SYS_fchmod),
#endif
#ifdef SYS_fchmodat
        ALLOW_NR(SYS_fchmodat),
#endif
#ifdef SYS_utimensat
        ALLOW_NR(SYS_utimensat),
#endif
#ifdef SYS_ioctl
        ALLOW_NR(SYS_ioctl),
#endif
        /* Node/libuv identifies inherited stdio socketpairs before attaching
         * them to the event loop. These calls only inspect already-open file
         * descriptors; socket creation and network I/O remain denied, and the
         * process is still isolated in its private network namespace. */
#ifdef SYS_getsockname
        ALLOW_NR(SYS_getsockname),
#endif
#ifdef SYS_getsockopt
        ALLOW_NR(SYS_getsockopt),
#endif
#ifdef SYS_dup
        ALLOW_NR(SYS_dup),
#endif
#ifdef SYS_dup2
        ALLOW_NR(SYS_dup2),
#endif
#ifdef SYS_dup3
        ALLOW_NR(SYS_dup3),
#endif
#ifdef SYS_pipe
        ALLOW_NR(SYS_pipe),
#endif
#ifdef SYS_pipe2
        ALLOW_NR(SYS_pipe2),
#endif
#ifdef SYS_poll
        ALLOW_NR(SYS_poll),
#endif
#ifdef SYS_ppoll
        ALLOW_NR(SYS_ppoll),
#endif
#ifdef SYS_select
        ALLOW_NR(SYS_select),
#endif
#ifdef SYS_pselect6
        ALLOW_NR(SYS_pselect6),
#endif
#ifdef SYS_epoll_create
        ALLOW_NR(SYS_epoll_create),
#endif
#ifdef SYS_epoll_create1
        ALLOW_NR(SYS_epoll_create1),
#endif
#ifdef SYS_epoll_ctl
        ALLOW_NR(SYS_epoll_ctl),
#endif
#ifdef SYS_epoll_wait
        ALLOW_NR(SYS_epoll_wait),
#endif
#ifdef SYS_epoll_pwait
        ALLOW_NR(SYS_epoll_pwait),
#endif
#ifdef SYS_epoll_pwait2
        ALLOW_NR(SYS_epoll_pwait2),
#endif
#ifdef SYS_eventfd
        ALLOW_NR(SYS_eventfd),
#endif
#ifdef SYS_eventfd2
        ALLOW_NR(SYS_eventfd2),
#endif
#ifdef SYS_timerfd_create
        ALLOW_NR(SYS_timerfd_create),
#endif
#ifdef SYS_timerfd_settime
        ALLOW_NR(SYS_timerfd_settime),
#endif
#ifdef SYS_timerfd_gettime
        ALLOW_NR(SYS_timerfd_gettime),
#endif
#ifdef SYS_sched_yield
        ALLOW_NR(SYS_sched_yield),
#endif
#ifdef SYS_sched_getaffinity
        ALLOW_NR(SYS_sched_getaffinity),
#endif
#ifdef SYS_sched_setaffinity
        ALLOW_NR(SYS_sched_setaffinity),
#endif
#ifdef SYS_sched_getparam
        ALLOW_NR(SYS_sched_getparam),
#endif
#ifdef SYS_sched_getscheduler
        ALLOW_NR(SYS_sched_getscheduler),
#endif
#ifdef SYS_sched_get_priority_max
        ALLOW_NR(SYS_sched_get_priority_max),
#endif
#ifdef SYS_sched_get_priority_min
        ALLOW_NR(SYS_sched_get_priority_min),
#endif
#ifdef SYS_prlimit64
        ALLOW_NR(SYS_prlimit64),
#endif
#ifdef SYS_getrlimit
        ALLOW_NR(SYS_getrlimit),
#endif
#ifdef SYS_uname
        ALLOW_NR(SYS_uname),
#endif
#ifdef SYS_sysinfo
        ALLOW_NR(SYS_sysinfo),
#endif
#ifdef SYS_getrandom
        ALLOW_NR(SYS_getrandom),
#endif
#ifdef SYS_prctl
        ALLOW_NR(SYS_prctl),
#endif
#ifdef SYS_membarrier
        ALLOW_NR(SYS_membarrier),
#endif
#ifdef SYS_capget
        ALLOW_NR(SYS_capget),
#endif
#ifdef SYS_execve
        ALLOW_NR(SYS_execve),
#endif
#ifdef SYS_execveat
        ALLOW_NR(SYS_execveat),
#endif
#ifdef SYS_clone3
        ERRNO_NR(SYS_clone3, ENOSYS),
#endif
#ifdef SYS_clone
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_clone, 0, 3),
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[0])),
        BPF_JUMP(BPF_JMP | BPF_JSET | BPF_K, CLONE_THREAD, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
#endif
        BPF_STMT(BPF_RET | BPF_K, DENIED),
    };
    struct sock_fprog program = { .len = (unsigned short)(sizeof(filter) / sizeof(filter[0])), .filter = filter };
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) return -1;
    return (int)syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &program);
#undef LOAD_NR
#undef ALLOW_NR
#undef ERRNO_NR
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
    const char *cgroup_root = argument(argc, argv, "--cgroup-root");
    if (provider == NULL || strcmp(provider, "forme-linux-v1") != 0
            || manifest_hash == NULL || schema_hash == NULL || entry_hash == NULL
            || working_directory == NULL || runtime == NULL || runtime_root == NULL
            || entry == NULL || schema == NULL || stage == NULL
            || cgroup_root == NULL) return 64;

    char canonical_cwd[PATH_MAX], canonical_entry[PATH_MAX], canonical_runtime[PATH_MAX];
    char canonical_runtime_root[PATH_MAX];
    char canonical_schema[PATH_MAX] = {0};
    if (realpath(working_directory, canonical_cwd) == NULL
            || realpath(entry, canonical_entry) == NULL
            || realpath(runtime, canonical_runtime) == NULL
            || realpath(runtime_root, canonical_runtime_root) == NULL
            || strncmp(canonical_entry, canonical_cwd, strlen(canonical_cwd)) != 0
            || canonical_entry[strlen(canonical_cwd)] != '/'
            || strncmp(canonical_runtime, canonical_runtime_root, strlen(canonical_runtime_root)) != 0
            || canonical_runtime[strlen(canonical_runtime_root)] != '/') return 65;
    if (strcmp(schema, "-") != 0
            && (realpath(schema, canonical_schema) == NULL
                || strncmp(canonical_schema, canonical_cwd, strlen(canonical_cwd)) != 0
                || canonical_schema[strlen(canonical_cwd)] != '/')) return 65;
    rlim_t memory = 0, cpu_ms = 0, wall_ms = 0, descriptors = 0;
    if (parse_limit(memory_text, &memory) != 0 || parse_limit(cpu_text, &cpu_ms) != 0
            || parse_limit(wall_text, &wall_ms) != 0
            || parse_limit(fd_text, &descriptors) != 0) return 66;

    char cgroup[PATH_MAX] = {0};
    if (setup_cgroup(cgroup_root, memory, cgroup, sizeof(cgroup)) != 0) {
        perror("cgroup v2 setup");
        cleanup_cgroup(cgroup);
        return 67;
    }
    uid_t uid = getuid();
    gid_t gid = getgid();
    if (enter_namespaces(uid, gid) != 0 || mount(NULL, "/", NULL, MS_REC | MS_PRIVATE, NULL) != 0) {
        perror("namespace setup");
        cleanup_cgroup(cgroup);
        return 68;
    }
    char root[PATH_MAX] = {0}, sandbox_runtime[PATH_MAX], sandbox_entry[PATH_MAX];
    if (setup_root(canonical_cwd, canonical_runtime, canonical_runtime_root, canonical_entry, root, sizeof(root),
            sandbox_runtime, sizeof(sandbox_runtime), sandbox_entry, sizeof(sandbox_entry)) != 0) {
        fprintf(stderr, "private root setup (%s): %s\n", root_setup_stage, strerror(errno));
        if (root[0] != '\0') {
            umount2(root, MNT_DETACH);
            rmdir(root);
        }
        cleanup_cgroup(cgroup);
        return 69;
    }

    int installed[2];
    if (pipe2(installed, O_CLOEXEC) != 0) {
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return 70;
    }
    pid_t child = fork();
    if (child < 0) {
        close(installed[0]);
        close(installed[1]);
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return 71;
    }
    if (child == 0) {
        close(installed[0]);
        close(READY_FD);
        long close_max = sysconf(_SC_OPEN_MAX);
        if (installed[1] != 4) {
            if (dup2(installed[1], 4) < 0) _exit(72);
            close(installed[1]);
            installed[1] = 4;
        }
        if (fcntl(installed[1], F_SETFD, FD_CLOEXEC) != 0) _exit(72);
        rlim_t cpu_seconds = (cpu_ms + 999) / 1000;
        struct rlimit cpu_limit = { cpu_seconds, cpu_seconds };
        struct rlimit fd_limit = { descriptors, descriptors };
        if (setrlimit(RLIMIT_CPU, &cpu_limit) != 0 || setrlimit(RLIMIT_NOFILE, &fd_limit) != 0
                || chroot(root) != 0 || chdir("/work") != 0
                || mount("proc", "/proc", "proc", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL) != 0) _exit(72);
        if (setenv("HOME", "/work", 1) != 0
                || setenv("TMPDIR", "/work", 1) != 0
                || setenv("TMP", "/work", 1) != 0
                || setenv("TEMP", "/work", 1) != 0) _exit(73);
        int closed_range = -1;
#ifdef SYS_close_range
        closed_range = (int)syscall(SYS_close_range, 5U, ~0U, 0U);
#endif
        if (closed_range != 0) {
            for (int fd = 5; fd < close_max; fd++) close(fd);
        }
        if (prctl(PR_SET_PDEATHSIG, SIGKILL, 0, 0, 0) != 0 || getppid() == 1) _exit(74);
        if (verify_sha256_file(sandbox_entry, entry_hash) != 0) _exit(75);
        if (strcmp(schema_hash, "-") != 0
                && verify_sha256_file("/work/.forme-snapshot/plugin-config-schema.json", schema_hash) != 0) _exit(76);
        if (drop_namespace_root_authority() != 0) _exit(77);
        if (install_seccomp() != 0) _exit(78);
        if (write_all(installed[1], "1", 1) != 0) _exit(79);
        execl(sandbox_runtime, sandbox_runtime, sandbox_entry, stage, schema_hash, (char *)NULL);
        write_all(installed[1], "0", 1);
        _exit(80);
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
        int child_status = 0;
        waitpid(child, &child_status, 0);
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return WIFEXITED(child_status) ? WEXITSTATUS(child_status) : 81;
    }
    close(installed[0]);
    if (fcntl(READY_FD, F_SETFD, FD_CLOEXEC) != 0) {
        kill(child, SIGKILL);
        waitpid(child, NULL, 0);
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return 76;
    }
    char readiness[2048];
    int readiness_length = snprintf(readiness, sizeof(readiness),
        "{\"protocol\":1,\"provider\":\"forme-linux-v1\","
        "\"manifestHash\":\"%s\",\"configSchemaHash\":%s%s%s,\"entryHash\":\"%s\"}\n",
        manifest_hash, strcmp(schema_hash, "-") == 0 ? "" : "\"",
        strcmp(schema_hash, "-") == 0 ? "null" : schema_hash,
        strcmp(schema_hash, "-") == 0 ? "" : "\"", entry_hash);
    if (readiness_length <= 0 || (size_t)readiness_length >= sizeof(readiness)
            || write_all(READY_FD, readiness, (size_t)readiness_length) != 0) {
        kill(child, SIGKILL);
        waitpid(child, NULL, 0);
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return 76;
    }
    close(READY_FD);
    int status = 0;
    uint64_t started = monotonic_milliseconds();
    if (started == UINT64_MAX) {
        kill(child, SIGKILL);
        waitpid(child, &status, 0);
        umount2(root, MNT_DETACH);
        rmdir(root);
        cleanup_cgroup(cgroup);
        return 77;
    }
    for (;;) {
        pid_t waited = waitpid(child, &status, WNOHANG);
        if (waited == child) break;
        if (waited < 0 && errno != EINTR) {
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            umount2(root, MNT_DETACH);
            rmdir(root);
            cleanup_cgroup(cgroup);
            return 77;
        }
        uint64_t now = monotonic_milliseconds();
        if (now == UINT64_MAX || now - started > (uint64_t)wall_ms) {
            kill(child, SIGKILL);
            waitpid(child, &status, 0);
            status = 124 << 8;
            break;
        }
        struct timespec pause = { .tv_sec = 0, .tv_nsec = 10000000 };
        nanosleep(&pause, NULL);
    }
    umount2(root, MNT_DETACH);
    rmdir(root);
    if (cleanup_cgroup(cgroup) != 0) return 82;
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 77;
}
