/*
 * ppm_codec.c — Netpbm PPM (P6) encoder / decoder (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `image-codec-ppm` crate. Encoding is a header
 * `printf` plus a tight RGB copy; decoding is a small hand-written tokenizer for
 * the ASCII header (whitespace- and '#'-comment-aware, exactly as the Netpbm
 * spec and the Rust require) followed by a bounds-checked binary read.
 *
 * The one representational bridge is alpha: a `PixelContainer` is RGBA8, but PPM
 * has no alpha channel — so encode writes only R,G,B and decode fills alpha with
 * 255. Every size computation that the Rust does with `checked_mul` is guarded
 * here against `size_t` wrap and surfaced as PPM_ERR_OVERFLOW.
 */
#include "image_codec_ppm/ppm_codec.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, strlen */

/* ------------------------------------------------------------------------- *
 * Small helpers
 * ------------------------------------------------------------------------- */

/* Overflow-checked multiply: 1 on overflow, else 0 with *out set. */
static int mul_ovf(size_t a, size_t b, size_t *out) {
    if (a != 0 && b > ((size_t)-1) / a) {
        return 1;
    }
    *out = a * b;
    return 0;
}

/*
 * PPM/Netpbm whitespace: space, tab, LF, FF, CR — matching Rust's
 * `u8::is_ascii_whitespace` (which, note, does NOT include the vertical tab).
 */
static int is_ppm_ws(unsigned char b) {
    return b == ' ' || b == '\t' || b == '\n' || b == '\r' || b == '\f';
}

/* Skip ASCII whitespace and '#'-prefixed comment lines (to end of line). */
static void skip_ws_and_comments(const unsigned char *bytes, size_t len, size_t *pos) {
    for (;;) {
        while (*pos < len && is_ppm_ws(bytes[*pos])) {
            (*pos)++;
        }
        if (*pos < len && bytes[*pos] == '#') {
            while (*pos < len && bytes[*pos] != '\n') {
                (*pos)++;
            }
        } else {
            break;
        }
    }
}

/*
 * Read a whitespace-delimited token, returning its [start, end) span via *start
 * and *tok_len. Returns 0 (and leaves the span empty) when at end of data.
 */
static int read_token(const unsigned char *bytes, size_t len, size_t *pos,
                      size_t *start, size_t *tok_len) {
    skip_ws_and_comments(bytes, len, pos);
    if (*pos >= len) {
        return 0;
    }
    *start = *pos;
    while (*pos < len && !is_ppm_ws(bytes[*pos])) {
        (*pos)++;
    }
    *tok_len = *pos - *start;
    return 1;
}

/*
 * Read a decimal-integer token into *value. Returns 0 when there is no token, the
 * token has a non-digit, or the value overflows size_t (mirroring Rust's
 * `str::parse::<usize>()` returning None on any of these).
 */
static int read_int(const unsigned char *bytes, size_t len, size_t *pos, size_t *value) {
    size_t start = 0;
    size_t tok_len = 0;
    size_t i;
    size_t v = 0;
    if (!read_token(bytes, len, pos, &start, &tok_len) || tok_len == 0) {
        return 0;
    }
    for (i = 0; i < tok_len; i++) {
        unsigned char c = bytes[start + i];
        unsigned digit;
        if (c < '0' || c > '9') {
            return 0;
        }
        digit = (unsigned)(c - '0');
        /* v = v*10 + digit, guarded against size_t wrap. */
        if (v > (((size_t)-1) - digit) / 10) {
            return 0;
        }
        v = v * 10 + digit;
    }
    *value = v;
    return 1;
}

/* ------------------------------------------------------------------------- *
 * Public API
 * ------------------------------------------------------------------------- */

const char *ppm_mime_type(void) {
    return "image/x-portable-pixmap";
}

void ppm_free(unsigned char *buf) {
    free(buf);
}

