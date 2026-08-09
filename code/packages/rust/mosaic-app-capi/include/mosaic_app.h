#ifndef MOSAIC_APP_H
#define MOSAIC_APP_H

#include <stddef.h>
#include <stdint.h>

#define MOSAIC_APP_PROTOCOL_VERSION 1u

#ifdef __cplusplus
extern "C" {
#endif

typedef void *mosaic_handle;

typedef struct mosaic_bytes {
    const uint8_t *ptr;
    size_t len;
} mosaic_bytes;

typedef struct mosaic_buffer {
    uint8_t *ptr;
    size_t len;
    size_t capacity;
} mosaic_buffer;

typedef uint32_t mosaic_status;

enum {
    MOSAIC_STATUS_OK = 0,
    MOSAIC_STATUS_INVALID_ARGUMENT = 1,
    MOSAIC_STATUS_DECODE_ERROR = 2,
    MOSAIC_STATUS_PROTOCOL_ERROR = 3,
    MOSAIC_STATUS_APPLICATION_ERROR = 4,
    MOSAIC_STATUS_ENCODE_ERROR = 5,
    MOSAIC_STATUS_PANIC = 6,
    MOSAIC_STATUS_POISONED = 7
};

/* On success, output buffers contain JSON. On failure they contain a bounded
 * UTF-8 diagnostic. Every returned buffer must be released exactly once before
 * its output slot is reused. */
mosaic_status mosaic_app_create(mosaic_bytes start, mosaic_handle *app,
                                mosaic_buffer *initial_update);
mosaic_status mosaic_app_dispatch(mosaic_handle app, mosaic_bytes event,
                                  mosaic_buffer *update);
mosaic_status mosaic_app_snapshot(mosaic_handle app, mosaic_buffer *snapshot);
mosaic_status mosaic_app_restore(mosaic_handle app, mosaic_bytes snapshot,
                                 mosaic_buffer *update);
void mosaic_buffer_free(mosaic_buffer buffer);
void mosaic_app_destroy(mosaic_handle app);

#ifdef __cplusplus
}
#endif

#endif
