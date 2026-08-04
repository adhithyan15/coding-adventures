/*
 * example_plugin.c — a minimal conforming plugin.
 * ===========================================================================
 *
 * Built into a shared library (.so / .dylib / .dll) by the package's run script,
 * then loaded and called by the host at run time. It exports exactly one symbol,
 * the entry point the ABI names, marked for export so the loader can resolve it.
 *
 * The behaviour is deliberately trivial and pure so the test can assert an exact
 * result: it doubles its argument and adds one.
 */
#include "plugin_host/plugin_abi.h"

OSP_PLUGIN_EXPORT int osp_plugin_entry(int x) {
    return x * 2 + 1;
}
