/*
 * plugin_host_test.c — load the example plugin and call it, on every OS.
 * ===========================================================================
 *
 * The run script first compiles plugins/example_plugin.c into a shared library
 * at the path below (relative to the package root, which is the working
 * directory when this test runs). Then this test loads it through the host,
 * calls its entry point, checks the exact results, and exercises the error
 * paths. If the toolchain here cannot build a shared library at all, the run
 * script skips gracefully before this test is ever built.
 */
#include "iso_test.h"

#include "plugin_host/host.h"

#include <stddef.h> /* NULL */

#ifdef _WIN32
#define OSP_PLUGIN_PATH "_build\\osp_plugin.dll"
#else
#define OSP_PLUGIN_PATH "_build/osp_plugin.so"
#endif

int main(void) {
    osp_plugin *p = NULL;
    int result = 0;

    /* ── load and call the plugin ───────────────────────────────────────── */
    ISO_CHECK(osp_plugin_open(OSP_PLUGIN_PATH, &p) == OSP_OK);
    if (p != NULL) {
        ISO_CHECK(osp_plugin_call(p, 20, &result) == OSP_OK);
        ISO_CHECK_EQ_INT(result, 41); /* 20*2 + 1 */
        ISO_CHECK(osp_plugin_call(p, 0, &result) == OSP_OK);
        ISO_CHECK_EQ_INT(result, 1); /* 0*2 + 1 */
        ISO_CHECK(osp_plugin_close(p) == OSP_OK);
    } else {
        ISO_CHECK_MSG(0, "osp_plugin_open returned OK but a NULL handle");
    }

    /* ── error + NULL-argument paths ────────────────────────────────────── */
    p = NULL;
    ISO_CHECK(osp_plugin_open("osp_no_such_plugin.zzz", &p) == OSP_ERR_OS);
    ISO_CHECK(osp_plugin_open(NULL, &p) == OSP_ERR_INVAL);
    ISO_CHECK(osp_plugin_open(OSP_PLUGIN_PATH, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_plugin_call(NULL, 1, &result) == OSP_ERR_INVAL);
    ISO_CHECK(osp_plugin_close(NULL) == OSP_ERR_INVAL);

    return ISO_TEST_RESULT();
}
