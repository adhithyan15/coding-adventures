#ifndef C_MOSAIC_RUNTIME_H
#define C_MOSAIC_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void *mosaic_binding_app;
typedef uint32_t mosaic_binding_status;

typedef struct mosaic_binding_bytes {
    const uint8_t *ptr;
    size_t len;
} mosaic_binding_bytes;

typedef struct mosaic_binding_buffer {
    uint8_t *ptr;
    size_t len;
    size_t capacity;
} mosaic_binding_buffer;

typedef struct mosaic_binding_runtime mosaic_binding_runtime;

mosaic_binding_runtime *mosaic_binding_open(const char *library_path);
int mosaic_binding_is_ready(const mosaic_binding_runtime *runtime);
const char *mosaic_binding_error(const mosaic_binding_runtime *runtime);

mosaic_binding_status mosaic_binding_create(
    mosaic_binding_runtime *runtime,
    mosaic_binding_bytes start,
    mosaic_binding_app *app,
    mosaic_binding_buffer *initial_update);
mosaic_binding_status mosaic_binding_dispatch(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_bytes event,
    mosaic_binding_buffer *update);
mosaic_binding_status mosaic_binding_snapshot(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_buffer *snapshot);
mosaic_binding_status mosaic_binding_restore(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app,
    mosaic_binding_bytes snapshot,
    mosaic_binding_buffer *update);
void mosaic_binding_buffer_free(
    mosaic_binding_runtime *runtime,
    mosaic_binding_buffer buffer);
void mosaic_binding_destroy(
    mosaic_binding_runtime *runtime,
    mosaic_binding_app app);
void mosaic_binding_close(mosaic_binding_runtime *runtime);

#ifdef __cplusplus
}
#endif

#endif
