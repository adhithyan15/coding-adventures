/*
 * fs_test.c — round-trip tests for os_platform/fs, run on each OS.
 * ===========================================================================
 *
 * The test drives a real file through the API and checks observable results:
 *
 *   - write then read returns the exact bytes (binary-safe: the payload holds an
 *     embedded NUL, so a string-based reader would truncate — this proves ours
 *     does not), with the right length and a '\0' terminator past the data;
 *   - stat reports a regular file of the written size, and osp_fs_exists agrees;
 *   - list_dir enumerates the directory and surfaces our file by name;
 *   - NULL arguments are rejected.
 *
 * Files are created under `_build/` — the harness makes that directory and it is
 * .gitignore'd, so nothing leaks into the working tree. Cleanup uses ISO C
 * remove() (not part of the fs API under test). The working directory when the
 * test binary runs is the package root (both run.sh and run.ps1 cd there), so
 * the relative paths below resolve correctly on every OS.
 */
#include "iso_test.h"

#include "os_platform/fs.h"

#include <stdio.h>  /* remove */
#include <stdlib.h> /* free */
#include <stddef.h> /* NULL */
#include <string.h> /* strcmp */

/* A payload with an embedded NUL, to prove reads are length-based, not string
 * based. */
static const unsigned char PAYLOAD[] = {'h', 'i', 0x00, 'x', 'y'};
#define PAYLOAD_LEN 5

#define PROBE_DIR "_build"
#define PROBE_NAME "osp_fs_probe.bin"
#define PROBE_PATH PROBE_DIR "/" PROBE_NAME

typedef struct {
    const char *target;
    int found;
} list_ctx;

static void list_cb(const char *name, void *user) {
    list_ctx *c = (list_ctx *)user;
    if (strcmp(name, c->target) == 0) {
        c->found = 1;
    }
}

int main(void) {
    unsigned char *data = NULL;
    size_t len = 0;
    osp_file_info info;
    list_ctx lc;

    /* ── write → read round-trip (binary-safe) ──────────────────────────── */
    ISO_CHECK(osp_fs_write_file(PROBE_PATH, PAYLOAD, PAYLOAD_LEN) == OSP_OK);
    ISO_CHECK(osp_fs_read_file(PROBE_PATH, &data, &len) == OSP_OK);
    ISO_CHECK_EQ_UINT(len, PAYLOAD_LEN);
    if (data != NULL) {
        ISO_CHECK_MEM_EQ(data, PAYLOAD, PAYLOAD_LEN);
        ISO_CHECK_MSG(data[PAYLOAD_LEN] == '\0', "read buffer must be terminated");
        free(data);
    } else {
        ISO_CHECK_MSG(0, "osp_fs_read_file returned OK but a NULL buffer");
    }

    /* ── stat + exists ──────────────────────────────────────────────────── */
    ISO_CHECK(osp_fs_stat(PROBE_PATH, &info) == OSP_OK);
    ISO_CHECK_MSG(info.is_regular == 1, "probe should be a regular file");
    ISO_CHECK_MSG(info.is_dir == 0, "probe should not be a directory");
    ISO_CHECK_EQ_UINT(info.size, PAYLOAD_LEN);
    ISO_CHECK(osp_fs_exists(PROBE_PATH) == 1);
    ISO_CHECK(osp_fs_exists(PROBE_DIR "/does_not_exist") == 0);

    /* stat should also recognise a directory */
    ISO_CHECK(osp_fs_stat(PROBE_DIR, &info) == OSP_OK);
    ISO_CHECK_MSG(info.is_dir == 1, "_build should stat as a directory");

    /* ── list_dir surfaces our file ─────────────────────────────────────── */
    lc.target = PROBE_NAME;
    lc.found = 0;
    ISO_CHECK(osp_fs_list_dir(PROBE_DIR, list_cb, &lc) == OSP_OK);
    ISO_CHECK_MSG(lc.found == 1, "list_dir must surface the probe file");

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(osp_fs_stat(NULL, &info) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_stat(PROBE_PATH, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_read_file(NULL, &data, &len) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_write_file(NULL, PAYLOAD, PAYLOAD_LEN) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_write_file(PROBE_PATH, NULL, 1) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_list_dir(NULL, list_cb, &lc) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_list_dir(PROBE_DIR, NULL, &lc) == OSP_ERR_INVAL);
    ISO_CHECK(osp_fs_exists(NULL) == 0);

    /* reading a non-existent file must fail cleanly */
    ISO_CHECK(osp_fs_read_file(PROBE_DIR "/does_not_exist", &data, &len) == OSP_ERR_OS);

    /* ── cleanup (ISO C remove, not part of the fs API) ─────────────────── */
    remove(PROBE_PATH);

    return ISO_TEST_RESULT();
}
