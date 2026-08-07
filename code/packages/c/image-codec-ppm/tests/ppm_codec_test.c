/*
 * ppm_codec_test.c — tests for the PPM (P6) encoder / decoder.
 * ===========================================================================
 *
 * Mirrors the Rust unit tests (P6 header, dimensions in header, exact encoded
 * size, alpha dropped, round-trips, comment handling, and the three decode
 * errors) and adds the C ownership / NULL-argument paths. Runs under ASan+UBSan
 * so any leak or out-of-bounds parse fails the build. Encoding composes the
 * pure-ISO pixel-container package for the pixel buffers.
 */
#include "image_codec_ppm/ppm_codec.h"
#include "pixel_container.h"
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

/* Encode a container and hand back the buffer + length (asserts success). */
static unsigned char *encode_ok(const PixelContainer *c, size_t *len) {
    unsigned char *buf = NULL;
    ISO_CHECK_EQ_INT(ppm_encode(c, &buf, len), PPM_OK);
    ISO_CHECK(buf != NULL);
    return buf;
}

/* Rust: header_starts_with_p6. */
static void test_header_starts_with_p6(void) {
    PixelContainer *buf = pixel_new(2, 2);
    size_t len = 0;
    unsigned char *ppm = encode_ok(buf, &len);
    ISO_CHECK(len >= 3);
    ISO_CHECK_MEM_EQ(ppm, "P6\n", 3);
    ppm_free(ppm);
    pixel_free(buf);
}

/* Rust: header_contains_dimensions. */
static void test_header_contains_dimensions(void) {
    PixelContainer *buf = pixel_new(640, 480);
    size_t len = 0;
    unsigned char *ppm = encode_ok(buf, &len);
    ISO_CHECK(len >= 15);
    ISO_CHECK_MEM_EQ(ppm, "P6\n640 480\n255\n", 15); /* the full header */
    ppm_free(ppm);
    pixel_free(buf);
}

/* Rust: encoded_size_correct. */
static void test_encoded_size_correct(void) {
    PixelContainer *buf = pixel_new(3, 2);
    size_t len = 0;
    unsigned char *ppm = encode_ok(buf, &len);
    /* header "P6\n3 2\n255\n" = 11 bytes, pixel data = 3*2*3 = 18. */
    ISO_CHECK_EQ_UINT(len, 11 + 18);
    ppm_free(ppm);
    pixel_free(buf);
}

/* Rust: alpha_channel_not_in_file. */
static void test_alpha_dropped(void) {
    PixelContainer *buf = pixel_new(1, 1);
    size_t len = 0;
    unsigned char *ppm;
    pixel_set(buf, 0, 0, 10, 20, 30, 128); /* semi-transparent */
    ppm = encode_ok(buf, &len);
    /* header "P6\n1 1\n255\n" = 11 bytes, then exactly 3 RGB bytes (no alpha). */
    ISO_CHECK_EQ_UINT(len, 11 + 3);
    ISO_CHECK_EQ_INT(ppm[11], 10);
    ISO_CHECK_EQ_INT(ppm[12], 20);
    ISO_CHECK_EQ_INT(ppm[13], 30);
    ppm_free(ppm);
    pixel_free(buf);
}

/* Rust: round_trip_solid_colour. */
static void test_round_trip_solid(void) {
    PixelContainer *orig = pixel_new(5, 3);
    PixelContainer *decoded = NULL;
    unsigned char *ppm;
    size_t len = 0;
    uint32_t x, y;
    pixel_fill(orig, 100, 150, 200, 255);
    ppm = encode_ok(orig, &len);
    ISO_CHECK_EQ_INT(ppm_decode(ppm, len, &decoded), PPM_OK);
    ISO_CHECK_EQ_UINT(pixel_width(decoded), 5);
    ISO_CHECK_EQ_UINT(pixel_height(decoded), 3);
    for (y = 0; y < 3; y++) {
        for (x = 0; x < 5; x++) {
            uint8_t rgba[4];
            pixel_at(decoded, x, y, rgba);
            ISO_CHECK_EQ_INT(rgba[0], 100);
            ISO_CHECK_EQ_INT(rgba[1], 150);
            ISO_CHECK_EQ_INT(rgba[2], 200);
            ISO_CHECK_EQ_INT(rgba[3], 255); /* alpha restored to opaque */
        }
    }
    ppm_free(ppm);
    pixel_free(orig);
    pixel_free(decoded);
}

/* Rust: round_trip_rgb_preserved. */
static void test_round_trip_rgb(void) {
    PixelContainer *orig = pixel_new(2, 2);
    PixelContainer *decoded = NULL;
    unsigned char *ppm;
    size_t len = 0;
    uint32_t x, y;
    pixel_set(orig, 0, 0, 255, 0, 0, 255);
    pixel_set(orig, 1, 0, 0, 255, 0, 255);
    pixel_set(orig, 0, 1, 0, 0, 255, 255);
    pixel_set(orig, 1, 1, 128, 128, 128, 255);
    ppm = encode_ok(orig, &len);
    ISO_CHECK_EQ_INT(ppm_decode(ppm, len, &decoded), PPM_OK);
    for (y = 0; y < 2; y++) {
        for (x = 0; x < 2; x++) {
            uint8_t a[4], b[4];
            pixel_at(orig, x, y, a);
            pixel_at(decoded, x, y, b);
            ISO_CHECK_EQ_INT(a[0], b[0]);
            ISO_CHECK_EQ_INT(a[1], b[1]);
            ISO_CHECK_EQ_INT(a[2], b[2]);
            ISO_CHECK_EQ_INT(b[3], 255);
        }
    }
    ppm_free(ppm);
    pixel_free(orig);
    pixel_free(decoded);
}

