/* Tests for the C wasm-leb128, using the iso_test.h harness. Vectors are taken
 * from the Rust crate's own tests (WASM/DWARF LEB128 vectors). */
#include "iso_test.h"

#include "wasm_leb128.h"

/* Assert encode_unsigned(value) equals expected[0..exp_len]. */
static void check_enc_u(unsigned long long value, const unsigned char *expected,
                        size_t exp_len) {
    unsigned char out[LEB128_MAX_BYTES];
    size_t n = leb128_encode_unsigned(value, out);
    ISO_CHECK_EQ_UINT(n, exp_len);
    if (n == exp_len) {
        ISO_CHECK_MEM_EQ(out, expected, exp_len);
    }
}

static void check_enc_s(long long value, const unsigned char *expected,
                        size_t exp_len) {
    unsigned char out[LEB128_MAX_BYTES];
    size_t n = leb128_encode_signed(value, out);
    ISO_CHECK_EQ_UINT(n, exp_len);
    if (n == exp_len) {
        ISO_CHECK_MEM_EQ(out, expected, exp_len);
    }
}

int main(void) {
    /* ---- unsigned decoding ------------------------------------------- */
    {
        unsigned long long v = 99;
        size_t used = 99;
        const unsigned char z[] = {0x00};
        const unsigned char three[] = {0x03};
        const unsigned char big[] = {0xE5, 0x8E, 0x26};
        const unsigned char maxu32[] = {0xFF, 0xFF, 0xFF, 0xFF, 0x0F};
        const unsigned char offbuf[] = {0x00, 0x00, 0xE5, 0x8E, 0x26};

        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(z, 1, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 0ull && used == 1);
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(three, 1, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 3ull && used == 1);
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(big, 3, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 624485ull && used == 3);
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(maxu32, 5, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 4294967295ull && used == 5);
        /* decode at a non-zero offset (skip two leading zero bytes) */
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(offbuf, 5, 2, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 624485ull && used == 3);
    }

    /* unsigned decode errors */
    {
        unsigned long long v = 7;
        size_t used = 7;
        const unsigned char unterm[] = {0x80, 0x80};
        const unsigned char one[] = {0x01};
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(unterm, 2, 0, &v, &used),
                         (int)LEB128_ERR_UNTERMINATED);
        ISO_CHECK(v == 0ull && used == 0); /* outputs cleared on error */
        ISO_CHECK_EQ_INT((int)leb128_decode_unsigned(one, 1, 5, &v, &used),
                         (int)LEB128_ERR_OFFSET);
    }

    /* ---- signed decoding --------------------------------------------- */
    {
        long long v = 99;
        size_t used = 99;
        const unsigned char z[] = {0x00};
        const unsigned char neg2[] = {0x7E};
        const unsigned char maxi32[] = {0xFF, 0xFF, 0xFF, 0xFF, 0x07};
        const unsigned char mini32[] = {0x80, 0x80, 0x80, 0x80, 0x78};
        const unsigned char offbuf[] = {0x00, 0x00, 0x00, 0x7E};

        ISO_CHECK_EQ_INT((int)leb128_decode_signed(z, 1, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 0 && used == 1);
        ISO_CHECK_EQ_INT((int)leb128_decode_signed(neg2, 1, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == -2 && used == 1);
        ISO_CHECK_EQ_INT((int)leb128_decode_signed(maxi32, 5, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == 2147483647LL && used == 5);
        ISO_CHECK_EQ_INT((int)leb128_decode_signed(mini32, 5, 0, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == -2147483648LL && used == 5);
        ISO_CHECK_EQ_INT((int)leb128_decode_signed(offbuf, 4, 3, &v, &used),
                         (int)LEB128_OK);
        ISO_CHECK(v == -2 && used == 1);
    }

    /* signed decode errors */
    {
        long long v = 7;
        size_t used = 7;
        const unsigned char unterm[] = {0x80, 0x80};
        ISO_CHECK_EQ_INT((int)leb128_decode_signed(unterm, 2, 0, &v, &used),
                         (int)LEB128_ERR_UNTERMINATED);
    }

    /* ---- encoding ---------------------------------------------------- */
    {
        const unsigned char z[] = {0x00};
        const unsigned char three[] = {0x03};
        const unsigned char big[] = {0xE5, 0x8E, 0x26};
        const unsigned char maxu32[] = {0xFF, 0xFF, 0xFF, 0xFF, 0x0F};
        const unsigned char neg2[] = {0x7E};
        const unsigned char mini32[] = {0x80, 0x80, 0x80, 0x80, 0x78};
        const unsigned char maxi32[] = {0xFF, 0xFF, 0xFF, 0xFF, 0x07};
        check_enc_u(0, z, 1);
        check_enc_u(3, three, 1);
        check_enc_u(624485, big, 3);
        check_enc_u(4294967295ull, maxu32, 5);
        check_enc_s(0, z, 1);
        check_enc_s(-2, neg2, 1);
        check_enc_s(-2147483648LL, mini32, 5);
        check_enc_s(2147483647LL, maxi32, 5);
    }

    /* ---- round trips ------------------------------------------------- */
    {
        unsigned long long uvals[] = {0ull,      1ull,       127ull,
                                      128ull,    255ull,     624485ull,
                                      4294967295ull, 0xFFFFFFFFFFFFFFFFull};
        long long svals[] = {0,   1,          -1,        -2,
                             63,  -64,        127,       -128,
                             2147483647LL, -2147483648LL,
                             9223372036854775807LL,
                             (-9223372036854775807LL - 1)};
        size_t i;
        for (i = 0; i < sizeof uvals / sizeof uvals[0]; i++) {
            unsigned char buf[LEB128_MAX_BYTES];
            unsigned long long dec;
            size_t enc_n, used;
            enc_n = leb128_encode_unsigned(uvals[i], buf);
            ISO_CHECK_EQ_INT(
                (int)leb128_decode_unsigned(buf, enc_n, 0, &dec, &used),
                (int)LEB128_OK);
            ISO_CHECK(dec == uvals[i]);
            ISO_CHECK_EQ_UINT(used, enc_n);
        }
        for (i = 0; i < sizeof svals / sizeof svals[0]; i++) {
            unsigned char buf[LEB128_MAX_BYTES];
            long long dec;
            size_t enc_n, used;
            enc_n = leb128_encode_signed(svals[i], buf);
            ISO_CHECK_EQ_INT(
                (int)leb128_decode_signed(buf, enc_n, 0, &dec, &used),
                (int)LEB128_OK);
            ISO_CHECK(dec == svals[i]);
            ISO_CHECK_EQ_UINT(used, enc_n);
        }
    }

    /* An overlong sequence (11 continuation bytes) overflows the 70-bit limit. */
    {
        unsigned char overlong[11];
        unsigned long long v;
        size_t used, i;
        for (i = 0; i < 10; i++) {
            overlong[i] = 0x80;
        }
        overlong[10] = 0x00;
        ISO_CHECK_EQ_INT(
            (int)leb128_decode_unsigned(overlong, 11, 0, &v, &used),
            (int)LEB128_ERR_OVERFLOW);
    }

    return ISO_TEST_RESULT();
}