ppm_status ppm_encode(const PixelContainer *c, unsigned char **out, size_t *out_len) {
    char header[48];
    int header_written;
    size_t header_len;
    size_t w, h;
    size_t pixel_count, pixel_bytes, total;
    unsigned char *buf;
    size_t o;
    uint32_t x, y;
    if (!c || !out || !out_len) {
        return PPM_ERR_INVALID;
    }
    w = (size_t)pixel_width(c);
    h = (size_t)pixel_height(c);

    /* Header "P6\n<w> <h>\n255\n". The values are uint32_t, so at most 10 digits
     * each — well within the 48-byte buffer; snprintf never truncates here. */
    header_written = snprintf(header, sizeof(header), "P6\n%lu %lu\n255\n",
                              (unsigned long)w, (unsigned long)h);
    if (header_written < 0 || (size_t)header_written >= sizeof(header)) {
        return PPM_ERR_OVERFLOW; /* unreachable for uint32 dims, but stay total */
    }
    header_len = (size_t)header_written;

    /* pixel_bytes = w*h*3; total = header_len + pixel_bytes — all size_t-checked. */
    if (mul_ovf(w, h, &pixel_count) || mul_ovf(pixel_count, 3, &pixel_bytes)) {
        return PPM_ERR_OVERFLOW;
    }
    if (pixel_bytes > ((size_t)-1) - header_len) {
        return PPM_ERR_OVERFLOW;
    }
    total = header_len + pixel_bytes;

    buf = (unsigned char *)malloc(total > 0 ? total : 1);
    if (!buf) {
        return PPM_ERR_NOMEM;
    }
    memcpy(buf, header, header_len);
    o = header_len;

    /* Three bytes per pixel, row-major, dropping alpha. */
    for (y = 0; y < (uint32_t)h; y++) {
        for (x = 0; x < (uint32_t)w; x++) {
            uint8_t rgba[4];
            pixel_at(c, x, y, rgba);
            buf[o++] = rgba[0];
            buf[o++] = rgba[1];
            buf[o++] = rgba[2];
        }
    }

    *out = buf;
    *out_len = total;
    return PPM_OK;
}

ppm_status ppm_decode(const unsigned char *bytes, size_t len, PixelContainer **out) {
    size_t pos = 0;
    size_t start = 0, tok_len = 0;
    size_t width, height, maxval;
    size_t pixel_count, needed, data_len;
    unsigned char *data;
    size_t i, dp;
    PixelContainer *container;
    if (!bytes || !out) {
        return PPM_ERR_INVALID;
    }

    /* Magic must be exactly "P6". */
    if (!read_token(bytes, len, &pos, &start, &tok_len) ||
        tok_len != 2 || bytes[start] != 'P' || bytes[start + 1] != '6') {
        return PPM_ERR_MAGIC;
    }

    skip_ws_and_comments(bytes, len, &pos);
    if (!read_int(bytes, len, &pos, &width)) {
        return PPM_ERR_DIMENSIONS;
    }
    skip_ws_and_comments(bytes, len, &pos);
    if (!read_int(bytes, len, &pos, &height)) {
        return PPM_ERR_DIMENSIONS;
    }
    skip_ws_and_comments(bytes, len, &pos);
    if (!read_int(bytes, len, &pos, &maxval)) {
        return PPM_ERR_MAXVAL;
    }
    if (maxval != 255) {
        return PPM_ERR_MAXVAL;
    }

    /* Exactly one whitespace byte separates the header from the binary data. */
    if (pos >= len) {
        return PPM_ERR_TRUNCATED;
    }
    pos += 1;

    /* Sizes: pixel_count = w*h, needed = *3 (input), data_len = *4 (RGBA out). */
    if (mul_ovf(width, height, &pixel_count)) {
        return PPM_ERR_DIMENSIONS;
    }
    if (mul_ovf(pixel_count, 3, &needed)) {
        return PPM_ERR_OVERFLOW;
    }
    if (pos > len || len - pos < needed) {
        return PPM_ERR_TRUNCATED;
    }
    /* pixel_container is a uint32×uint32 buffer; reject dims that would truncate. */
    if (width > 0xFFFFFFFFu || height > 0xFFFFFFFFu) {
        return PPM_ERR_DIMENSIONS;
    }
    if (mul_ovf(pixel_count, 4, &data_len)) {
        return PPM_ERR_OVERFLOW;
    }

    data = (unsigned char *)malloc(data_len > 0 ? data_len : 1);
    if (!data) {
        return PPM_ERR_NOMEM;
    }
    /* Expand each RGB triple to RGBA with alpha 255. */
    dp = 0;
    for (i = 0; i < pixel_count; i++) {
        data[dp++] = bytes[pos];
        data[dp++] = bytes[pos + 1];
        data[dp++] = bytes[pos + 2];
        data[dp++] = 255;
        pos += 3;
    }

    container = pixel_from_data((uint32_t)width, (uint32_t)height, data, data_len);
    free(data);
    if (!container) {
        return PPM_ERR_NOMEM;
    }
    *out = container;
    return PPM_OK;
}
