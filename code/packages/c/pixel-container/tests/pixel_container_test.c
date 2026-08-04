/* Tests for the C pixel-container, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include "pixel_container.h"

int main(void) {
    /* ── new: correct size, all zeros ───────────────────────────────────── */
    {
        PixelContainer *p = pixel_new(10, 20);
        size_t i;
        const uint8_t *d;
        int all_zero = 1;
        ISO_CHECK_EQ_UINT(pixel_width(p), 10u);
        ISO_CHECK_EQ_UINT(pixel_height(p), 20u);
        ISO_CHECK_EQ_UINT(pixel_byte_count(p), 10u * 20u * 4u);
        ISO_CHECK_EQ_UINT(pixel_count(p), 200u);
        d = pixel_data(p);
        for (i = 0; i < pixel_byte_count(p); i++) {
            if (d[i] != 0) {
                all_zero = 0;
            }
        }
        ISO_CHECK(all_zero);
        pixel_free(p);
    }

    /* ── zero dimensions are valid (no pixels) ──────────────────────────── */
    {
        PixelContainer *p = pixel_new(0, 0);
        ISO_CHECK_EQ_UINT(pixel_byte_count(p), 0u);
        ISO_CHECK_EQ_UINT(pixel_count(p), 0u);
        pixel_free(p);
    }

    /* ── from_data round trip; length mismatch returns NULL ─────────────── */
    {
        uint8_t data[4] = {255, 128, 64, 32};
        PixelContainer *p = pixel_from_data(1, 1, data, 4);
        uint8_t px[4];
        ISO_CHECK(p != NULL);
        pixel_at(p, 0, 0, px);
        ISO_CHECK(px[0] == 255 && px[1] == 128 && px[2] == 64 && px[3] == 32);
        pixel_free(p);
        /* wrong length -> NULL (Rust panics). */
        {
            uint8_t bad[3] = {1, 2, 3};
            ISO_CHECK(pixel_from_data(1, 1, bad, 3) == NULL);
        }
    }

    /* ── set / get pixel, offset correctness ────────────────────────────── */
    {
        PixelContainer *p = pixel_new(5, 5);
        uint8_t px[4];
        const uint8_t *d;
        pixel_set(p, 3, 2, 1, 2, 3, 4); /* offset = (2*5+3)*4 = 52 */
        pixel_at(p, 3, 2, px);
        ISO_CHECK(px[0] == 1 && px[1] == 2 && px[2] == 3 && px[3] == 4);
        d = pixel_data(p);
        ISO_CHECK(d[52] == 1 && d[53] == 2 && d[54] == 3 && d[55] == 4);
        pixel_free(p);
    }

    /* ── out-of-bounds: pixel_at -> zeros, pixel_set -> no-op ────────────── */
    {
        PixelContainer *p = pixel_new(4, 4);
        uint8_t px[4];
        const uint8_t *d;
        size_t i;
        int untouched = 1;
        pixel_at(p, 4, 0, px);
        ISO_CHECK(px[0] == 0 && px[1] == 0 && px[2] == 0 && px[3] == 0);
        pixel_at(p, 100, 100, px);
        ISO_CHECK(px[0] == 0 && px[3] == 0);
        pixel_set(p, 10, 10, 255, 255, 255, 255); /* no-op */
        d = pixel_data(p);
        for (i = 0; i < pixel_byte_count(p); i++) {
            if (d[i] != 0) {
                untouched = 0;
            }
        }
        ISO_CHECK(untouched);
        pixel_free(p);
    }

    /* ── fill sets every pixel ──────────────────────────────────────────── */
    {
        PixelContainer *p = pixel_new(3, 3);
        uint32_t x, y;
        int ok = 1;
        pixel_fill(p, 255, 128, 0, 255);
        for (y = 0; y < 3; y++) {
            for (x = 0; x < 3; x++) {
                uint8_t px[4];
                pixel_at(p, x, y, px);
                if (px[0] != 255 || px[1] != 128 || px[2] != 0 || px[3] != 255) {
                    ok = 0;
                }
            }
        }
        ISO_CHECK(ok);
        pixel_free(p);
    }

    /* ── clone is independent; equality compares fields ─────────────────── */
    {
        PixelContainer *orig = pixel_new(2, 2);
        PixelContainer *cl;
        uint8_t px[4];
        pixel_set(orig, 0, 0, 1, 2, 3, 4);
        cl = pixel_clone(orig);
        ISO_CHECK(pixel_equals(orig, cl));
        pixel_set(cl, 0, 0, 99, 99, 99, 99);
        pixel_at(orig, 0, 0, px); /* original unchanged */
        ISO_CHECK(px[0] == 1 && px[1] == 2 && px[2] == 3 && px[3] == 4);
        ISO_CHECK(!pixel_equals(orig, cl));
        pixel_free(orig);
        pixel_free(cl);
    }

    /* ── equality across constructions ──────────────────────────────────── */
    {
        uint8_t d1[4] = {1, 2, 3, 4};
        uint8_t d2[4] = {1, 2, 3, 5};
        PixelContainer *a = pixel_from_data(1, 1, d1, 4);
        PixelContainer *b = pixel_from_data(1, 1, d1, 4);
        PixelContainer *c = pixel_from_data(1, 1, d2, 4);
        ISO_CHECK(pixel_equals(a, b));
        ISO_CHECK(!pixel_equals(a, c));
        pixel_free(a);
        pixel_free(b);
        pixel_free(c);
    }

    /* ── dimension overflow returns NULL (Rust panics) ──────────────────── */
    {
        /* 65536 * 65536 * 4 = 2^34, fine on 64-bit; use values that overflow
         * the pixel*4 product only where size_t is 32-bit. On 64-bit this
         * succeeds, so just check a huge allocation is rejected cleanly. */
        PixelContainer *p = pixel_new(0xFFFFFFFFu, 0xFFFFFFFFu);
        ISO_CHECK(p == NULL); /* width*height*4 overflows size_t */
    }

    return ISO_TEST_RESULT();
}
