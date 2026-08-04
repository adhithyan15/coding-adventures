/* Tests for the C BLAKE2b, using the iso_test.h harness. Pinned to the published
 * RFC 7693 / reference test vectors, plus digest-size, streaming, and keyed
 * checks. */
#include "iso_test.h"

#include <string.h>

#include "blake2b.h"

int main(void) {
    char hex[2 * BLAKE2B_MAX_DIGEST + 1];
    uint8_t digest[BLAKE2B_MAX_DIGEST];

    /* BLAKE2b-512 of the empty string. */
    ISO_CHECK(blake2b_hex("", 0, 64, hex));
    ISO_CHECK_STR_EQ(
        hex,
        "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419"
        "d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce");

    /* BLAKE2b-512 of "abc". */
    ISO_CHECK(blake2b_hex("abc", 3, 64, hex));
    ISO_CHECK_STR_EQ(
        hex,
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
        "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");

    /* "abc" as raw bytes via ISO_CHECK_MEM_EQ. */
    {
        const uint8_t expected[64] = {
            0xba, 0x80, 0xa5, 0x3f, 0x98, 0x1c, 0x4d, 0x0d, 0x6a, 0x27, 0x97,
            0xb6, 0x9f, 0x12, 0xf6, 0xe9, 0x4c, 0x21, 0x2f, 0x14, 0x68, 0x5a,
            0xc4, 0xb7, 0x4b, 0x12, 0xbb, 0x6f, 0xdb, 0xff, 0xa2, 0xd1, 0x7d,
            0x87, 0xc5, 0x39, 0x2a, 0xab, 0x79, 0x2d, 0xc2, 0x52, 0xd5, 0xde,
            0x45, 0x33, 0xcc, 0x95, 0x18, 0xd3, 0x8a, 0xa8, 0xdb, 0xf1, 0x92,
            0x5a, 0xb9, 0x23, 0x86, 0xed, 0xd4, 0x00, 0x99, 0x23};
        ISO_CHECK(blake2b("abc", 3, 64, digest));
        ISO_CHECK_MEM_EQ(digest, expected, 64);
    }

    /* BLAKE2b-256 of "abc" (a smaller digest size). */
    ISO_CHECK(blake2b_hex("abc", 3, 32, hex));
    ISO_CHECK_STR_EQ(
        hex, "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319");

    /* Streaming must equal one-shot. */
    {
        blake2b_ctx ctx;
        static const char hexchars[] = "0123456789abcdef";
        char shex[2 * BLAKE2B_MAX_DIGEST + 1];
        size_t i;
        ISO_CHECK(blake2b_init(&ctx, 64, NULL, 0, NULL, NULL));
        blake2b_update(&ctx, "ab", 2);
        blake2b_update(&ctx, "c", 1);
        blake2b_final(&ctx, digest);
        for (i = 0; i < 64; i++) {
            shex[i * 2] = hexchars[digest[i] >> 4];
            shex[i * 2 + 1] = hexchars[digest[i] & 0x0f];
        }
        shex[128] = '\0';
        ISO_CHECK_STR_EQ(
            shex,
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
            "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");
    }

    /* Keyed hashing: deterministic, and different from the unkeyed hash. */
    {
        const uint8_t key[16] = {1, 2,  3,  4,  5,  6,  7,  8,
                                 9, 10, 11, 12, 13, 14, 15, 16};
        blake2b_ctx a, b;
        uint8_t da[64], db[64], unkeyed[64];
        ISO_CHECK(blake2b_init(&a, 64, key, sizeof key, NULL, NULL));
        blake2b_update(&a, "message", 7);
        blake2b_final(&a, da);
        ISO_CHECK(blake2b_init(&b, 64, key, sizeof key, NULL, NULL));
        blake2b_update(&b, "message", 7);
        blake2b_final(&b, db);
        ISO_CHECK_MEM_EQ(da, db, 64); /* deterministic */
        blake2b("message", 7, 64, unkeyed);
        ISO_CHECK(memcmp(da, unkeyed, 64) != 0); /* key changes the result */
    }

    /* Invalid digest sizes are rejected. */
    ISO_CHECK(!blake2b("x", 1, 0, digest));
    ISO_CHECK(!blake2b("x", 1, 65, digest));

    return ISO_TEST_RESULT();
}
