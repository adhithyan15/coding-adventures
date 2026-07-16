# plugin-host (C)

**CCPP02 Phase 4** — the representative that ties the whole lane together. A
*plugin host* loads code at run time, resolves a known entry point, and calls it
— the pattern behind editor extensions, codec plugins, and language-runtime FFI.
It is built entirely on the os-platform `dynlib` primitive, so it has **no per-OS
backend of its own**: `dlopen`/`LoadLibrary` etc. all live in `dynlib`, and the
host just knows the plugin contract.

## The contract (`plugin_host/plugin_abi.h`)

A plugin is a shared library that exports one function:

```c
#include "plugin_host/plugin_abi.h"
OSP_PLUGIN_EXPORT int osp_plugin_entry(int x) { return x * 2 + 1; }
```

`OSP_PLUGIN_EXPORT` is `__declspec(dllexport)` on Windows and nothing on POSIX
(default visibility already exports it). The host resolves the symbol named
`OSP_PLUGIN_ENTRY_NAME`.

## The host (`plugin_host/host.h`)

```c
#include "plugin_host/host.h"

osp_plugin *p;
osp_plugin_open("libmyplugin.so", &p);   /* dynlib_open + resolve the entry */
int result;
osp_plugin_call(p, 20, &result);          /* -> 41 */
osp_plugin_close(p);                       /* dynlib_close + free */
```

| Function | Purpose |
|----------|---------|
| `osp_plugin_open(path, &p)` | load the library, resolve the entry point |
| `osp_plugin_call(p, arg, &out)` | invoke the entry point |
| `osp_plugin_close(p)` | unload + free |

Errors use the shared `osp_status` (`OSP_OK`, `OSP_ERR_OS` for a missing library
or symbol, etc.).

## Build & test

`tools/run.sh` (POSIX) / `tools/run.ps1` (Windows) first compile
`plugins/example_plugin.c` into a shared library (`cc -shared -fPIC` / `cl /LD`),
then build and run `tests/plugin_host_test.c`, which loads that plugin through the
host, calls it, checks the exact results (`20 → 41`, `0 → 1`), and exercises the
error paths (missing library, NULL args).

**Graceful skip:** if this toolchain cannot build a shared library at all, the run
script prints a skip message and exits 0 — the CCPP02 plan's "graceful skip when
the SDK is absent" rule. On macOS a dylib named `.so` loads fine via `dlopen`, so
one plugin filename serves both Unix platforms; Windows uses `.dll`. On Linux the
host test links `-ldl`.

```sh
cd code/packages/c/plugin-host
sh tools/run.sh
```

Locally (macOS): 11 checks / 0 failed under gcc + clang; clean under ASan+UBSan;
0 leaks.

## Layout

```
plugin-host/
├── include/plugin_host/
│   ├── plugin_abi.h              # the host↔plugin contract (export marker, entry name)
│   └── host.h                    # the host API
├── src/host.c                    # host built on os_platform/dynlib (no per-OS code)
├── plugins/example_plugin.c      # a minimal conforming plugin
├── tests/plugin_host_test.c      # loads the plugin, calls it, checks results
├── tools/run.sh  · run.ps1       # build the plugin lib, then build+run the test
├── BUILD  · BUILD_windows        # per-OS build drivers
└── required_capabilities.json    # CI needs gcc, clang, cl
```
