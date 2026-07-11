/* Tests for the C Reed-Solomon codec, using the iso_test.h harness. Verifies the
 * generator polynomial and end-to-end encode -> corrupt -> decode recovery. */
#include "iso_test.h"

#include <string.h>

#include "reed_solomon.h"

int main(void) {
    /* Generator for n_check = 2 is 8 + 6x + x^2 (little-endian [8, 6, 1]). */
    {
        uint8_t g[8];
        size_t glen = 0;
        ISO_CHECK_EQ_INT(rs_build_generator(2, g, &glen), RS_OK);
        ISO_CHECK_EQ_UINT(glen, 3);
        ISO_CHECK_EQ_UINT(g[0], 8);
        ISO_CHECK_EQ_UINT(g[1], 6);
        ISO_CHECK_EQ_UINT(g[2], 1);
    }
    /* Odd n_check is rejected. */
    {
        uint8_t g[8];
        size_t glen = 0;
        ISO_CHECK_EQ_INT(rs_build_generator(3, g, &glen), RS_INVALID_INPUT);
    }

    /* Encode / decode round-trip with no errors. */
    {
        const uint8_t msg[5] = {'H', 'E', 'L', 'L', 'O'};
        uint8_t code[16], dec[16];
        size_t clen = 0, dlen = 0;
        ISO_CHECK_EQ_INT(rs_encode(msg, 5, 4, code, &clen), RS_OK);
        ISO_CHECK_EQ_UINT(clen, 9); /* 5 message + 4 check */
        ISO_CHECK_MEM_EQ(code, msg, 5); /* systematic: message appears first */
        ISO_CHECK_EQ_INT(rs_decode(code, clen, 4, dec, &dlen), RS_OK);
        ISO_CHECK_EQ_UINT(dlen, 5);
        ISO_CHECK_MEM_EQ(dec, msg, 5);
    }

    /* Correct a single-byte error (t = 2 with n_check = 4). */
    {
        const uint8_t msg[5] = {'H', 'E', 'L', 'L', 'O'};
        uint8_t code[16], dec[16];
        size_t clen = 0, dlen = 0;
        rs_encode(msg, 5, 4, code, &clen);
        code[2] ^= 0x5A; /* flip a message byte */
        ISO_CHECK_EQ_INT(rs_decode(code, clen, 4, dec, &dlen), RS_OK);
        ISO_CHECK_MEM_EQ(dec, msg, 5);
    }

    /* Correct two-byte errors (the maximum, t = 2). */
    {
        const uint8_t msg[8] = {1, 2, 3, 4, 5, 6, 7, 8};
        uint8_t code[16], dec[16];
        size_t clen = 0, dlen = 0;
        rs_encode(msg, 8, 4, code, &clen);
        ISO_CHECK_EQ_UINT(clen, 12);
        code[1] ^= 0xFF;  /* a message byte */
        code[10] ^= 0x33; /* a check byte */
        ISO_CHECK_EQ_INT(rs_decode(code, clen, 4, dec, &dlen), RS_OK);
        ISO_CHECK_MEM_EQ(dec, msg, 8);
    }

    /* Correct up to t = 4 errors with n_check = 8. */
    {
        uint8_t msg[16], code[32], dec[32];
        size_t clen = 0, dlen = 0;
        int i;
        for (i = 0; i < 16; i++) {
            msg[i] = (uint8_t)(i * 17 + 3);
        }
        rs_encode(msg, 16, 8, code, &clen);
        ISO_CHECK_EQ_UINT(clen, 24);
        code[0] ^= 0x11;
        code[7] ^= 0x22;
        code[15] ^= 0x44;
        code[23] ^= 0x88;
        ISO_CHECK_EQ_INT(rs_decode(code, clen, 8, dec, &dlen), RS_OK);
        ISO_CHECK_MEM_EQ(dec, msg, 16);
    }

    /* Too many errors (3 > t = 2) is reported, not silently mis-decoded. */
    {
        const uint8_t msg[8] = {1, 2, 3, 4, 5, 6, 7, 8};
        uint8_t code[16], dec[16];
        size_t clen = 0, dlen = 0;
        rs_status st;
        rs_encode(msg, 8, 4, code, &clen);
        code[0] ^= 0x01;
        code[3] ^= 0x02;
        code[6] ^= 0x04;
        st = rs_decode(code, clen, 4, dec, &dlen);
        ISO_CHECK(st == RS_TOO_MANY_ERRORS);
    }

    /* Decode rejects a too-short codeword. */
    {
        uint8_t code[2] = {0, 0}, dec[2];
        size_t dlen = 0;
        ISO_CHECK_EQ_INT(rs_decode(code, 2, 4, dec, &dlen), RS_INVALID_INPUT);
    }

    /* Oversized parameters are rejected (these guard fixed-buffer overflow). */
    {
        uint8_t buf[300];
        size_t len = 0;
        /* n_check beyond the 254 codeword limit. */
        ISO_CHECK_EQ_INT(rs_build_generator(1000, buf, &len), RS_INVALID_INPUT);
        ISO_CHECK_EQ_INT(rs_build_generator(256, buf, &len), RS_INVALID_INPUT);
        /* received_len beyond the 255-byte GF(256) block. */
        {
            uint8_t big[256] = {0};
            ISO_CHECK_EQ_INT(rs_decode(big, 256, 4, buf, &len), RS_INVALID_INPUT);
        }
        /* A syndrome sequence longer than any real codeword. */
        {
            uint8_t syn[300] = {0};
            ISO_CHECK_EQ_UINT(rs_error_locator(syn, 300, buf), 0);
        }
    }

    return ISO_TEST_RESULT();
}
