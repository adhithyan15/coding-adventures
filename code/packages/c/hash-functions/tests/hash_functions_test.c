/* Tests for hash-functions, using the header-only iso_test.h harness (pure ISO).
 * The known-answer vectors mirror the Rust crate's own unit tests. 64-bit values
 * are checked full-width via ISO_CHECK (ISO_CHECK_EQ_UINT narrows to `unsigned
 * long`, which is 32-bit on LLP64/Windows). */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "hash_functions.h"

/* Hash a string literal's bytes (excluding the terminating NUL). */
#define B(s) ((const uint8_t *)(s)), (sizeof(s) - 1)

/* ── Analysis callbacks ────────────────────────────────────────────────────*/

static uint64_t cb_zero(const uint8_t *d, size_t n, void *ctx) {
    (void)d;
    (void)n;
    (void)ctx;
    return 0;
}
static uint64_t cb_len(const uint8_t *d, size_t n, void *ctx) {
    (void)d;
    (void)ctx;
    return (uint64_t)n;
}
/* Matches the Rust test's `deterministic_fill`: an LCG whose byte 24..31 feed
 * the buffer. ctx points at the running seed. */
static void det_fill(uint8_t *buf, size_t len, void *ctx) {
    uint64_t *seed = (uint64_t *)ctx;
    for (size_t i = 0; i < len; i++) {
        *seed = *seed * (uint64_t)6364136223846793005ull + 1u;
        buf[i] = (uint8_t)(*seed >> 24);
    }
}

