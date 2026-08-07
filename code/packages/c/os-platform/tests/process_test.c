/*
 * process_test.c — spawn/wait/exit-code tests for os_platform/process.
 * ===========================================================================
 *
 * We spawn a real child — the system shell — asked to terminate with a specific
 * exit code, then assert osp_process_wait reports that exact code. This proves
 * the whole round trip at once: the child was spawned, the arguments reached it
 * intact (a wrong exit code would mean the "exit N" arguments were mangled — on
 * Windows that also exercises the command-line quoting/join), the parent waited,
 * and the code was read back correctly.
 *
 * The shell differs per OS, so (only in this test — never in the library
 * backends) a #ifdef selects it:
 *   - POSIX : /bin/sh -c "exit N"
 *   - Windows: %ComSpec% (cmd.exe) /c exit N
 *
 * NULL-argument validation rounds out the checks.
 */
#include "iso_test.h"

#include "os_platform/process.h"

#include <stddef.h> /* NULL */
#include <stdlib.h> /* getenv */

/* Spawn the system shell to exit with `code`, and return the code wait reports
 * (or -999 on any API failure so the assertion shows something obvious). */
static int run_shell_exit(int code) {
    osp_process *proc = NULL;
    int got = -999;

#ifdef _WIN32
    const char *shell = getenv("ComSpec"); /* full path to cmd.exe */
    char code_str[16];
    const char *argv[5];
    if (shell == NULL) {
        return -998;
    }
    /* Render the code as text without sprintf: small non-negative integers. */
    {
        int v = code;
        int n = 0;
        char tmp[16];
        if (v == 0) {
            tmp[n++] = '0';
        }
        while (v > 0 && n < (int)sizeof(tmp)) {
            tmp[n++] = (char)('0' + (v % 10));
            v /= 10;
        }
        {
            int j;
            int m = 0;
            for (j = n - 1; j >= 0; j--) {
                code_str[m++] = tmp[j];
            }
            code_str[m] = '\0';
        }
    }
    argv[0] = "cmd.exe";
    argv[1] = "/c";
    argv[2] = "exit";
    argv[3] = code_str;
    argv[4] = NULL;
    if (osp_process_spawn(&proc, shell, argv) != OSP_OK) {
        return -997;
    }
#else
    const char *shell = "/bin/sh";
    char cmd[32];
    const char *argv[4];
    /* Build "exit N" without sprintf. */
    {
        int v = code;
        int n = 0;
        char digits[16];
        cmd[0] = 'e'; cmd[1] = 'x'; cmd[2] = 'i'; cmd[3] = 't'; cmd[4] = ' ';
        if (v == 0) {
            digits[n++] = '0';
        }
        while (v > 0 && n < (int)sizeof(digits)) {
            digits[n++] = (char)('0' + (v % 10));
            v /= 10;
        }
        {
            int j;
            int m = 5;
            for (j = n - 1; j >= 0; j--) {
                cmd[m++] = digits[j];
            }
            cmd[m] = '\0';
        }
    }
    argv[0] = "sh";
    argv[1] = "-c";
    argv[2] = cmd;
    argv[3] = NULL;
    if (osp_process_spawn(&proc, shell, argv) != OSP_OK) {
        return -997;
    }
#endif

    if (osp_process_wait(proc, &got) != OSP_OK) {
        return -996;
    }
    return got;
}

int main(void) {
    osp_process *dummy = NULL;
    const char *const empty_argv[] = {NULL};

    /* ── exit-code propagation ──────────────────────────────────────────── */
    ISO_CHECK_EQ_INT(run_shell_exit(42), 42);
    ISO_CHECK_EQ_INT(run_shell_exit(0), 0);
    ISO_CHECK_EQ_INT(run_shell_exit(7), 7);

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(osp_process_spawn(NULL, "/bin/sh", empty_argv) == OSP_ERR_INVAL);
    ISO_CHECK(osp_process_spawn(&dummy, NULL, empty_argv) == OSP_ERR_INVAL);
    ISO_CHECK(osp_process_spawn(&dummy, "/bin/sh", NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_process_wait(NULL, NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
