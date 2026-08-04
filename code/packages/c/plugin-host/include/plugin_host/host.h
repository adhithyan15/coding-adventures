/*
 * plugin_host/host.h — load a plugin, call its entry point, unload it.
 * ===========================================================================
 *
 * The host side of the plugin ABI (plugin_abi.h). It is a thin, typed wrapper
 * over the os-platform `dynlib` primitive: osp_plugin_open loads the shared
 * library and resolves OSP_PLUGIN_ENTRY_NAME once; osp_plugin_call invokes it;
 * osp_plugin_close unloads and frees. All the OS-specific loading lives in
 * dynlib — this layer just knows the plugin contract.
 *
 * Reuses the os-platform error vocabulary (osp_status).
 */
#ifndef PLUGIN_HOST_HOST_H
#define PLUGIN_HOST_HOST_H

#include "os_platform/status.h"       /* osp_status */
#include "plugin_host/plugin_abi.h"   /* the ABI */

#ifdef __cplusplus
extern "C" {
#endif

/* An opaque loaded-plugin handle. Created by osp_plugin_open, freed by
 * osp_plugin_close. */
typedef struct osp_plugin osp_plugin;

/*
 * osp_plugin_open — load the plugin shared library at `path` and resolve its
 * entry point. Returns OSP_ERR_INVAL (NULL args), OSP_ERR_NOMEM, or OSP_ERR_OS
 * (the library will not load, or it lacks the OSP_PLUGIN_ENTRY_NAME symbol).
 */
osp_status osp_plugin_open(const char *path, osp_plugin **out);

/*
 * osp_plugin_call — invoke the plugin's entry point with `arg`, writing its
 * result to *out. OSP_ERR_INVAL if p or out is NULL.
 */
osp_status osp_plugin_call(osp_plugin *p, int arg, int *out);

/*
 * osp_plugin_close — unload the plugin and free the handle. OSP_ERR_INVAL if p
 * is NULL, OSP_ERR_OS if the unload fails (the handle is freed either way).
 */
osp_status osp_plugin_close(osp_plugin *p);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* PLUGIN_HOST_HOST_H */
