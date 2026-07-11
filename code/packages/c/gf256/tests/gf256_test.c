/* Tests for the C gf256, using the iso_test.h harness. Pinned to known GF(2^8)
 * values (Reed-Solomon 0x11D and AES 0x11B fields) plus algebraic round-trips. */
#include "iso_test.h"

#include "gf256.h"

int main(void) {
    /* Addition / subtraction are XOR. */
    ISO_CHECK_EQ_UINT(gf256_add(0x53, 0xCA), 0x99);
    ISO_CHECK_EQ_UINT(gf256_subtract(0x53, 0xCA), 0x99);
    ISO_CHECK_EQ_UINT(gf256_add(0xAB, 0xAB), 0x00); /* every element self-inverse */

    /* Multiplication in the default (0x11D) field. */
    ISO_CHECK_EQ_UINT(gf256_multiply(3, 7), 9);      /* (x+1)(x^2+x+1) = x^3+1 */
    ISO_CHECK_EQ_UINT(gf256_multiply(2, 0x80), 0x1D); /* 2 * x^7, one reduction */
    ISO_CHECK_EQ_UINT(gf256_multiply(0, 0x42), 0);
    ISO_CHECK_EQ_UINT(gf256_multiply(1, 0x42), 0x42); /* identity */

    /* Power and its special cases. */
    ISO_CHECK_EQ_UINT(gf256_power(2, 8), 0x1D);
    ISO_CHECK_EQ_UINT(gf256_power(0x42, 0), 1);
    ISO_CHECK_EQ_UINT(gf256_power(0, 0), 1);
    ISO_CHECK_EQ_UINT(gf256_power(0, 5), 0);

    /* Every non-zero element has an inverse: a * a^-1 == 1. */
    {
        int a;
        int ok = 1;
        for (a = 1; a < 256; a++) {
            uint8_t inv = gf256_inverse((uint8_t)a);
            if (gf256_multiply((uint8_t)a, inv) != 1) {
                ok = 0;
            }
        }
        ISO_CHECK_MSG(ok, "a * inverse(a) must be 1 for all non-zero a");
    }

    /* Division inverts multiplication. */
    ISO_CHECK_EQ_UINT(gf256_divide(9, 3), 7);
    {
        int a, b;
        int ok = 1;
        for (b = 1; b < 256; b += 17) {
            for (a = 0; a < 256; a += 13) {
                uint8_t prod = gf256_multiply((uint8_t)a, (uint8_t)b);
                if (gf256_divide(prod, (uint8_t)b) != (uint8_t)a) {
                    ok = 0;
                }
            }
        }
        ISO_CHECK_MSG(ok, "divide(multiply(a,b), b) == a");
    }
    ISO_CHECK_EQ_UINT(gf256_divide(0x42, 0), 0); /* by-zero → 0 (crate panics) */
    ISO_CHECK_EQ_UINT(gf256_inverse(0), 0);

    /* The parameterisable field on 0x11D must match the module functions. */
    {
        gf256_field rs = gf256_field_new(GF256_PRIMITIVE_POLYNOMIAL);
        int a, b;
        int ok = 1;
        for (a = 0; a < 256; a += 11) {
            for (b = 0; b < 256; b += 11) {
                if (gf256_field_multiply(&rs, (uint8_t)a, (uint8_t)b) !=
                    gf256_multiply((uint8_t)a, (uint8_t)b)) {
                    ok = 0;
                }
            }
        }
        ISO_CHECK_MSG(ok, "Field(0x11D) must match the module-level multiply");
    }

    /* The AES field (0x11B): classic FIPS-197 values. */
    {
        gf256_field aes = gf256_field_new(0x11B);
        ISO_CHECK_EQ_UINT(gf256_field_multiply(&aes, 0x57, 0x83), 0xC1);
        ISO_CHECK_EQ_UINT(gf256_field_multiply(&aes, 0x53, 0xCA), 0x01);
        ISO_CHECK_EQ_UINT(gf256_field_inverse(&aes, 0x53), 0xCA);
        ISO_CHECK_EQ_UINT(gf256_field_add(&aes, 0x57, 0x83), 0xD4);
        /* Inverse round-trip across the AES field. */
        {
            int a;
            int ok = 1;
            for (a = 1; a < 256; a++) {
                uint8_t inv = gf256_field_inverse(&aes, (uint8_t)a);
                if (gf256_field_multiply(&aes, (uint8_t)a, inv) != 1) {
                    ok = 0;
                }
            }
            ISO_CHECK_MSG(ok, "AES-field inverse round-trip");
        }
        ISO_CHECK_EQ_UINT(gf256_field_divide(&aes, 0x01, 0x53), 0xCA);
        ISO_CHECK_EQ_UINT(gf256_field_power(&aes, 0x03, 0), 1);
    }

    return ISO_TEST_RESULT();
}
