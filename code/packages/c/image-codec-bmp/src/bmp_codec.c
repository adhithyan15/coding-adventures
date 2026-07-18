/*
 * bmp_codec.c — BMP (Windows bitmap) encoder / decoder (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `image-codec-bmp` crate. Encoding lays down the
 * fixed 54-byte header (all little-endian) and then the BGRA raster; decoding
 * reads the header fields at their fixed offsets, validates them, and copies the
 * raster back into a `PixelContainer` (RGBA), honouring the top-down / bottom-up
 * row direction.
 *
 * Every size the Rust computes with `checked_mul`/`checked_add` is guarded here
 * against `size_t` wrap and surfaced as BMP_ERR_OVERFLOW. The decoder parses
 * untrusted bytes, so every raster read is proven in-bounds by the up-front
 * `len >= pixel_offset + width*height*4` check.
 */
#include "image_codec_bmp/bmp_codec.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy */

/* ------------------------------------------------------------------------- *
 * Little-endian byte helpers
 * ------------------------------------------------------------------------- */

/* Append helpers write into buf at *o, advancing it. */
static void put_u16(unsigned char *buf, size_t *o, unsigned v) {
    buf[(*o)++] = (unsigned char)(v & 0xFF);
    buf[(*o)++] = (unsigned char)((v >> 8) & 0xFF);
}
static void put_u32(unsigned char *buf, size_t *o, unsigned long v) {
    buf[(*o)++] = (unsigned char)(v & 0xFF);
    buf[(*o)++] = (unsigned char)((v >> 8) & 0xFF);
    buf[(*o)++] = (unsigned char)((v >> 16) & 0xFF);
    buf[(*o)++] = (unsigned char)((v >> 24) & 0xFF);
}

/* Read helpers pull fixed-offset little-endian fields (offsets are all < 54, so
 * the caller has already ensured len >= 54). */
static unsigned read_u16(const unsigned char *b, size_t off) {
    return (unsigned)b[off] | ((unsigned)b[off + 1] << 8);
}
static unsigned long read_u32(const unsigned char *b, size_t off) {
    return (unsigned long)b[off] | ((unsigned long)b[off + 1] << 8) |
           ((unsigned long)b[off + 2] << 16) | ((unsigned long)b[off + 3] << 24);
}
/* Reinterpret a little-endian u32 as a two's-complement i32, portably (no
 * implementation-defined out-of-range unsigned→signed cast). */
static long read_i32(const unsigned char *b, size_t off) {
    unsigned long u = read_u32(b, off);
    if (u <= 0x7FFFFFFFUL) {
        return (long)u;
    }
    return -(long)(0xFFFFFFFFUL - u) - 1;
}

/* Overflow-checked multiply: 1 on overflow, else 0 with *out set. */
static int mul_ovf(size_t a, size_t b, size_t *out) {
    if (a != 0 && b > ((size_t)-1) / a) {
        return 1;
    }
    *out = a * b;
    return 0;
}

/* ------------------------------------------------------------------------- *
 * Public API
 * ------------------------------------------------------------------------- */

const char *bmp_mime_type(void) {
    return "image/bmp";
}

void bmp_free(unsigned char *buf) {
    free(buf);
}

bmp_status bmp_encode(const PixelContainer *c, unsigned char **out, size_t *out_len) {
    size_t w, h, pixel_count, pixel_bytes, file_size, o;
    unsigned char *buf;
    uint32_t x, y;
    if (!c || !out || !out_len) {
        return BMP_ERR_INVALID;
    }
    w = (size_t)pixel_width(c);
    h = (size_t)pixel_height(c);

    /* pixel_bytes = w*h*4; file_size = 54 + pixel_bytes — all size_t-checked. */
    if (mul_ovf(w, h, &pixel_count) || mul_ovf(pixel_count, 4, &pixel_bytes)) {
        return BMP_ERR_OVERFLOW;
    }
    if (pixel_bytes > ((size_t)-1) - 54) {
        return BMP_ERR_OVERFLOW;
    }
    file_size = 54 + pixel_bytes;

    buf = (unsigned char *)malloc(file_size);
    if (!buf) {
        return BMP_ERR_NOMEM;
    }
    o = 0;

    /* --- BITMAPFILEHEADER (14 bytes) --- */
    buf[o++] = 'B';
    buf[o++] = 'M';
    put_u32(buf, &o, (unsigned long)(file_size & 0xFFFFFFFFUL)); /* bfSize */
    put_u16(buf, &o, 0);  /* bfReserved1 */
    put_u16(buf, &o, 0);  /* bfReserved2 */
    put_u32(buf, &o, 54); /* bfOffBits — pixel data at offset 54 */

    /* --- BITMAPINFOHEADER (40 bytes) --- */
    put_u32(buf, &o, 40);                                 /* biSize */
    put_u32(buf, &o, (unsigned long)(w & 0xFFFFFFFFUL));  /* biWidth (positive) */
    /* biHeight NEGATIVE → top-down scanlines. -(height) as i32 = two's complement. */
    put_u32(buf, &o, (unsigned long)((0UL - (unsigned long)h) & 0xFFFFFFFFUL));
    put_u16(buf, &o, 1);  /* biPlanes */
    put_u16(buf, &o, 32); /* biBitCount — 32bpp BGRA */
    put_u32(buf, &o, 0);  /* biCompression — BI_RGB */
    put_u32(buf, &o, (unsigned long)(pixel_bytes & 0xFFFFFFFFUL)); /* biSizeImage */
    put_u32(buf, &o, 0);  /* biXPelsPerMeter */
    put_u32(buf, &o, 0);  /* biYPelsPerMeter */
    put_u32(buf, &o, 0);  /* biClrUsed */
    put_u32(buf, &o, 0);  /* biClrImportant */

    /* --- Pixel data: RGBA → BGRA (swap R and B), top-down. --- */
    for (y = 0; y < (uint32_t)h; y++) {
        for (x = 0; x < (uint32_t)w; x++) {
            uint8_t rgba[4];
            pixel_at(c, x, y, rgba);
            buf[o++] = rgba[2]; /* Blue */
            buf[o++] = rgba[1]; /* Green */
            buf[o++] = rgba[0]; /* Red */
            buf[o++] = rgba[3]; /* Alpha */
        }
    }

    *out = buf;
    *out_len = file_size;
    return BMP_OK;
}

