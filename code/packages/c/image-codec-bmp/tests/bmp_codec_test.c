/*
 * bmp_codec_test.c — tests for the BMP encoder / decoder.
 * ===========================================================================
 *
 * Mirrors the Rust unit tests (header fields, BGRA byte order, round-trips, and
 * the decode errors) and adds a bottom-up-layout decode and the C ownership /
 * NULL-argument paths. Runs under ASan+UBSan so any leak or out-of-bounds parse
 * fails the build. Pixel buffers come from the composed pure-ISO pixel-container.
 */
#include "image_codec_bmp/bmp_codec.h"
#include "pixel_container.h"
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

/* A small solid-colour container. */
static PixelContainer *solid(uint32_t w, uint32_t h, uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    PixelContainer *p = pixel_new(w, h);
    pixel_fill(p, r, g, b, a);
    return p;
}

/* Encode and hand back the buffer + length (asserts success). */
static unsigned char *encode_ok(const PixelContainer *c, size_t *len) {
    unsigned char *buf = NULL;
    ISO_CHECK_EQ_INT(bmp_encode(c, &buf, len), BMP_OK);
    ISO_CHECK(buf != NULL);
    return buf;
}

static unsigned long rd_u32(const unsigned char *b, size_t o) {
    return (unsigned long)b[o] | ((unsigned long)b[o + 1] << 8) |
           ((unsigned long)b[o + 2] << 16) | ((unsigned long)b[o + 3] << 24);
}

/* Rust: encoded_magic_is_bm. */
static void test_magic(void) {
    PixelContainer *c = solid(4, 4, 0, 0, 0, 255);
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    ISO_CHECK_MEM_EQ(bmp, "BM", 2);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: encoded_file_size_correct. */
static void test_file_size(void) {
    PixelContainer *c = solid(4, 4, 0, 0, 0, 255);
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    ISO_CHECK_EQ_UINT(rd_u32(bmp, 2), len);   /* bfSize field == actual length */
    ISO_CHECK_EQ_UINT(len, 54 + 4 * 4 * 4);   /* 54 header + 64 pixels */
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: encoded_pixel_offset_is_54. */
static void test_pixel_offset(void) {
    PixelContainer *c = solid(2, 2, 0, 0, 0, 255);
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    ISO_CHECK_EQ_UINT(rd_u32(bmp, 10), 54);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: encoded_biheight_is_negative. */
static void test_biheight_negative(void) {
    PixelContainer *c = solid(3, 5, 0, 0, 0, 255);
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    /* biHeight (i32 at 22) must be -5 → two's-complement u32 = 2^32 - 5. */
    ISO_CHECK_EQ_UINT(rd_u32(bmp, 22), 0xFFFFFFFBUL);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: encoded_bit_count_is_32. */
static void test_bit_count(void) {
    PixelContainer *c = solid(1, 1, 0, 0, 0, 255);
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    ISO_CHECK_EQ_INT((int)(bmp[28] | (bmp[29] << 8)), 32);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: pixel_data_is_bgra_order. */
static void test_bgra_order(void) {
    unsigned char rgba[4];
    PixelContainer *c;
    size_t len = 0;
    unsigned char *bmp;
    rgba[0] = 1; rgba[1] = 2; rgba[2] = 3; rgba[3] = 4; /* R=1 G=2 B=3 A=4 */
    c = pixel_from_data(1, 1, rgba, 4);
    ISO_CHECK(c != NULL);
    bmp = encode_ok(c, &len);
    ISO_CHECK_EQ_INT(bmp[54], 3); /* B */
    ISO_CHECK_EQ_INT(bmp[55], 2); /* G */
    ISO_CHECK_EQ_INT(bmp[56], 1); /* R */
    ISO_CHECK_EQ_INT(bmp[57], 4); /* A */
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: round_trip_solid_colour / round_trip_checkerboard / _with_transparency. */
static void test_round_trip(void) {
    PixelContainer *solid_c = solid(4, 4, 200, 100, 50, 255);
    PixelContainer *check = pixel_new(4, 4);
    PixelContainer *trans = pixel_new(2, 2);
    PixelContainer *decoded = NULL;
    unsigned char *bmp;
    size_t len = 0;
    uint32_t x, y;

    /* solid */
    bmp = encode_ok(solid_c, &len);
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len, &decoded), BMP_OK);
    ISO_CHECK_EQ_INT(pixel_equals(decoded, solid_c), 1);
    bmp_free(bmp);
    pixel_free(decoded);
    decoded = NULL;

    /* checkerboard */
    for (y = 0; y < 4; y++) {
        for (x = 0; x < 4; x++) {
            if ((x + y) % 2 == 0) {
                pixel_set(check, x, y, 255, 255, 255, 255);
            } else {
                pixel_set(check, x, y, 0, 0, 0, 255);
            }
        }
    }
    bmp = encode_ok(check, &len);
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len, &decoded), BMP_OK);
    ISO_CHECK_EQ_INT(pixel_equals(decoded, check), 1);
    bmp_free(bmp);
    pixel_free(decoded);
    decoded = NULL;

    /* transparency preserved */
    pixel_set(trans, 0, 0, 255, 0, 0, 128);
    pixel_set(trans, 1, 0, 0, 255, 0, 0);
    pixel_set(trans, 0, 1, 0, 0, 255, 200);
    pixel_set(trans, 1, 1, 100, 100, 100, 255);
    bmp = encode_ok(trans, &len);
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len, &decoded), BMP_OK);
    ISO_CHECK_EQ_INT(pixel_equals(decoded, trans), 1);
    bmp_free(bmp);
    pixel_free(decoded);

    pixel_free(solid_c);
    pixel_free(check);
    pixel_free(trans);
}