int main(void) {
    /* ── FNV-1a 32 ─────────────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(hf_fnv1a_32(B("")), 0x811C9DC5u);
    ISO_CHECK_EQ_UINT(hf_fnv1a_32(B("a")), 0xE40C292Cu);
    ISO_CHECK_EQ_UINT(hf_fnv1a_32(B("abc")), 0x1A47E90Bu);
    ISO_CHECK_EQ_UINT(hf_fnv1a_32(B("hello")), 1335831723u);
    ISO_CHECK_EQ_UINT(hf_fnv1a_32(B("foobar")), 3214735720u);

    /* ── FNV-1a 64 ─────────────────────────────────────────────────────────*/
    ISO_CHECK(hf_fnv1a_64(B("")) == 0xCBF29CE484222325ull);
    ISO_CHECK(hf_fnv1a_64(B("a")) == 0xAF63DC4C8601EC8Cull);
    ISO_CHECK(hf_fnv1a_64(B("abc")) == 0xE71FA2190541574Bull);
    ISO_CHECK(hf_fnv1a_64(B("hello")) == 0xA430D84680AABD0Bull);

    /* ── DJB2 ──────────────────────────────────────────────────────────────*/
    ISO_CHECK(hf_djb2(B("")) == 5381ull);
    ISO_CHECK(hf_djb2(B("a")) == 177670ull);
    ISO_CHECK(hf_djb2(B("abc")) == 193485963ull);

    /* ── Polynomial rolling ────────────────────────────────────────────────*/
    ISO_CHECK(hf_polynomial_rolling(B("")) == 0ull);
    ISO_CHECK(hf_polynomial_rolling(B("a")) == 97ull);
    ISO_CHECK(hf_polynomial_rolling(B("ab")) == 3105ull);
    ISO_CHECK(hf_polynomial_rolling(B("abc")) == 96354ull);
    /* base 37, modulus 1e9+7: ((97*37+98)*37+99). */
    ISO_CHECK(hf_polynomial_rolling_with_params(B("abc"), 37, 1000000007ull) ==
              (uint64_t)(((97ull * 37 + 98) * 37 + 99)));
    /* mulmod stays exact even when base*hash exceeds 64 bits: a large modulus
     * near 2^63 with multi-byte input must not overflow. */
    {
        uint64_t big_mod = (((uint64_t)1 << 62) - 57); /* < 2^62 */
        uint64_t h = hf_polynomial_rolling_with_params(B("hello"), 1000003ull,
                                                       big_mod);
        ISO_CHECK(h < big_mod);
    }
    /* Defensive: a zero modulus returns 0 rather than dividing by zero. */
    ISO_CHECK(hf_polynomial_rolling_with_params(B("abc"), 31, 0) == 0ull);

    /* ── Murmur3 (32-bit) ──────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(hf_murmur3_32(B("")), 0u);
    ISO_CHECK_EQ_UINT(hf_murmur3_32_with_seed(B(""), 1), 0x514E28B7u);
    ISO_CHECK_EQ_UINT(hf_murmur3_32(B("a")), 0x3C2569B2u);
    ISO_CHECK_EQ_UINT(hf_murmur3_32(B("abc")), 0xB3DD93FAu);
    ISO_CHECK_EQ_UINT(hf_murmur3_32(B("abcd")), 0x43ED676Au);

    /* ── SipHash-2-4 ───────────────────────────────────────────────────────*/
    {
        uint8_t key[16];
        for (int i = 0; i < 16; i++) key[i] = (uint8_t)i;
        ISO_CHECK(hf_siphash_2_4(B(""), key) == 0x726FDB47DD0E0E31ull);
        {
            uint8_t one_zero[1] = {0x00};
            ISO_CHECK(hf_siphash_2_4(one_zero, 1, key) ==
                      0x74F839C593DC67FDull);
        }
    }

    /* ── String helpers ────────────────────────────────────────────────────*/
    {
        uint8_t zero_key[16];
        memset(zero_key, 0, sizeof zero_key);
        ISO_CHECK_EQ_UINT(hf_hash_str_fnv1a_32("hello"),
                          hf_fnv1a_32(B("hello")));
        ISO_CHECK(hf_hash_str_siphash("hello", zero_key) ==
                  hf_siphash_2_4(B("hello"), zero_key));
    }

    /* ── HashFunction dispatch forwards to the free functions ──────────────*/
    {
        uint8_t zero_key[16];
        HfHashFunction fnv32 = hf_new_fnv1a_32();
        HfHashFunction fnv64 = hf_new_fnv1a_64();
        HfHashFunction djb = hf_new_djb2();
        HfHashFunction poly = hf_new_polynomial_rolling();
        HfHashFunction murmur = hf_new_murmur3_32();
        HfHashFunction sip;
        memset(zero_key, 0, sizeof zero_key);
        sip = hf_new_siphash_2_4(zero_key);

        ISO_CHECK(hf_hash(&fnv32, B("abc")) == (uint64_t)hf_fnv1a_32(B("abc")));
        ISO_CHECK(hf_hash(&fnv64, B("abc")) == hf_fnv1a_64(B("abc")));
        ISO_CHECK(hf_hash(&djb, B("abc")) == hf_djb2(B("abc")));
        ISO_CHECK(hf_hash(&poly, B("abc")) == hf_polynomial_rolling(B("abc")));
        ISO_CHECK(hf_hash(&murmur, B("abc")) ==
                  (uint64_t)hf_murmur3_32(B("abc")));
        ISO_CHECK(hf_hash(&sip, B("abc")) ==
                  hf_siphash_2_4(B("abc"), zero_key));

        ISO_CHECK_EQ_UINT(hf_output_bits(&fnv32), 32u);
        ISO_CHECK_EQ_UINT(hf_output_bits(&fnv64), 64u);
        ISO_CHECK_EQ_UINT(hf_output_bits(&djb), 64u);
        ISO_CHECK_EQ_UINT(hf_output_bits(&poly), 64u);
        ISO_CHECK_EQ_UINT(hf_output_bits(&murmur), 32u);
        ISO_CHECK_EQ_UINT(hf_output_bits(&sip), 64u);
    }
    /* Non-default polynomial constructor carries its params through dispatch. */
    {
        HfHashFunction poly = hf_new_polynomial_rolling_with(37, 1000000007ull);
        ISO_CHECK(hf_hash(&poly, B("abc")) ==
                  hf_polynomial_rolling_with_params(B("abc"), 37,
                                                    1000000007ull));
    }

    /* ── Analysis: avalanche ───────────────────────────────────────────────*/
    {
        /* A constant hash never flips an output bit → score 0.0. */
        uint64_t seed = 1;
        double score = hf_avalanche_score(cb_zero, NULL, 32, 4, det_fill, &seed);
        ISO_CHECK_EQ_DBL(score, 0.0, 1e-12);
    }
    {
        /* Contract violations return 0.0. */
        uint64_t seed = 1;
        ISO_CHECK_EQ_DBL(hf_avalanche_score(cb_zero, NULL, 32, 0, det_fill,
                                            &seed),
                         0.0, 1e-12);
        ISO_CHECK_EQ_DBL(hf_avalanche_score(cb_zero, NULL, 65, 4, det_fill,
                                            &seed),
                         0.0, 1e-12);
    }

    /* ── Analysis: distribution ────────────────────────────────────────────*/
    {
        /* Constant hash dumps every input into bucket 0. counts=[4,0,0,0],
         * expected=1 → chi2 = 9 + 1 + 1 + 1 = 12.0. */
        HfInput inputs[4];
        inputs[0].data = (const uint8_t *)"a";
        inputs[0].len = 1;
        inputs[1].data = (const uint8_t *)"b";
        inputs[1].len = 1;
        inputs[2].data = (const uint8_t *)"c";
        inputs[2].len = 1;
        inputs[3].data = (const uint8_t *)"d";
        inputs[3].len = 1;
        ISO_CHECK_EQ_DBL(hf_distribution_test(cb_zero, NULL, inputs, 4, 4), 12.0,
                         1e-9);
        /* A length-based hash yields a non-negative chi-square. */
        {
            HfInput two[2];
            two[0].data = (const uint8_t *)"hello";
            two[0].len = 5;
            two[1].data = (const uint8_t *)"world";
            two[1].len = 5;
            ISO_CHECK(hf_distribution_test(cb_len, NULL, two, 2, 4) >= 0.0);
        }
        /* Contract violations return a negative sentinel. */
        ISO_CHECK(hf_distribution_test(cb_zero, NULL, inputs, 4, 0) < 0.0);
        ISO_CHECK(hf_distribution_test(cb_zero, NULL, inputs, 0, 4) < 0.0);
    }

    return ISO_TEST_RESULT();
}
