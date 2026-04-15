#ifndef CODING_ADVENTURES_PAINT_CODEC_PNG_C_H
#define CODING_ADVENTURES_PAINT_CODEC_PNG_C_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct paint_encoded_bytes_t {
    uint8_t *data;
    size_t len;
} paint_encoded_bytes_t;

uint8_t paint_codec_png_encode_rgba8(
    uint32_t width,
    uint32_t height,
    const uint8_t *rgba_bytes,
    size_t rgba_len,
    paint_encoded_bytes_t *out_bytes
);

void paint_codec_png_free_bytes(uint8_t *data, size_t len);

#ifdef __cplusplus
}
#endif

#endif