/*
 * Bottom-up layout: flip an encoded (top-down) BMP into a positive-biHeight,
 * row-reversed file and confirm the decoder reconstructs the same image (the
 * dest_row = height-1-row branch).
 */
static void test_decode_bottom_up(void) {
    PixelContainer *orig = pixel_new(2, 2);
    PixelContainer *decoded = NULL;
    unsigned char *td;
    unsigned char *bu;
    size_t len = 0;
    size_t row_bytes = 2 * 4; /* width*4 */
    size_t i;
    pixel_set(orig, 0, 0, 10, 20, 30, 255);
    pixel_set(orig, 1, 0, 40, 50, 60, 255);
    pixel_set(orig, 0, 1, 70, 80, 90, 255);
    pixel_set(orig, 1, 1, 100, 110, 120, 255);
    td = encode_ok(orig, &len);

    bu = (unsigned char *)malloc(len);
    ISO_CHECK(bu != NULL);
    memcpy(bu, td, len);
    /* Flip biHeight sign: -2 → +2 (positive = bottom-up). */
    bu[22] = 2; bu[23] = 0; bu[24] = 0; bu[25] = 0;
    /* Reverse the two rows in the pixel data (offset 54). */
    for (i = 0; i < row_bytes; i++) {
        bu[54 + i] = td[54 + row_bytes + i];
        bu[54 + row_bytes + i] = td[54 + i];
    }
    ISO_CHECK_EQ_INT(bmp_decode(bu, len, &decoded), BMP_OK);
    ISO_CHECK_EQ_INT(pixel_equals(decoded, orig), 1); /* same image, either layout */

    bmp_free(td);
    free(bu);
    pixel_free(decoded);
    pixel_free(orig);
}

/* Rust: decode_too_short_returns_error. */
static void test_decode_too_short(void) {
    static const unsigned char d[] = {0x42, 0x4D, 0x00};
    PixelContainer *decoded = NULL;
    ISO_CHECK_EQ_INT(bmp_decode(d, sizeof(d), &decoded), BMP_ERR_TOO_SHORT);
    ISO_CHECK(decoded == NULL);
}

