/*
 * process_windows.c — the Win32 backend of os_platform/process.
 * ===========================================================================
 *
 * Compiled on Windows (named by `BUILD_windows`; macOS/Linux use process_posix.c
 * via the shared `BUILD`). No OS #ifdefs — the build chose this file. Uses only
 * kernel32.
 *
 * The impedance mismatch: POSIX spawns from an argv ARRAY, but CreateProcess
 * takes a single command-line STRING, and the child's C runtime later splits
 * that string back into argv using a specific, quirky rule set (the one
 * CommandLineToArgvW implements). To make a round trip faithful — caller's argv
 * in, identical argv out — we must quote each argument by exactly those rules:
 *
 *   - an argument needs quoting if it is empty or contains a space, tab, or ";
 *   - inside quotes, a run of N backslashes that PRECEDES a " becomes 2N+1
 *     backslashes plus the " (so the " is escaped); a run that precedes the
 *     closing quote becomes 2N (so the backslashes are literal, not an escape);
 *     backslashes elsewhere are literal.
 *
 * Getting this wrong is a classic argument-injection bug, so the rule is
 * implemented once, here, and unit-tested via a child that echoes its argv.
 */
#include "os_platform/process.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h>
#include <string.h>

struct osp_process {
    HANDLE hProcess;
};

/* Does this argument require surrounding quotes? */
static int osp__arg_needs_quotes(const char *arg) {
    size_t i;
    if (arg[0] == '\0') {
        return 1; /* the empty argument must be written as "" */
    }
    for (i = 0; arg[i] != '\0'; i++) {
        if (arg[i] == ' ' || arg[i] == '\t' || arg[i] == '"') {
            return 1;
        }
    }
    return 0;
}

/* Append `arg`, quoted per the CommandLineToArgvW rules, into buf at *pos. The
 * caller must have reserved at least 2*strlen(arg)+2 bytes for this argument. */
static void osp__append_quoted(char *buf, size_t *pos, const char *arg) {
    size_t i;
    if (!osp__arg_needs_quotes(arg)) {
        for (i = 0; arg[i] != '\0'; i++) {
            buf[(*pos)++] = arg[i];
        }
        return;
    }
    buf[(*pos)++] = '"';
    i = 0;
    for (;;) {
        size_t nbs = 0;
        while (arg[i] == '\\') {
            nbs++;
            i++;
        }
        if (arg[i] == '\0') {
            /* backslashes precede the closing quote: double them, don't escape */
            size_t k;
            for (k = 0; k < nbs * 2; k++) {
                buf[(*pos)++] = '\\';
            }
            break;
        } else if (arg[i] == '"') {
            /* backslashes precede a literal quote: double them, then escape " */
            size_t k;
            for (k = 0; k < nbs * 2 + 1; k++) {
                buf[(*pos)++] = '\\';
            }
            buf[(*pos)++] = '"';
            i++;
        } else {
            size_t k;
            for (k = 0; k < nbs; k++) {
                buf[(*pos)++] = '\\';
            }
            buf[(*pos)++] = arg[i];
            i++;
        }
    }
    buf[(*pos)++] = '"';
}

/* Build the full command line from argv into a freshly malloc'd, NUL-terminated
 * string. Returns NULL on allocation failure. */
static char *osp__build_command_line(const char *const argv[]) {
    size_t total = 1; /* trailing NUL */
    size_t i;
    char *buf;
    size_t pos;

    for (i = 0; argv[i] != NULL; i++) {
        /* worst case: every char doubles, plus two surrounding quotes, plus a
         * separating space between arguments. Guard the size_t arithmetic
         * against overflow (unreachable for real argv, but a hard bound beats a
         * silent under-allocation): reject if 2*len+3 or the running total would
         * wrap. */
        size_t len = strlen(argv[i]);
        size_t need;
        if (len > (SIZE_MAX - 3) / 2) {
            return NULL;
        }
        need = 2 * len + 3;
        if (need > SIZE_MAX - total) {
            return NULL;
        }
        total += need;
    }
    buf = (char *)malloc(total);
    if (buf == NULL) {
        return NULL;
    }
    pos = 0;
    for (i = 0; argv[i] != NULL; i++) {
        if (i > 0) {
            buf[pos++] = ' ';
        }
        osp__append_quoted(buf, &pos, argv[i]);
    }
    buf[pos] = '\0';
    return buf;
}

osp_status osp_process_spawn(osp_process **out, const char *path,
                             const char *const argv[]) {
    struct osp_process *p;
    char *cmdline;
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    BOOL ok;

    if (out == NULL || path == NULL || argv == NULL) {
        return OSP_ERR_INVAL;
    }
    p = (struct osp_process *)malloc(sizeof(*p));
    if (p == NULL) {
        return OSP_ERR_NOMEM;
    }
    cmdline = osp__build_command_line(argv);
    if (cmdline == NULL) {
        free(p);
        return OSP_ERR_NOMEM;
    }

    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    /* lpApplicationName = the explicit path (no search); lpCommandLine carries
     * argv (CreateProcessA may modify it in place, hence a writable buffer). */
    ok = CreateProcessA(path, cmdline, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    free(cmdline);
    if (!ok) {
        free(p);
        return OSP_ERR_OS;
    }
    /* We do not need the thread handle; closing it now does not affect the
     * process. The process handle is kept for the wait. */
    CloseHandle(pi.hThread);
    p->hProcess = pi.hProcess;
    *out = p;
    return OSP_OK;
}

osp_status osp_process_wait(osp_process *p, int *exit_code_out) {
    DWORD code;

    if (p == NULL) {
        return OSP_ERR_INVAL;
    }
    if (WaitForSingleObject(p->hProcess, INFINITE) != WAIT_OBJECT_0) {
        return OSP_ERR_OS; /* keep the handle: we did not reap the process */
    }
    if (!GetExitCodeProcess(p->hProcess, &code)) {
        /* The process was already reaped by the wait above, so the handle is of
         * no further use — release it (and the wrapper) rather than leak. */
        CloseHandle(p->hProcess);
        free(p);
        return OSP_ERR_OS;
    }
    CloseHandle(p->hProcess);
    if (exit_code_out != NULL) {
        *exit_code_out = (int)code;
    }
    free(p);
    return OSP_OK;
}
