/*
 * image_codec_bmp/bmp_codec.h — BMP (Windows bitmap) encoder / decoder.
 * ===========================================================================
 *
 * The C port of the Rust `image-codec-bmp` crate, and the fifth bucket-A port of
 * the CCPP02 campaign: a pure-ISO crate that needs no OS, so it rides the
 * `iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).
 *
 * A 32-bit BGRA BMP is a fixed 54-byte header then raw pixels:
 *
 *     BITMAPFILEHEADER  (14 bytes) — "BM", file size, pixel-data offset
 *     BITMAPINFOHEADER  (40 bytes) — width, height, bit depth, compression, …
 *     pixel data        (width*height*4 bytes, BGRA)
 *
 * All integers are little-endian. Two format wrinkles the codec handles:
 *
 *   - BGRA vs RGBA. BMP stores blue first; a `PixelContainer` is RGBA. The only
 *     transform is swapping R and B per pixel.
 *   - Row order. Classic BMP is *bottom-up* (last image row first in the file);
 *     a NEGATIVE biHeight marks *top-down*. The encoder always writes a negative
 *     biHeight (top-down, matching the container, no row reversal); the decoder
 *     accepts both.
 *
 * COMPOSES `c/pixel-container` (an RGBA8 buffer). This package is itself pure-ISO
 * — it compiles pixel-container's source in rather than linking anything.
 *
 * OWNERSHIP. `bmp_encode` hands back a fresh byte buffer the caller frees with
 * `bmp_free`; `bmp_decode` hands back a fresh `PixelContainer` freed with
 * `pixel_free`.
 */
#ifndef IMAGE_CODEC_BMP_BMP_CODEC_H
#define IMAGE_CODEC_BMP_BMP_CODEC_H

#include <stddef.h> /* size_t */

#include "pixel_container.h" /* PixelContainer */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Every result a codec operation can produce. The Rust returns `Result<_, String>`
 * with a message; here each failure is a distinct code (Rust message beside it).
 */
typedef enum {
    BMP_OK = 0,
    BMP_ERR_TOO_SHORT,   /* "file too short" (< 54-byte header) */
    BMP_ERR_MAGIC,       /* "invalid magic" (not "BM") */
    BMP_ERR_OFFSET,      /* "pixel offset is before end of header" */
    BMP_ERR_WIDTH,       /* "invalid width" (<= 0) */
    BMP_ERR_HEIGHT,      /* "invalid height" (i32::MIN, or 0) */
    BMP_ERR_BIT_DEPTH,   /* "unsupported bit depth" (only 32 is supported) */
    BMP_ERR_COMPRESSION, /* "unsupported compression" (only BI_RGB / 0) */
    BMP_ERR_TRUNCATED,   /* "pixel data truncated" */
    BMP_ERR_OVERFLOW,    /* a width*height*4 or offset+size overflowed size_t */
    BMP_ERR_NOMEM,       /* allocation failure */
    BMP_ERR_INVALID      /* NULL argument */
} bmp_status;

/* The BMP MIME type, "image/bmp" (a static string; do not free). */
const char *bmp_mime_type(void);

/*
 * bmp_encode — encode a container to 32-bit BGRA BMP bytes (top-down). On success
 * *out points to a fresh buffer of *out_len bytes (release with bmp_free).
 * BMP_ERR_INVALID (NULL arg), BMP_ERR_OVERFLOW (dimensions overflow), BMP_ERR_NOMEM.
 */
bmp_status bmp_encode(const PixelContainer *c, unsigned char **out, size_t *out_len);

/*
 * bmp_decode — decode BMP bytes into a fresh container (release with pixel_free).
 * Accepts top-down or bottom-up 32-bit BI_RGB BMPs. The various BMP_ERR_* codes
 * report a malformed or unsupported file.
 */
bmp_status bmp_decode(const unsigned char *bytes, size_t len, PixelContainer **out);

/* Free a buffer returned by bmp_encode (safe on NULL). */
void bmp_free(unsigned char *buf);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* IMAGE_CODEC_BMP_BMP_CODEC_H */
