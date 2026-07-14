/*
 * os_platform/process.h — spawn a child program, wait for it, read its exit code.
 * ===========================================================================
 *
 * The fourth os-platform primitive. Running another program is pure bucket B:
 * ISO C's only door to it is system(), which blocks, hands control to a shell,
 * and cannot even reliably report the child's exit code. Real process control
 * means the OS's own calls:
 *
 *      step     macOS / Linux            Windows
 *      ───────  ───────────────────────  ─────────────────────────────────
 *      spawn    fork() + execv()         CreateProcess
 *      wait     waitpid()                WaitForSingleObject
 *      exit     WIFEXITED/WEXITSTATUS    GetExitCodeProcess
 *
 * MODEL. A process is an opaque handle created by osp_process_spawn and consumed
 * by osp_process_wait (which blocks until the child exits, reports its exit code,
 * and frees the handle — so no OS process handle leaks). The child runs
 * concurrently between the two calls.
 *
 * NO SHELL, NO PATH SEARCH. `path` is an explicit executable path; it is handed
 * to execv/CreateProcess directly, never to a shell. That means this primitive
 * does NOT perform shell word-splitting, globbing, PATH lookup, or any other
 * interpretation of `path` or the arguments — a deliberate safety property: there
 * is no shell to inject into. Callers that want a PATH search resolve it before
 * calling.
 *
 * ARGUMENTS. `argv` is a NULL-terminated array of C strings; by convention
 * argv[0] is the program's own name (often the same as `path`). The values are
 * passed to the child verbatim. On Windows, where the OS takes a single command
 * line rather than an array, the backend re-quotes argv using the exact rules
 * CommandLineToArgvW parses, so a child built with the C runtime sees the same
 * argv the caller supplied.
 *
 * BUILD. Compiled by platform-harness; POSIX links no extra library, Windows uses
 * only kernel32. Per-OS source selection is done by the BUILD file.
 */
#ifndef OS_PLATFORM_PROCESS_H
#define OS_PLATFORM_PROCESS_H

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque child-process handle. Created by osp_process_spawn, freed by
 * osp_process_wait. Do not copy or free it yourself. */
typedef struct osp_process osp_process;

/*
 * osp_process_spawn — start `path` with arguments `argv`.
 *
 * `argv` is NUL-terminated (argv[0] conventionally the program name). On success
 * writes a handle through *out and returns OSP_OK; the child runs concurrently.
 * Returns OSP_ERR_INVAL if out, path, or argv is NULL; OSP_ERR_NOMEM on an
 * allocation failure; OSP_ERR_OS if the OS cannot start the process.
 *
 * On POSIX a spawn "succeeds" as soon as fork() succeeds; if execv then fails in
 * the child (e.g. no such file), that surfaces as exit code 127 from
 * osp_process_wait, mirroring the shell convention.
 */
osp_status osp_process_spawn(osp_process **out, const char *path,
                             const char *const argv[]);

/*
 * osp_process_wait — block until the child exits, then free the handle.
 *
 * If `exit_code_out` is non-NULL it receives the child's exit code. On POSIX a
 * child terminated by a signal reports 128 + signal-number (the shell
 * convention). The handle `p` is freed and must not be reused. Returns
 * OSP_ERR_INVAL if `p` is NULL, OSP_ERR_OS on a wait failure.
 */
osp_status osp_process_wait(osp_process *p, int *exit_code_out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_PROCESS_H */
