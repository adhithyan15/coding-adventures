/*
 * process_posix.c — the POSIX backend of os_platform/process (macOS + Linux).
 * ===========================================================================
 *
 * Compiled on macOS + Linux (named by the shared `BUILD`; Windows uses
 * process_windows.c via `BUILD_windows`). No OS #ifdefs — the build chose this
 * file. Uses only libc (fork/execv/waitpid), so no extra library is linked.
 *
 * The classic UNIX dance:
 *
 *     pid = fork();          duplicate this process into parent + child
 *     if (pid == 0)          in the CHILD:
 *         execv(path, argv);   replace the image with the target program
 *         _exit(127);          only reached if execv failed
 *     // in the PARENT: remember pid, later waitpid() for it
 *
 * We use execv (not execvp): it takes an explicit path and performs NO PATH
 * search — the target is exactly `path`, nothing else. The child uses _exit
 * (not exit) so it does not flush or run the parent's atexit handlers on the
 * exec-failure path.
 */
#include "os_platform/process.h"

#include <errno.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

struct osp_process {
    pid_t pid;
};

osp_status osp_process_spawn(osp_process **out, const char *path,
                             const char *const argv[]) {
    struct osp_process *p;
    pid_t pid;

    if (out == NULL || path == NULL || argv == NULL) {
        return OSP_ERR_INVAL;
    }
    p = (struct osp_process *)malloc(sizeof(*p));
    if (p == NULL) {
        return OSP_ERR_NOMEM;
    }
    pid = fork();
    if (pid < 0) {
        free(p);
        return OSP_ERR_OS;
    }
    if (pid == 0) {
        /* Child. execv's prototype takes char *const argv[]; our argv is
         * const-deeper, so cast the pointer type. execv does not modify the
         * strings, so this is safe. If it returns, exec failed — exit 127, the
         * shell's "command not found / not executable" convention. */
        execv(path, (char *const *)argv);
        _exit(127);
    }
    /* Parent. */
    p->pid = pid;
    *out = p;
    return OSP_OK;
}

osp_status osp_process_wait(osp_process *p, int *exit_code_out) {
    int status;
    pid_t r;

    if (p == NULL) {
        return OSP_ERR_INVAL;
    }
    /* waitpid can be interrupted by a signal delivered to us; retry. */
    do {
        r = waitpid(p->pid, &status, 0);
    } while (r < 0 && errno == EINTR);

    if (r < 0) {
        /* Leave the handle un-freed: we did not reap the child, so its pid is
         * still meaningful to a retry. */
        return OSP_ERR_OS;
    }
    if (exit_code_out != NULL) {
        if (WIFEXITED(status)) {
            *exit_code_out = WEXITSTATUS(status);
        } else if (WIFSIGNALED(status)) {
            *exit_code_out = 128 + WTERMSIG(status); /* shell convention */
        } else {
            *exit_code_out = -1; /* stopped/continued shouldn't occur here */
        }
    }
    free(p);
    return OSP_OK;
}
