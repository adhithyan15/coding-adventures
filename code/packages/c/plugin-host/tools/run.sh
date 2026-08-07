#!/bin/sh
# run.sh — build the example plugin, then build & run the host test (Unix).
#
# Unlike the other packages this one has an extra first step: it compiles the
# plugin source into a real shared library that the host loads at run time. If
# this toolchain cannot produce a shared library, the whole test SKIPS gracefully
# (exit 0) — the CCPP02 plan's "graceful skip when the SDK is absent" rule.
# The Windows half lives in run.ps1 (BUILD_windows), building a .dll with cl /LD.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/platform-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/platform-harness"
ISO="$d/code/packages/c/iso-harness"
OSP="$d/code/packages/c/os-platform"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi

. "$HARNESS/lib/platform-lib.sh"

PLATFORM_INCLUDE="$SELF/include $OSP/include $ISO/include"
export PLATFORM_INCLUDE

echo "plugin-host ($(platform_os)): C compilers: $(platform_compilers c)"

# ── step 1: build the plugin shared library ──────────────────────────────────
mkdir -p _build
# Pick any present compiler to build the plugin (the loader is compiler-agnostic).
PLUGCC=$(platform_compilers c | awk '{print $1}')
if [ -z "$PLUGCC" ]; then
    echo "plugin-host: no C compiler available to build the plugin; skipping"
    exit 0
fi
# On macOS a dylib named .so loads fine via dlopen; keep one name for both.
if ! "$PLUGCC" -shared -fPIC -I"$SELF/include" \
        plugins/example_plugin.c -o _build/osp_plugin.so 2>_build/plugin_build.log; then
    echo "plugin-host: this toolchain cannot build a shared plugin; skipping gracefully"
    sed 's/^/plugin-host:   /' _build/plugin_build.log || true
    exit 0
fi

# ── step 2: build & run the host test ────────────────────────────────────────
# dlopen lives in libdl on Linux (macOS has it in libc and no libdl).
if [ "$(platform_os)" = "linux" ]; then
    PLATFORM_LIBS="-ldl"
else
    PLATFORM_LIBS=""
fi
export PLATFORM_LIBS

rc=0
platform_build_and_run c plugin-host-tests \
    tests/plugin_host_test.c src/host.c "$OSP/src/dynlib_posix.c" || rc=1
exit "$rc"
