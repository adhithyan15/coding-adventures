/* Tests for the C SHA-256, using the iso_test.h harness. Pinned to the published
 * FIPS 180-4 test vectors, plus streaming and multi-block-boundary checks. */
#include "iso_test.h"

#include <string.h>

#include "sha256.h"

int main(void) {
    char hex[SHA256_HEX_SIZE];
    uint8_t digest[SHA256_DIGEST_SIZE];

    /* Empty string. */
    sha256_hex("", 0, hex);
    ISO_CHECK_STR_EQ(
        hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

    /* "abc" — the canonical single-block vector. */
    sha256_hex("abc", 3, hex);
    ISO_CHECK_STR_EQ(
        hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

    /* Same vector, checked as raw bytes via ISO_CHECK_MEM_EQ. */
    {
        const uint8_t expected_abc[SHA256_DIGEST_SIZE] = {
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40,
            0xde, 0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17,
            0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad};
        sha256("abc", 3, digest);
        ISO_CHECK_MEM_EQ(digest, expected_abc, SHA256_DIGEST_SIZE);
    }

    /* 56-byte message — exercises the padding-boundary case (needs a second
     * padding block). */
    sha256_hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", 56,
               hex);
    ISO_CHECK_STR_EQ(
        hex, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");

    /* Streaming must equal the one-shot result. */
    {
        sha256_ctx ctx;
        char shex[SHA256_HEX_SIZE];
        sha256_init(&ctx);
        sha256_update(&ctx, "ab", 2);
        sha256_update(&ctx, "c", 1);
        sha256_final(&ctx, digest);
        {
            static const char hexchars[] = "0123456789abcdef";
            size_t i;
            for (i = 0; i < SHA256_DIGEST_SIZE; i++) {
                shex[i * 2] = hexchars[digest[i] >> 4];
                shex[i * 2 + 1] = hexchars[digest[i] & 0x0f];
            }
            shex[64] = '\0';
        }
        ISO_CHECK_STR_EQ(
            shex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    /* A 64-byte input (exactly one block, so padding spills into a second
     * block): 64 'a' characters. */
    {
        char buf[64];
        memset(buf, 'a', sizeof buf);
        sha256_hex(buf, sizeof buf, hex);
        ISO_CHECK_STR_EQ(
            hex,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
    }

    return ISO_TEST_RESULT();
}
