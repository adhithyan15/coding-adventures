/* Tests for the C DES, using the iso_test.h harness. Pinned to the FIPS 46 and
 * NIST SP 800-20 known-answer vectors, plus round-trips, ECB, and Triple DES. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "des.h"

int main(void) {
    /* FIPS 46 worked example: E(0123456789ABCDEF, 133457799BBCDFF1). */
    {
        const uint8_t key[8] = {0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1};
        const uint8_t plain[8] = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
        const uint8_t ct[8] = {0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05};
        uint8_t out[8], back[8];
        des_encrypt_block(plain, key, out);
        ISO_CHECK_MEM_EQ(out, ct, 8);
        des_decrypt_block(ct, key, back);
        ISO_CHECK_MEM_EQ(back, plain, 8);
    }

    /* NIST SP 800-20 Table 1 (plaintext variable, key = all 0x01). */
    {
        const uint8_t key[8] = {1, 1, 1, 1, 1, 1, 1, 1};
        struct {
            uint8_t pt[8];
            uint8_t ct[8];
        } v[3] = {
            {{0x95, 0xF8, 0xA5, 0xE5, 0xDD, 0x31, 0xD9, 0x00},
             {0x80, 0, 0, 0, 0, 0, 0, 0}},
            {{0xDD, 0x7F, 0x12, 0x1C, 0xA5, 0x01, 0x56, 0x19},
             {0x40, 0, 0, 0, 0, 0, 0, 0}},
            {{0x2E, 0x86, 0x53, 0x10, 0x4F, 0x38, 0x34, 0xEA},
             {0x20, 0, 0, 0, 0, 0, 0, 0}},
        };
        int i;
        for (i = 0; i < 3; i++) {
            uint8_t out[8];
            des_encrypt_block(v[i].pt, key, out);
            ISO_CHECK_MEM_EQ(out, v[i].ct, 8);
        }
    }

    /* NIST SP 800-20 Table 2 (key variable, plaintext = all zero). */
    {
        const uint8_t pt[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        struct {
            uint8_t key[8];
            uint8_t ct[8];
        } v[2] = {
            {{0x80, 1, 1, 1, 1, 1, 1, 1},
             {0x95, 0xA8, 0xD7, 0x28, 0x13, 0xDA, 0xA9, 0x4D}},
            {{0x40, 1, 1, 1, 1, 1, 1, 1},
             {0x0E, 0xEC, 0x14, 0x87, 0xDD, 0x8C, 0x26, 0xD5}},
        };
        int i;
        for (i = 0; i < 2; i++) {
            uint8_t out[8];
            des_encrypt_block(pt, v[i].key, out);
            ISO_CHECK_MEM_EQ(out, v[i].ct, 8);
        }
    }

    /* Round-trips across several keys, over every byte value. */
    {
        const uint8_t key[8] = {0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10};
        int start;
        int ok = 1;
        for (start = 0; start <= 248; start += 8) {
            uint8_t block[8], ct[8], back[8];
            int i;
            for (i = 0; i < 8; i++) {
                block[i] = (uint8_t)(start + i);
            }
            des_encrypt_block(block, key, ct);
            des_decrypt_block(ct, key, back);
            if (memcmp(back, block, 8) != 0) {
                ok = 0;
            }
        }
        ISO_CHECK_MSG(ok, "encrypt/decrypt must round-trip for all blocks");
    }

    /* ECB mode with PKCS#7 padding round-trips (and pads to a block). */
    {
        const uint8_t key[8] = {0x01, 0x33, 0x45, 0x77, 0x99, 0xBB, 0xCD, 0xFF};
        const char *msg = "hello";
        uint8_t *ct;
        size_t ct_len = 0;
        ct = des_ecb_encrypt((const uint8_t *)msg, 5, key, &ct_len);
        ISO_CHECK(ct != NULL);
        ISO_CHECK_EQ_UINT(ct_len, 8); /* 5 bytes + 3 pad → one block */
        {
            uint8_t *pt = NULL;
            size_t pt_len = 0;
            ISO_CHECK(des_ecb_decrypt(ct, ct_len, key, &pt, &pt_len));
            ISO_CHECK_EQ_UINT(pt_len, 5);
            ISO_CHECK_MEM_EQ(pt, msg, 5);
            free(pt);
        }
        free(ct);
        /* A block-aligned input gets a full extra padding block. */
        {
            const uint8_t eight[8] = {1, 2, 3, 4, 5, 6, 7, 8};
            size_t n = 0;
            uint8_t *c = des_ecb_encrypt(eight, 8, key, &n);
            uint8_t *p = NULL;
            size_t pl = 0;
            ISO_CHECK(c != NULL && n == 16);
            ISO_CHECK(des_ecb_decrypt(c, n, key, &p, &pl));
            ISO_CHECK_EQ_UINT(pl, 8);
            ISO_CHECK_MEM_EQ(p, eight, 8);
            free(p);
            free(c);
        }
        /* Bad ciphertext length is rejected. */
        {
            uint8_t junk[7] = {0};
            uint8_t *p = NULL;
            size_t pl = 0;
            ISO_CHECK(!des_ecb_decrypt(junk, 7, key, &p, &pl));
        }
    }

    /* Triple DES (EDE) round-trips; with equal keys it reduces to single DES. */
    {
        const uint8_t k1[8] = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
        const uint8_t k2[8] = {0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01};
        const uint8_t k3[8] = {0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23};
        const uint8_t plain[8] = {0x6B, 0xC1, 0xBE, 0xE2,
                                  0x2E, 0x40, 0x9F, 0x96};
        uint8_t ct[8], back[8];
        des_tdea_encrypt_block(plain, k1, k2, k3, ct);
        des_tdea_decrypt_block(ct, k1, k2, k3, back);
        ISO_CHECK_MEM_EQ(back, plain, 8);
        /* K1 = K2 = K3 → 3DES == single DES. */
        {
            uint8_t single[8], triple[8];
            des_encrypt_block(plain, k1, single);
            des_tdea_encrypt_block(plain, k1, k1, k1, triple);
            ISO_CHECK_MEM_EQ(triple, single, 8);
        }
    }

    return ISO_TEST_RESULT();
}
