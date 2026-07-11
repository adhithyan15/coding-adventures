/* Tests for the C ct-compare, using the iso_test.h harness. These verify
 * functional correctness (the constant-time property itself is not unit-testable
 * here); the every-bit-flip sweeps are what a short-circuit bug would fail. */
#include "iso_test.h"

#include <string.h>

#include "ct_compare.h"

int main(void) {
    /* ct_eq: equal inputs. */
    {
        uint8_t z32[32] = {0};
        uint8_t f64[64];
        memset(f64, 0xFF, sizeof f64);
        ISO_CHECK(ct_eq((const uint8_t *)"abcdef", 6,
                        (const uint8_t *)"abcdef", 6));
        ISO_CHECK(ct_eq(z32, 32, z32, 32));
        ISO_CHECK(ct_eq(f64, 64, f64, 64));
    }

    /* ct_eq: every single-bit flip at every byte position is detected. */
    {
        uint8_t base[32], flipped[32];
        int i, bit;
        int all_detected = 1;
        memset(base, 0x42, sizeof base);
        for (i = 0; i < 32; i++) {
            for (bit = 0; bit < 8; bit++) {
                memcpy(flipped, base, sizeof base);
                flipped[i] ^= (uint8_t)(1 << bit);
                if (ct_eq(base, 32, flipped, 32)) {
                    all_detected = 0; /* a flip went unnoticed */
                }
            }
        }
        ISO_CHECK_MSG(all_detected, "every bit flip must be detected");
    }

    /* ct_eq: length mismatch and empties. */
    {
        ISO_CHECK(!ct_eq((const uint8_t *)"abc", 3, (const uint8_t *)"abcd", 4));
        ISO_CHECK(!ct_eq((const uint8_t *)"abcd", 4, (const uint8_t *)"abc", 3));
        ISO_CHECK(ct_eq((const uint8_t *)"", 0, (const uint8_t *)"", 0));
        /* First-byte and last-byte differences. */
        ISO_CHECK(!ct_eq((const uint8_t *)"abcdef", 6,
                         (const uint8_t *)"bbcdef", 6));
        ISO_CHECK(!ct_eq((const uint8_t *)"abcdef", 6,
                         (const uint8_t *)"abcdeg", 6));
    }

    /* ct_eq_fixed matches ct_eq for same-length inputs. */
    {
        uint8_t a[16], b[16];
        memset(a, 0x11, sizeof a);
        memset(b, 0x11, sizeof b);
        ISO_CHECK(ct_eq_fixed(a, b, 16));
        b[7] ^= 0x80;
        ISO_CHECK(!ct_eq_fixed(a, b, 16));
    }

    /* ct_select_bytes: choice picks the right buffer, byte for byte. */
    {
        uint8_t a[8], b[8], out[8];
        int i;
        for (i = 0; i < 8; i++) {
            a[i] = (uint8_t)(0xA0 + i);
            b[i] = (uint8_t)(0xB0 + i);
        }
        ct_select_bytes(a, b, 1, 8, out);
        ISO_CHECK_MEM_EQ(out, a, 8);
        ct_select_bytes(a, b, 0, 8, out);
        ISO_CHECK_MEM_EQ(out, b, 8);
        /* Any non-zero choice selects a. */
        ct_select_bytes(a, b, 42, 8, out);
        ISO_CHECK_MEM_EQ(out, a, 8);
    }

    /* ct_eq_u64: equal, every single-bit difference, and the high bit. */
    {
        int bit;
        int all_detected = 1;
        ISO_CHECK(ct_eq_u64(0, 0));
        ISO_CHECK(ct_eq_u64(0xFFFFFFFFFFFFFFFFull, 0xFFFFFFFFFFFFFFFFull));
        ISO_CHECK(ct_eq_u64(0xDEADBEEFull, 0xDEADBEEFull));
        for (bit = 0; bit < 64; bit++) {
            uint64_t base = 0x0123456789ABCDEFull;
            uint64_t flipped = base ^ ((uint64_t)1 << bit);
            if (ct_eq_u64(base, flipped)) {
                all_detected = 0;
            }
        }
        ISO_CHECK_MSG(all_detected, "every u64 bit difference must be detected");
        ISO_CHECK(!ct_eq_u64(0, (uint64_t)1 << 63));
    }

    return ISO_TEST_RESULT();
}