/* Rust: decode_wrong_magic_returns_error. */
static void test_decode_wrong_magic(void) {
    PixelContainer *c = solid(2, 2, 0, 0, 0, 255);
    PixelContainer *decoded = NULL;
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    bmp[0] = 0xFF; /* corrupt magic */
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len, &decoded), BMP_ERR_MAGIC);
    ISO_CHECK(decoded == NULL);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: decode_unsupported_bit_depth_returns_error. */
static void test_decode_bad_bit_depth(void) {
    PixelContainer *c = solid(2, 2, 0, 0, 0, 255);
    PixelContainer *decoded = NULL;
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    bmp[28] = 24; bmp[29] = 0; /* biBitCount → 24 */
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len, &decoded), BMP_ERR_BIT_DEPTH);
    ISO_CHECK(decoded == NULL);
    bmp_free(bmp);
    pixel_free(c);
}

/* Additional decode errors: compression, offset, width, truncation. */
static void test_decode_more_errors(void) {
    PixelContainer *c = solid(2, 2, 0, 0, 0, 255);
    PixelContainer *decoded = NULL;
    size_t len = 0;
    unsigned char *bmp = encode_ok(c, &len);
    unsigned char *copy = (unsigned char *)malloc(len);
    ISO_CHECK(copy != NULL);

    memcpy(copy, bmp, len);
    copy[30] = 1; /* biCompression → 1 (unsupported) */
    ISO_CHECK_EQ_INT(bmp_decode(copy, len, &decoded), BMP_ERR_COMPRESSION);

    memcpy(copy, bmp, len);
    copy[10] = 10; /* pixel offset 10 < 54 */
    ISO_CHECK_EQ_INT(bmp_decode(copy, len, &decoded), BMP_ERR_OFFSET);

    memcpy(copy, bmp, len);
    copy[18] = 0; copy[19] = 0; copy[20] = 0; copy[21] = 0; /* biWidth = 0 */
    ISO_CHECK_EQ_INT(bmp_decode(copy, len, &decoded), BMP_ERR_WIDTH);

    /* Truncated: same header, one byte short of the raster. */
    ISO_CHECK_EQ_INT(bmp_decode(bmp, len - 1, &decoded), BMP_ERR_TRUNCATED);

    ISO_CHECK(decoded == NULL);
    free(copy);
    bmp_free(bmp);
    pixel_free(c);
}

/* Rust: codec_mime_type. */
static void test_mime(void) {
    ISO_CHECK_STR_EQ(bmp_mime_type(), "image/bmp");
}

static void test_invalid_params(void) {
    PixelContainer *c = solid(1, 1, 0, 0, 0, 255);
    unsigned char *out = NULL;
    size_t len = 0;
    PixelContainer *dec = NULL;
    ISO_CHECK_EQ_INT(bmp_encode(NULL, &out, &len), BMP_ERR_INVALID);
    ISO_CHECK_EQ_INT(bmp_encode(c, NULL, &len), BMP_ERR_INVALID);
    ISO_CHECK_EQ_INT(bmp_encode(c, &out, NULL), BMP_ERR_INVALID);
    ISO_CHECK_EQ_INT(bmp_decode(NULL, 54, &dec), BMP_ERR_INVALID);
    ISO_CHECK_EQ_INT(bmp_decode((const unsigned char *)"BM", 2, NULL), BMP_ERR_INVALID);
    bmp_free(NULL);
    pixel_free(c);
}

int main(void) {
    test_magic();
    test_file_size();
    test_pixel_offset();
    test_biheight_negative();
    test_bit_count();
    test_bgra_order();
    test_round_trip();
    test_decode_bottom_up();
    test_decode_too_short();
    test_decode_wrong_magic();
    test_decode_bad_bit_depth();
    test_decode_more_errors();
    test_mime();
    test_invalid_params();
    return ISO_TEST_RESULT();
}