bmp_status bmp_decode(const unsigned char *bytes, size_t len, PixelContainer **out) {
    size_t pixel_offset;
    long bi_width, bi_height;
    uint32_t width, height;
    int top_down;
    unsigned bit_count;
    unsigned long compression;
    size_t pixel_count, pixel_bytes, pixel_end;
    PixelContainer *container;
    uint32_t row, col;
    if (!bytes || !out) {
        return BMP_ERR_INVALID;
    }
    /* The fixed 54-byte header must be present before any field read. */
    if (len < 54) {
        return BMP_ERR_TOO_SHORT;
    }
    if (bytes[0] != 'B' || bytes[1] != 'M') {
        return BMP_ERR_MAGIC;
    }

    pixel_offset = (size_t)read_u32(bytes, 10);
    if (pixel_offset < 54) {
        return BMP_ERR_OFFSET;
    }

    bi_width = read_i32(bytes, 18);
    bi_height = read_i32(bytes, 22);
    if (bi_width <= 0) {
        return BMP_ERR_WIDTH;
    }
    /* i32::MIN's magnitude (2^31) doesn't fit a positive i32; reject it (matches
     * the Rust unsigned_abs guard). read_i32 yields it as -2147483648L. */
    if (bi_height == -2147483647L - 1L) {
        return BMP_ERR_HEIGHT;
    }
    width = (uint32_t)bi_width;
    height = (uint32_t)(bi_height < 0 ? -bi_height : bi_height);
    top_down = (bi_height < 0);
    if (height == 0) {
        return BMP_ERR_HEIGHT;
    }

    bit_count = read_u16(bytes, 28);
    if (bit_count != 32) {
        return BMP_ERR_BIT_DEPTH;
    }
    compression = read_u32(bytes, 30);
    if (compression != 0) {
        return BMP_ERR_COMPRESSION;
    }

    /* pixel_bytes = width*height*4; pixel_end = offset + pixel_bytes (checked). */
    if (mul_ovf((size_t)width, (size_t)height, &pixel_count) ||
        mul_ovf(pixel_count, 4, &pixel_bytes)) {
        return BMP_ERR_OVERFLOW;
    }
    if (pixel_bytes > ((size_t)-1) - pixel_offset) {
        return BMP_ERR_OVERFLOW;
    }
    pixel_end = pixel_offset + pixel_bytes;
    if (len < pixel_end) {
        return BMP_ERR_TRUNCATED;
    }

    container = pixel_new(width, height);
    if (!container) {
        return BMP_ERR_NOMEM;
    }
    /* Copy BGRA → RGBA, mapping file rows to image rows by direction. Every
     * file_idx is < pixel_end <= len (pixel_bytes did not overflow), so in-bounds. */
    for (row = 0; row < height; row++) {
        uint32_t dest_row = top_down ? row : (height - 1 - row);
        for (col = 0; col < width; col++) {
            size_t file_idx =
                pixel_offset + ((size_t)row * width + col) * 4;
            uint8_t b = bytes[file_idx];
            uint8_t g = bytes[file_idx + 1];
            uint8_t r = bytes[file_idx + 2];
            uint8_t a = bytes[file_idx + 3];
            pixel_set(container, col, dest_row, r, g, b, a);
        }
    }

    *out = container;
    return BMP_OK;
}
