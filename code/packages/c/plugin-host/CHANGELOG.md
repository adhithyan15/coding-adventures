# Changelog

All notable changes to the `plugin-host` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — `dlopen` plugin host** (CCPP02 Phase 4, the lane's
  representative). Loads a plugin shared library at run time, resolves a known
  entry point, and calls it — built entirely on the os-platform `dynlib`
  primitive, so it has no per-OS backend of its own.
  - `plugin_host/plugin_abi.h`: the host↔plugin contract — `OSP_PLUGIN_EXPORT`
    (dllexport on Windows / default visibility on POSIX), `OSP_PLUGIN_ENTRY_NAME`,
    and the `int -> int` entry signature.
  - `plugin_host/host.h` + `src/host.c`: `osp_plugin_open` (dynlib_open + resolve
    the entry, converting the void* to a function pointer via memcpy) /
    `osp_plugin_call` / `osp_plugin_close` (dynlib_close + free). Reuses
    `osp_status`.
  - `plugins/example_plugin.c`: a minimal conforming plugin (`x -> x*2 + 1`).
  - `tools/run.sh` / `run.ps1`: build the plugin into a shared library
    (`cc -shared -fPIC` / `cl /LD`), then build & run the host test — with a
    **graceful skip** (exit 0) if a shared library cannot be built. On macOS a
    dylib named `.so` loads via `dlopen`, so one filename serves both Unix
    platforms; Linux links `-ldl`.
  - `tests/plugin_host_test.c`: loads the example plugin through the host, calls
    it (`20 → 41`, `0 → 1`), and checks the error paths (missing library, NULL
    args). Verified under ASan+UBSan with 0 leaks.
