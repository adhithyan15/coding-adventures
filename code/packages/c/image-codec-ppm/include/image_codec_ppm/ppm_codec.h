/*
 * image_codec_ppm/ppm_codec.h — Netpbm PPM (P6) image encoder / decoder.
 * ===========================================================================
 *
 * The C port of the Rust `image-codec-ppm` crate, and the fourth bucket-A port of
 * the CCPP02 campaign: a pure-ISO crate that needs no OS, so it rides the
 * `iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).
 *
 * PPM P6 is the simplest real image format — a few lines of ASCII header, then
 * raw RGB, three bytes per pixel:
 *
 *     P6\n
 *     <width> <height>\n
 *     255\n
 *     <width*height*3 raw bytes: R G B per pixel, row-major from the top-left>
 *
 * No compression, no metadata, no padding. Files this encoder writes are read by
 * ImageMagick / ffmpeg / any Netpbm tool, and vice-versa.
 *
 * COMPOSES `c/pixel-container`. Pixels live in a `PixelContainer` (an RGBA8
 * buffer from the pure-ISO `pixel-container` package). PPM has no alpha: encode
 * DROPS the alpha byte, and decode sets every pixel's alpha to 255 (opaque). This
 * package is itself pure-ISO — it composes pixel-container's source rather than
 * linking anything.
 *
 * OWNERSHIP. `ppm_encode` hands back a fresh byte buffer the caller frees with
 * `ppm_free`. `ppm_decode` hands back a fresh `PixelContainer` the caller frees
 * with `pixel_free` (from pixel-container).
 */
#ifndef IMAGE_CODEC_PPM_PPM_CODEC_H
#define IMAGE_CODEC_PPM_PPM_CODEC_H

#include <stddef.h> /* size_t */

#include "pixel_container.h" /* PixelContainer */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Every result a codec operation can produce. The Rust returns `Result<_, String>`
 * with a human-readable message; here each failure is a distinct code (the Rust
 * message text is documented beside it).
 */
typedef enum {
    PPM_OK = 0,
    PPM_ERR_MAGIC,      /* "invalid magic, expected P6" */
    PPM_ERR_DIMENSIONS, /* "invalid dimensions" / dimension overflow */
    PPM_ERR_MAXVAL,     /* "unsupported max value" (only 255 is supported) */
    PPM_ERR_TRUNCATED,  /* "pixel data truncated" */
    PPM_ERR_OVERFLOW,   /* a width*height*channels size overflowed size_t */
    PPM_ERR_NOMEM,      /* allocation failure */
    PPM_ERR_INVALID     /* NULL argument */
} ppm_status;

/* The PPM MIME type, "image/x-portable-pixmap" (a static string; do not free). */
const char *ppm_mime_type(void);

/*
 * ppm_encode — encode a container to PPM P6 bytes. On success *out points to a
 * fresh buffer of *out_len bytes (release with ppm_free) holding the header plus
 * RGB pixel data (alpha dropped). PPM_ERR_INVALID (NULL arg), PPM_ERR_OVERFLOW
 * (dimensions overflow size_t), PPM_ERR_NOMEM.
 */
ppm_status ppm_encode(const PixelContainer *c, unsigned char **out, size_t *out_len);

/*
 * ppm_decode — decode PPM P6 bytes into a fresh container (release with
 * pixel_free); decoded pixels have alpha 255. PPM_ERR_MAGIC / _DIMENSIONS /
 * _MAXVAL / _TRUNCATED / _OVERFLOW / _NOMEM / _INVALID as appropriate.
 */
ppm_status ppm_decode(const unsigned char *bytes, size_t len, PixelContainer **out);

/* Free a buffer returned by ppm_encode (safe on NULL). */
void ppm_free(unsigned char *buf);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* IMAGE_CODEC_PPM_PPM_CODEC_H */
