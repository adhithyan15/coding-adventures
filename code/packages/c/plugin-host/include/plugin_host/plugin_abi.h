/*
 * plugin_host/plugin_abi.h — the contract between a plugin host and its plugins.
 * ===========================================================================
 *
 * CCPP02 Phase 4: the representative that ties the whole lane together. A plugin
 * host loads code at run time (the `dynlib` primitive), resolves a known entry
 * point, and calls it — the pattern behind editor extensions, codec plugins, and
 * language-runtime FFI. This header is the shared ABI both sides agree on:
 *
 *   - a plugin is a shared library that EXPORTS one function named
 *     OSP_PLUGIN_ENTRY_NAME with the osp_plugin_entry_fn signature;
 *   - the host LOADS that library and resolves that symbol.
 *
 * A conforming plugin is just:
 *
 *     #include "plugin_host/plugin_abi.h"
 *     OSP_PLUGIN_EXPORT int osp_plugin_entry(int x) { return x * 2 + 1; }
 *
 * The export marker makes the symbol visible to the loader: __declspec(dllexport)
 * on Windows (where nothing is exported by default), and nothing on POSIX (where
 * default visibility already exports it).
 */
#ifndef PLUGIN_HOST_PLUGIN_ABI_H
#define PLUGIN_HOST_PLUGIN_ABI_H

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#define OSP_PLUGIN_EXPORT __declspec(dllexport)
#else
#define OSP_PLUGIN_EXPORT
#endif

/* The symbol the host resolves. */
#define OSP_PLUGIN_ENTRY_NAME "osp_plugin_entry"

/* The signature the host expects and a plugin provides: int -> int. */
typedef int (*osp_plugin_entry_fn)(int);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* PLUGIN_HOST_PLUGIN_ABI_H */