/* Rust: decode_with_comment_in_header. */
static void test_decode_with_comment(void) {
    /* "P6\n# this is a comment\n4 4\n255\n" + 4*4*3 zero bytes. */
    static const char head[] = "P6\n# this is a comment\n4 4\n255\n";
    size_t head_len = sizeof(head) - 1;
    size_t total = head_len + 4 * 4 * 3;
    unsigned char *data = (unsigned char *)calloc(total, 1);
    PixelContainer *decoded = NULL;
    ISO_CHECK(data != NULL);
    memcpy(data, head, head_len);
    ISO_CHECK_EQ_INT(ppm_decode(data, total, &decoded), PPM_OK);
    ISO_CHECK_EQ_UINT(pixel_width(decoded), 4);
    ISO_CHECK_EQ_UINT(pixel_height(decoded), 4);
    free(data);
    pixel_free(decoded);
}

/* Rust: decode_wrong_magic_returns_error. */
static void test_decode_wrong_magic(void) {
    static const char d[] = "P3\n1 1\n255\n\x00\x00\x00";
    PixelContainer *decoded = NULL;
    ISO_CHECK_EQ_INT(ppm_decode((const unsigned char *)d, sizeof(d) - 1, &decoded),
                     PPM_ERR_MAGIC);
    ISO_CHECK(decoded == NULL);
}

/* Rust: decode_unsupported_maxval_returns_error. */
static void test_decode_bad_maxval(void) {
    static const char d[] = "P6\n1 1\n65535\n\x00\x00\x00\x00\x00\x00";
    PixelContainer *decoded = NULL;
    ISO_CHECK_EQ_INT(ppm_decode((const unsigned char *)d, sizeof(d) - 1, &decoded),
                     PPM_ERR_MAXVAL);
    ISO_CHECK(decoded == NULL);
}

/* Rust: decode_truncated_pixel_data_returns_error. */
static void test_decode_truncated(void) {
    /* Header says 4×4 = 16 pixels = 48 bytes, but only 3 follow. */
    static const char d[] = "P6\n4 4\n255\n\x00\x00\x00";
    PixelContainer *decoded = NULL;
    ISO_CHECK_EQ_INT(ppm_decode((const unsigned char *)d, sizeof(d) - 1, &decoded),
                     PPM_ERR_TRUNCATED);
    ISO_CHECK(decoded == NULL);
}

/* Rust: codec_mime_type. */
static void test_mime_type(void) {
    ISO_CHECK_STR_EQ(ppm_mime_type(), "image/x-portable-pixmap");
}

/* Additional: a comment/whitespace-heavy header still parses; a missing maxval
 * separator (data starts immediately) is truncated. */
static void test_decode_extra_whitespace(void) {
    static const char head[] = "P6  \n\n2 1\n255\n";
    size_t head_len = sizeof(head) - 1;
    size_t total = head_len + 2 * 1 * 3;
    unsigned char *data = (unsigned char *)calloc(total, 1);
    PixelContainer *decoded = NULL;
    ISO_CHECK(data != NULL);
    memcpy(data, head, head_len);
    ISO_CHECK_EQ_INT(ppm_decode(data, total, &decoded), PPM_OK);
    ISO_CHECK_EQ_UINT(pixel_width(decoded), 2);
    free(data);
    pixel_free(decoded);
}

static void test_invalid_params(void) {
    PixelContainer *buf = pixel_new(1, 1);
    unsigned char *out = NULL;
    size_t len = 0;
    PixelContainer *dec = NULL;
    ISO_CHECK_EQ_INT(ppm_encode(NULL, &out, &len), PPM_ERR_INVALID);
    ISO_CHECK_EQ_INT(ppm_encode(buf, NULL, &len), PPM_ERR_INVALID);
    ISO_CHECK_EQ_INT(ppm_encode(buf, &out, NULL), PPM_ERR_INVALID);
    ISO_CHECK_EQ_INT(ppm_decode(NULL, 0, &dec), PPM_ERR_INVALID);
    ISO_CHECK_EQ_INT(ppm_decode((const unsigned char *)"P6", 2, NULL), PPM_ERR_INVALID);
    /* Empty input → no magic token → magic error. */
    ISO_CHECK_EQ_INT(ppm_decode((const unsigned char *)"", 0, &dec), PPM_ERR_MAGIC);
    ppm_free(NULL);
    pixel_free(buf);
}

int main(void) {
    test_header_starts_with_p6();
    test_header_contains_dimensions();
    test_encoded_size_correct();
    test_alpha_dropped();
    test_round_trip_solid();
    test_round_trip_rgb();
    test_decode_with_comment();
    test_decode_wrong_magic();
    test_decode_bad_maxval();
    test_decode_truncated();
    test_mime_type();
    test_decode_extra_whitespace();
    test_invalid_params();
    return ISO_TEST_RESULT();
}
