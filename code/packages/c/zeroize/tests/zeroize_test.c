/*
 * Tests for the C zeroize library, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own tests: after a wipe, the
 * bytes must read back as zero (including a growable buffer's full capacity).
 */
#include "iso_test.h"

#include <stdint.h>

#include "zeroize.h"

/* True if the first n bytes of buf are all zero. */
static int all_zero(const unsigned char *buf, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        if (buf[i] != 0) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    ISO_CHECK_STR_EQ(ZEROIZE_VERSION, "0.1.0");

    /* ── byte buffer is zeroed ───────────────────────────────────────────── */
    {
        unsigned char buf[32];
        size_t i;
        for (i = 0; i < 32; i++) {
            buf[i] = 0xAA;
        }
        zeroize_bytes(buf, 32);
        ISO_CHECK(all_zero(buf, 32));
    }

    /* ── an empty wipe is a no-op (NULL allowed when len == 0) ───────────── */
    {
        zeroize_bytes(NULL, 0);
        unsigned char one[1] = {0x7F};
        zeroize_bytes(one, 0); /* len 0 -> untouched */
        ISO_CHECK(one[0] == 0x7F);
    }

    /* ── zeroize_object over an arbitrary struct ─────────────────────────── */
    {
        struct {
            uint32_t a;
            uint8_t key[16];
            double d;
        } secret;
        size_t i;
        for (i = 0; i < 16; i++) {
            secret.key[i] = 0xFF;
        }
        secret.a = 0xDEADBEEFu;
        secret.d = 3.14;
        zeroize_object(&secret, sizeof secret);
        ISO_CHECK(secret.a == 0u);
        ISO_CHECK(all_zero(secret.key, 16));
        ISO_CHECK(secret.d == 0.0);
    }

    /* ── typed integer wipes ─────────────────────────────────────────────── */
    {
        uint64_t x = 0xDEADBEEFCAFEF00Dull;
        zeroize_u64(&x);
        ISO_CHECK(x == 0u);

        int32_t y = -12345;
        zeroize_i32(&y);
        ISO_CHECK(y == 0);

        uint32_t u = 0xFFFFFFFFu;
        zeroize_u32(&u);
        ISO_CHECK(u == 0u);

        uint8_t b = 0xAA;
        zeroize_u8(&b);
        ISO_CHECK(b == 0u);

        uint16_t h = 0xBEEF;
        zeroize_u16(&h);
        ISO_CHECK(h == 0u);

        int8_t s8 = -1;
        zeroize_i8(&s8);
        ISO_CHECK(s8 == 0);

        int16_t s16 = -1;
        zeroize_i16(&s16);
        ISO_CHECK(s16 == 0);

        int64_t s64 = -1;
        zeroize_i64(&s64);
        ISO_CHECK(s64 == 0);

        size_t sz = (size_t)-1;
        zeroize_size(&sz);
        ISO_CHECK(sz == 0u);
    }

    /* ── ZrBytes scrubs the FULL capacity, then clears the length ────────── */
    {
        ZrBytes b;
        zr_bytes_init(&b);
        unsigned char sixteen[16];
        size_t i;
        for (i = 0; i < 16; i++) {
            sixteen[i] = 0xFF;
        }
        ISO_CHECK(zr_bytes_extend(&b, sixteen, 16) == 0);
        ISO_CHECK(zr_bytes_reserve(&b, 48) == 0); /* force cap >= 64 */
        ISO_CHECK(b.cap >= 64);
        ISO_CHECK_EQ_UINT(b.len, 16u);

        /* Poison the unused capacity tail directly. */
        for (i = 16; i < b.cap; i++) {
            b.data[i] = 0xAA;
        }

        size_t cap_before = b.cap;
        zr_bytes_zeroize(&b);
        ISO_CHECK_EQ_UINT(b.len, 0u);
        ISO_CHECK(b.cap == cap_before); /* allocation kept */
        /* The whole capacity window (live prefix + tail) is now zero. */
        ISO_CHECK(all_zero(b.data, cap_before));

        zr_bytes_free(&b);
        ISO_CHECK(b.data == NULL);
    }

    /* ── push builds a buffer; zeroize then reuse ────────────────────────── */
    {
        ZrBytes b;
        zr_bytes_init(&b);
        ISO_CHECK(zr_bytes_push(&b, 0x11) == 0);
        ISO_CHECK(zr_bytes_push(&b, 0x22) == 0);
        ISO_CHECK_EQ_UINT(b.len, 2u);
        ISO_CHECK(b.data[0] == 0x11 && b.data[1] == 0x22);
        zr_bytes_zeroize(&b);
        ISO_CHECK(b.len == 0u && b.data[0] == 0 && b.data[1] == 0);
        zr_bytes_free(&b);
    }

    return ISO_TEST_RESULT();
}
