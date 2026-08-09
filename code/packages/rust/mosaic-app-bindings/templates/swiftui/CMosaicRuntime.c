#include "CMosaicRuntime.h"

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MOSAIC_BINDING_UNAVAILABLE 255u
#define MOSAIC_BINDING_ERROR_CAPACITY 512u

typedef mosaic_binding_status (*mosaic_create_fn)(
    mosaic_binding_bytes, mosaic_binding_app *, mosaic_binding_buffer *);
typedef mosaic_binding_status (*mosaic_dispatch_fn)(
    mosaic_binding_app, mosaic_binding_bytes, mosaic_binding_buffer *);
typedef mosaic_binding_status (*mosaic_snapshot_fn)(
    mosaic_binding_app, mosaic_binding_buffer *);
typedef mosaic_binding_status (*mosaic_restore_fn)(
    mosaic_binding_app, mosaic_binding_bytes, mosaic_binding_buffer *);
typedef void (*mosaic_buffer_free_fn)(mosaic_binding_buffer);
typedef void (*mosaic_destroy_fn)(mosaic_binding_app);

struct mosaic_binding_runtime {
    void *library;
    mosaic_create_fn create;
    mosaic_dispatch_fn dispatch;
    mosaic_snapshot_fn snapshot;
    mosaic_restore_fn restore;
    mosaic_buffer_free_fn buffer_free;
    mosaic_destroy_fn destroy;
    char error[MOSAIC_BINDING_ERROR_CAPACITY];
};

static void set_error(mosaic_binding_runtime *runtime, const char *message) {
    snprintf(runtime->error, sizeof(runtime->error), "%s",
             message == NULL ? "unknown dynamic-loader error" : message);
}

static int resolve_symbols(mosaic_binding_runtime *runtime) {
#define RESOLVE(field, symbol)                                                   \
    do {                                                                         \
        dlerror();                                                               \
        *(void **)(&runtime->field) = dlsym(runtime->library, symbol);           \
        const char *error = dlerror();                                            \
        if (error != NULL || runtime->field == NULL) {                           \
            set_error(runtime, error == NULL ? "missing " symbol : error);      \
            return 0;                                                            \
        }                                                                        \
    } while (0)

    RESOLVE(create, "mosaic_app_create");
    RESOLVE(dispatch, "mosaic_app_dispatch");
    RESOLVE(snapshot, "mosaic_app_snapshot");
    RESOLVE(restore, "mosaic_app_restore");
    RESOLVE(buffer_free, "mosaic_buffer_free");
    RESOLVE(destroy, "mosaic_app_destroy");
#undef RESOLVE
    runtime->error[0] = '\0';
    return 1;
}

static int try_library(mosaic_binding_runtime *runtime, const char *path) {
    runtime->create = NULL;
    runtime->dispatch = NULL;
    runtime->snapshot = NULL;
    runtime->restore = NULL;
    runtime->buffer_free = NULL;
    runtime->destroy = NULL;
    runtime->library = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (runtime->library == NULL) {
        set_error(runtime, dlerror());
        return 0;
    }
    if (resolve_symbols(runtime)) {
        return 1;
    }
    dlclose(runtime->library);
    runtime->library = NULL;
    return 0;
}

mosaic_binding_runtime *mosaic_binding_open(const char *library_path) {
    mosaic_binding_runtime *runtime = calloc(1, sizeof(*runtime));
    if (runtime == NULL) {
        return NULL;
    }

    if (library_path != NULL && library_path[0] != '\0') {
        try_library(runtime, library_path);
        return runtime;
    }

    if (try_library(runtime, NULL)) {
        return runtime;
    }
    if (try_library(runtime, "libmosaic_app.dylib")) {
        return runtime;
    }
    try_library(runtime, "mosaic_app.dylib");
    return runtime;
}

int mosaic_binding_is_ready(const mosaic_binding_runtime *runtime) {
    return runtime != NULL && runtime->library != NULL && runtime->create != NULL;
}

const char *mosaic_binding_error(const mosaic_binding_runtime *runtime) {
    if (runtime == NULL) {
        return "unable to allocate Mosaic runtime loader";
    }
    return runtime->error;
}

mosaic_binding_status mosaic_binding_create(
    mosaic_binding_runtime *runtime,
    mosaic_binding_bytes start,
    mosaic_binding_app *app,
    mosaic_binding_buffer *initial_update) {
    if (!mosaic_binding_is_ready(runtime)) return MOSAIC_BINDING_UNAVAILABLE;
    return runtime->create(start, app, initial_update);
}

mosaic_binding_status mosaic_binding_dispatch(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_bytes event,
    mosaic_binding_buffer *update) {
    if (!mosaic_binding_is_ready(runtime)) return MOSAIC_BINDING_UNAVAILABLE;
    return runtime->dispatch(app, event, update);
}

mosaic_binding_status mosaic_binding_snapshot(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_buffer *snapshot) {
    if (!mosaic_binding_is_ready(runtime)) return MOSAIC_BINDING_UNAVAILABLE;
    return runtime->snapshot(app, snapshot);
}

mosaic_binding_status mosaic_binding_restore(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_bytes snapshot,
    mosaic_binding_buffer *update) {
    if (!mosaic_binding_is_ready(runtime)) return MOSAIC_BINDING_UNAVAILABLE;
    return runtime->restore(app, snapshot, update);
}

void mosaic_binding_buffer_free(
    mosaic_binding_runtime *runtime,
    mosaic_binding_buffer buffer) {
    if (mosaic_binding_is_ready(runtime)) runtime->buffer_free(buffer);
}

void mosaic_binding_destroy(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app) {
    if (mosaic_binding_is_ready(runtime) && app != NULL) runtime->destroy(app);
}

void mosaic_binding_close(mosaic_binding_runtime *runtime) {
    if (runtime == NULL) return;
    if (runtime->library != NULL) dlclose(runtime->library);
    free(runtime);
}
