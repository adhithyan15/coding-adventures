/* Tests for the C aes-modes, using the header-only iso_test.h harness (pure
 * ISO). Vectors are NIST SP 800-38A (ECB/CBC) and the classic GCM test cases,
 * matching the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* memcmp, memset */

#include "aes_modes.h"

/* Decode a lowercase hex string into `out`; returns the byte count. */
static size_t from_hex(const char *hex, uint8_t *out) {
    size_t i;
    for (i = 0; hex[i] && hex[i + 1]; i += 2) {
        int hi = hex[i] <= '9' ? hex[i] - '0' : (hex[i] | 0x20) - 'a' + 10;
        int lo = hex[i + 1] <= '9' ? hex[i + 1] - '0'
                                   : (hex[i + 1] | 0x20) - 'a' + 10;
        out[i / 2] = (uint8_t)((hi << 4) | lo);
    }
    return i / 2;
}

/* NIST SP 800-38A test key and plaintext. */
static const char *KEY = "2b7e151628aed2a6abf7158809cf4f3c";
static const char *PT_ALL =
    "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51"
    "30c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710";

int main(void) {
    uint8_t key[32];
    uint8_t pt[64];
    size_t key_len = from_hex(KEY, key);
    size_t pt_len = from_hex(PT_ALL, pt);

    /* ── PKCS#7 ─────────────────────────────────────────────────────────── */
    {
        uint8_t *out;
        size_t out_len;
        uint8_t aligned[16];
        uint8_t thirteen[13];
        size_t i;
        for (i = 0; i < 16; i++) {
            aligned[i] = 0xAA;
        }
        ISO_CHECK_EQ_INT((int)aesm_pkcs7_pad(aligned, 16, &out, &out_len),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(out_len, 32u); /* full block appended */
        for (i = 16; i < 32; i++) {
            ISO_CHECK(out[i] == 16);
        }
        free(out);

        for (i = 0; i < 13; i++) {
            thirteen[i] = 0xBB;
        }
        ISO_CHECK_EQ_INT((int)aesm_pkcs7_pad(thirteen, 13, &out, &out_len),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(out_len, 16u);
        ISO_CHECK(out[13] == 3 && out[14] == 3 && out[15] == 3);
        free(out);

        /* round trip and rejection of bad padding. */
        {
            uint8_t five[5] = {1, 2, 3, 4, 5};
            uint8_t *padded;
            size_t plen;
            uint8_t *back;
            size_t blen;
            aesm_pkcs7_pad(five, 5, &padded, &plen);
            ISO_CHECK_EQ_INT((int)aesm_pkcs7_unpad(padded, plen, &back, &blen),
                             (int)AESM_OK);
            ISO_CHECK_EQ_UINT(blen, 5u);
            ISO_CHECK_MEM_EQ(back, five, 5);
            free(padded);
            free(back);
        }
        {
            uint8_t bad[16];
            uint8_t *o;
            size_t ol;
            memset(bad, 0, 16);
            bad[15] = 0; /* pad_len 0 is invalid */
            ISO_CHECK_EQ_INT((int)aesm_pkcs7_unpad(bad, 16, &o, &ol),
                             (int)AESM_BAD_PADDING);
            bad[15] = 2;
            bad[14] = 3; /* inconsistent */
            ISO_CHECK_EQ_INT((int)aesm_pkcs7_unpad(bad, 16, &o, &ol),
                             (int)AESM_BAD_PADDING);
        }
    }

    /* ── ECB (NIST SP 800-38A) ──────────────────────────────────────────── */
    {
        uint8_t *ct;
        size_t ct_len;
        uint8_t want[16];
        uint8_t *back;
        size_t blen;
        ISO_CHECK_EQ_INT(
            (int)aesm_ecb_encrypt(pt, pt_len, key, key_len, &ct, &ct_len),
            (int)AESM_OK);
        ISO_CHECK_EQ_UINT(ct_len, 80u); /* 64 + full pad block */
        from_hex("3ad77bb40d7a3660a89ecaf32466ef97", want);
        ISO_CHECK_MEM_EQ(ct, want, 16);
        from_hex("f5d3d58503b9699de785895a96fdbaaf", want);
        ISO_CHECK_MEM_EQ(ct + 16, want, 16);
        from_hex("43b1cd7f598ece23881b00e3ed030688", want);
        ISO_CHECK_MEM_EQ(ct + 32, want, 16);
        /* round trip. */
        ISO_CHECK_EQ_INT(
            (int)aesm_ecb_decrypt(ct, ct_len, key, key_len, &back, &blen),
            (int)AESM_OK);
        ISO_CHECK_EQ_UINT(blen, pt_len);
        ISO_CHECK_MEM_EQ(back, pt, pt_len);
        free(ct);
        free(back);
    }

    /* ── CBC (NIST SP 800-38A) ──────────────────────────────────────────── */
    {
        uint8_t iv[16];
        uint8_t *ct;
        size_t ct_len;
        uint8_t want[16];
        uint8_t *back;
        size_t blen;
        from_hex("000102030405060708090a0b0c0d0e0f", iv);
        ISO_CHECK_EQ_INT((int)aesm_cbc_encrypt(pt, pt_len, key, key_len, iv, 16,
                                               &ct, &ct_len),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(ct_len, 80u);
        from_hex("7649abac8119b246cee98e9b12e9197d", want);
        ISO_CHECK_MEM_EQ(ct, want, 16);
        from_hex("5086cb9b507219ee95db113a917678b2", want);
        ISO_CHECK_MEM_EQ(ct + 16, want, 16);
        ISO_CHECK_EQ_INT((int)aesm_cbc_decrypt(ct, ct_len, key, key_len, iv, 16,
                                               &back, &blen),
                         (int)AESM_OK);
        ISO_CHECK_MEM_EQ(back, pt, pt_len);
        free(ct);
        free(back);
        /* wrong IV length rejected. */
        {
            uint8_t badiv[8] = {0};
            ISO_CHECK_EQ_INT((int)aesm_cbc_encrypt(pt, 16, key, key_len, badiv, 8,
                                                   &ct, &ct_len),
                             (int)AESM_BAD_IV_LENGTH);
        }
    }

    /* ── CTR (round trip; no padding) ───────────────────────────────────── */
    {
        uint8_t nonce[12];
        uint8_t *ct;
        size_t ct_len;
        uint8_t *back;
        size_t blen;
        from_hex("f0f1f2f3f4f5f6f7f8f9fafb", nonce);
        ISO_CHECK_EQ_INT((int)aesm_ctr_encrypt(pt, pt_len, key, key_len, nonce,
                                               12, &ct, &ct_len),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(ct_len, pt_len); /* no padding */
        ISO_CHECK_EQ_INT((int)aesm_ctr_decrypt(ct, ct_len, key, key_len, nonce,
                                               12, &back, &blen),
                         (int)AESM_OK);
        ISO_CHECK_MEM_EQ(back, pt, pt_len);
        free(ct);
        free(back);
        /* wrong nonce length rejected. */
        {
            uint8_t bad[16] = {0};
            uint8_t one = 0;
            ISO_CHECK_EQ_INT((int)aesm_ctr_encrypt(&one, 1, key, key_len, bad, 16,
                                                   &ct, &ct_len),
                             (int)AESM_BAD_NONCE_LENGTH);
        }
    }

    /* ── GCM (classic test case 4) ──────────────────────────────────────── */
    {
        uint8_t gkey[16];
        uint8_t iv[12];
        uint8_t gpt[64];
        uint8_t want_ct[64];
        uint8_t want_tag[16];
        uint8_t *ct;
        size_t ct_len;
        uint8_t tag[16];
        size_t gpt_len;
        from_hex("feffe9928665731c6d6a8f9467308308", gkey);
        from_hex("cafebabefacedbaddecaf888", iv);
        /* GCM test case 3: 64-byte plaintext, empty AAD. Single-line hex
         * literals (splitting risks an odd-length total that overflows). */
        gpt_len = from_hex(
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255",
            gpt);
        from_hex(
            "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985",
            want_ct);
        from_hex("4d5c2af327cd64a62cf35abd2ba6fab4", want_tag);
        ISO_CHECK_EQ_INT((int)aesm_gcm_encrypt(gpt, gpt_len, gkey, 16, iv, 12,
                                               NULL, 0, &ct, &ct_len, tag),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(ct_len, gpt_len);
        ISO_CHECK_MEM_EQ(ct, want_ct, gpt_len);
        ISO_CHECK_MEM_EQ(tag, want_tag, 16);
        /* round trip decrypt verifies the tag. */
        {
            uint8_t *back;
            size_t blen;
            ISO_CHECK_EQ_INT((int)aesm_gcm_decrypt(ct, ct_len, gkey, 16, iv, 12,
                                                   NULL, 0, tag, &back, &blen),
                             (int)AESM_OK);
            ISO_CHECK_MEM_EQ(back, gpt, gpt_len);
            free(back);
        }
        /* tampered ciphertext / tag rejected. */
        {
            uint8_t *back;
            size_t blen;
            ct[0] ^= 0x01;
            ISO_CHECK_EQ_INT((int)aesm_gcm_decrypt(ct, ct_len, gkey, 16, iv, 12,
                                                   NULL, 0, tag, &back, &blen),
                             (int)AESM_AUTH_FAILED);
            ct[0] ^= 0x01; /* restore */
            tag[0] ^= 0x01;
            ISO_CHECK_EQ_INT((int)aesm_gcm_decrypt(ct, ct_len, gkey, 16, iv, 12,
                                                   NULL, 0, tag, &back, &blen),
                             (int)AESM_AUTH_FAILED);
        }
        free(ct);
    }

    /* ── GCM empty pt / empty aad (tag known-answer) ────────────────────── */
    {
        uint8_t gkey[16];
        uint8_t iv[12];
        uint8_t *ct;
        size_t ct_len;
        uint8_t tag[16];
        uint8_t want_tag[16];
        memset(gkey, 0, 16);
        memset(iv, 0, 12);
        ISO_CHECK_EQ_INT((int)aesm_gcm_encrypt(NULL, 0, gkey, 16, iv, 12, NULL, 0,
                                               &ct, &ct_len, tag),
                         (int)AESM_OK);
        ISO_CHECK_EQ_UINT(ct_len, 0u);
        from_hex("58e2fccefa7e3061367f1d57a4e7455a", want_tag);
        ISO_CHECK_MEM_EQ(tag, want_tag, 16);
        free(ct);
    }

    /* ── GCM with AAD round trip; wrong AAD rejected ────────────────────── */
    {
        uint8_t gkey[16];
        uint8_t iv[12];
        uint8_t gpt[16];
        uint8_t aad[16];
        uint8_t wrong_aad[8];
        uint8_t *ct;
        size_t ct_len;
        uint8_t tag[16];
        uint8_t *back;
        size_t blen;
        from_hex("feffe9928665731c6d6a8f9467308308", gkey);
        from_hex("cafebabefacedbaddecaf888", iv);
        from_hex("d9313225f88406e5a55909c5aff5269a", gpt);
        from_hex("feedfacedeadbeeffeedfacedeadbeef", aad);
        from_hex("deadbeeffeedface", wrong_aad);
        ISO_CHECK_EQ_INT((int)aesm_gcm_encrypt(gpt, 16, gkey, 16, iv, 12, aad, 16,
                                               &ct, &ct_len, tag),
                         (int)AESM_OK);
        ISO_CHECK_EQ_INT((int)aesm_gcm_decrypt(ct, ct_len, gkey, 16, iv, 12, aad,
                                               16, tag, &back, &blen),
                         (int)AESM_OK);
        ISO_CHECK_MEM_EQ(back, gpt, 16);
        free(back);
        ISO_CHECK_EQ_INT((int)aesm_gcm_decrypt(ct, ct_len, gkey, 16, iv, 12,
                                               wrong_aad, 8, tag, &back, &blen),
                         (int)AESM_AUTH_FAILED);
        free(ct);
        /* wrong IV length rejected. */
        {
            uint8_t badiv[16] = {0};
            ISO_CHECK_EQ_INT((int)aesm_gcm_encrypt(NULL, 0, gkey, 16, badiv, 16,
                                                   NULL, 0, &ct, &ct_len, tag),
                             (int)AESM_BAD_IV_LENGTH);
        }
    }

    return ISO_TEST_RESULT();
}
