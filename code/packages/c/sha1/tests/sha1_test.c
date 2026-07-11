/* Tests for the C SHA-1, using the iso_test.h harness. Pinned to the published
 * FIPS test vectors, plus streaming and padding-boundary checks. */
#include "iso_test.h"

#include "sha1.h"

int main(void) {
    char hex[SHA1_HEX_SIZE];
    uint8_t digest[SHA1_DIGEST_SIZE];

    /* Empty string. */
    sha1_hex("", 0, hex);
    ISO_CHECK_STR_EQ(hex, "da39a3ee5e6b4b0d3255bfef95601890afd80709");

    /* "abc". */
    sha1_hex("abc", 3, hex);
    ISO_CHECK_STR_EQ(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");

    /* "abc" as raw bytes via ISO_CHECK_MEM_EQ. */
    {
        const uint8_t expected[SHA1_DIGEST_SIZE] = {
            0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e,
            0x25, 0x71, 0x78, 0x50, 0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d};
        sha1("abc", 3, digest);
        ISO_CHECK_MEM_EQ(digest, expected, SHA1_DIGEST_SIZE);
    }

    /* 56-byte padding-boundary vector. */
    sha1_hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", 56, hex);
    ISO_CHECK_STR_EQ(hex, "84983e441c3bd26ebaae4aa1f95129e5e54670f1");

    /* Streaming equals one-shot. */
    {
        sha1_ctx ctx;
        char shex[SHA1_HEX_SIZE];
        static const char hexchars[] = "0123456789abcdef";
        size_t i;
        sha1_init(&ctx);
        sha1_update(&ctx, "ab", 2);
        sha1_update(&ctx, "c", 1);
        sha1_final(&ctx, digest);
        for (i = 0; i < SHA1_DIGEST_SIZE; i++) {
            shex[i * 2] = hexchars[digest[i] >> 4];
            shex[i * 2 + 1] = hexchars[digest[i] & 0x0f];
        }
        shex[40] = '\0';
        ISO_CHECK_STR_EQ(shex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    return ISO_TEST_RESULT();
}
