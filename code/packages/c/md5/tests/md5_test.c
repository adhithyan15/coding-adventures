/* Tests for the C MD5, using the iso_test.h harness. Pinned to the RFC 1321
 * test suite, plus streaming and padding-boundary checks. */
#include "iso_test.h"

#include "md5.h"

int main(void) {
    char hex[MD5_HEX_SIZE];
    uint8_t digest[MD5_DIGEST_SIZE];

    /* RFC 1321 test suite. */
    md5_hex("", 0, hex);
    ISO_CHECK_STR_EQ(hex, "d41d8cd98f00b204e9800998ecf8427e");

    md5_hex("a", 1, hex);
    ISO_CHECK_STR_EQ(hex, "0cc175b9c0f1b6a831c399e269772661");

    md5_hex("abc", 3, hex);
    ISO_CHECK_STR_EQ(hex, "900150983cd24fb0d6963f7d28e17f72");

    md5_hex("message digest", 14, hex);
    ISO_CHECK_STR_EQ(hex, "f96b697d7cb7938d525a2f31aaf161d0");

    /* Raw digest via ISO_CHECK_MEM_EQ. */
    {
        const uint8_t expected_abc[MD5_DIGEST_SIZE] = {
            0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0,
            0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72};
        md5("abc", 3, digest);
        ISO_CHECK_MEM_EQ(digest, expected_abc, MD5_DIGEST_SIZE);
    }

    /* 62-char message crossing the 56-byte padding boundary. */
    md5_hex("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789", 62,
            hex);
    ISO_CHECK_STR_EQ(hex, "d174ab98d277d9f5a5611c2c9f419d9f");

    /* Streaming equals one-shot. */
    {
        md5_ctx ctx;
        char shex[MD5_HEX_SIZE];
        static const char hexchars[] = "0123456789abcdef";
        size_t i;
        md5_init(&ctx);
        md5_update(&ctx, "ab", 2);
        md5_update(&ctx, "c", 1);
        md5_final(&ctx, digest);
        for (i = 0; i < MD5_DIGEST_SIZE; i++) {
            shex[i * 2] = hexchars[digest[i] >> 4];
            shex[i * 2 + 1] = hexchars[digest[i] & 0x0f];
        }
        shex[32] = '\0';
        ISO_CHECK_STR_EQ(shex, "900150983cd24fb0d6963f7d28e17f72");
    }

    return ISO_TEST_RESULT();
}
