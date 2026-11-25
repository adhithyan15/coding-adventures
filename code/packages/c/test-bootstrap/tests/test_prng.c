/*
 * test_prng.c -- SplitMix64, rejection sampling and FNV-1a (VER05i section 2).
 *
 * Every pinned value comes from the independent reference
 * code/scripts/pbt_conformance.py (or from the corpus), never from this code.
 *
 * The corpus draws spans so small that a WRONG rejection threshold almost never
 * fires, so the rejection branch is FORCED: the SplitMix64 finalizer is
 * invertible, hence tu_state_for_output() builds a state whose next output is
 * exactly the word we choose.
 */
#include <stdio.h>
#include <string.h>

#include "iso_test.h"
#include "pbt.h"
#include "pbt_hooks.h"
#include "test_util.h"

static pbt_rng at_output(uint64_t out)
{
    pbt_rng r;
    r.state = tu_state_for_output(out);
    return r;
}

static void test_reference_vectors(void)
{
    pbt_rng r;
    static const uint64_t first4[] = {0xe220a8397b1dcdafu, 0x6e789e6aa1b965f4u, 0x06c45d188009454fu, 0xf88bb8a8724c81ecu};
    size_t i;
    pbt_rng_init(&r, 0);
    for (i = 0; i < 4; i++) {
        ISO_CHECK(pbt_rng_next(&r) == first4[i]);
    }
    /* corpus pbt/gen_primitive/u64_seed0 */
    pbt_rng_init(&r, 0);
    ISO_CHECK(pbt_rng_next(&r) == 16294208416658607535u);
    /* The inverse really inverts: forcing an output and drawing it round-trips. */
    for (i = 0; i < 64; i++) {
        uint64_t want = (uint64_t)0x123456789abcdef1u * (i + 1) + i;
        pbt_rng f = at_output(want);
        ISO_CHECK(pbt_rng_next(&f) == want);
    }
}

static void test_fnv_and_sub_seed(void)
{
    ISO_CHECK(pbt_fnv1a64("", 0) == 0xcbf29ce484222325u);
    ISO_CHECK(pbt_fnv1a64("a", 1) == 0xaf63dc4c8601ec8cu);
    ISO_CHECK(pbt_fnv1a64("foobar", 6) == 0x85944171f73967e8u);
    /* sub_seed = SplitMix64(seed ^ fnv1a64(id)).next() (VER05 section 2.3), vectors from the reference. */
    ISO_CHECK(pbt_sub_seed(0, "prop/a") == 10806796643228322761u);
    ISO_CHECK(pbt_sub_seed(12345, "prop/a") == 6519938991058809955u);
    ISO_CHECK(pbt_sub_seed(UINT64_MAX, "prop/a") == 4475463234772179925u);
    ISO_CHECK(pbt_sub_seed(0, "pbt/replay/seed42_le_100") == 4972117744798573231u);
    ISO_CHECK(pbt_sub_seed(12345, "pbt/replay/seed42_le_100") == 10797653868158221918u);
    ISO_CHECK(pbt_sub_seed(UINT64_MAX, "pbt/replay/seed42_le_100") == 3820293018434567614u);
    ISO_CHECK(pbt_sub_seed(0, "") == 14087677454934409008u);
    ISO_CHECK(pbt_sub_seed(12345, "") == 6239939107816789547u);
    ISO_CHECK(pbt_sub_seed(UINT64_MAX, "") == 3526150024844925518u);
    /* Different ids give different sub-seeds. */
    ISO_CHECK(pbt_sub_seed(1, "a") != pbt_sub_seed(1, "b"));
}

static void test_bounded_basics(void)
{
    pbt_rng r;
    pbt_rng twin;
    int hits[6] = {0, 0, 0, 0, 0, 0};
    int i;
    /* bounded(1) CONSUMES one word (the section 2.2 loop run once). */
    pbt_rng_init(&r, 99);
    pbt_rng_init(&twin, 99);
    ISO_CHECK(pbt_rng_bounded(&r, 1) == 0);
    (void)pbt_rng_next(&twin);
    ISO_CHECK(r.state == twin.state);
    /* bounded(0) is a caller bug: it returns 0 and consumes nothing. */
    pbt_rng_init(&r, 99);
    pbt_rng_init(&twin, 99);
    ISO_CHECK(pbt_rng_bounded(&r, 0) == 0);
    ISO_CHECK(r.state == twin.state);
    /* Uniform enough on a small span, and always in range. */
    pbt_rng_init(&r, 5);
    for (i = 0; i < 6000; i++) {
        uint64_t v = pbt_rng_bounded(&r, 6);
        ISO_CHECK(v < 6);
        if (v < 6) {
            hits[v]++;
        }
    }
    for (i = 0; i < 6; i++) {
        ISO_CHECK(hits[i] > 850 && hits[i] < 1150);
    }
}

/*
 * THE FORCED REJECTION DRAW.  bounded(3): threshold t = (2^64 - 3) mod 3 = 1,
 * so the single word 0 must be REJECTED (0 < 1) and the next one used.
 */
static void test_forced_rejection_small(void)
{
    pbt_rng r = at_output(0);
    pbt_rng probe = at_output(0);
    uint64_t second;
    (void)pbt_rng_next(&probe);
    second = pbt_rng_next(&probe); /* the word a correct bounded(3) falls through to */
    ISO_CHECK(pbt_rng_bounded(&r, 3) == second % 3);
    ISO_CHECK(r.state == probe.state); /* exactly two words consumed */
    /* A word equal to the threshold is ACCEPTED: 1 % 3 = 1, one word consumed. */
    r = at_output(1);
    ISO_CHECK(pbt_rng_bounded(&r, 3) == 1);
    {
        pbt_rng one = at_output(1);
        (void)pbt_rng_next(&one);
        ISO_CHECK(r.state == one.state);
    }
    /* And the unrejected modulo would have answered 0 for the first case. */
    ISO_CHECK(0 % 3 == 0);
}

/* A threshold computed through SIGNED arithmetic: wrong for n > 2^63. */
static uint64_t signed_threshold(uint64_t n)
{
    int64_t sn = (int64_t)n;
    return (uint64_t)(((int64_t)0 - sn) % sn);
}

/*
 * n = 2^63 + 1: t = (2^64 - n) mod n = 2^63 - 1.  A draw of 2^63 - 2 is
 * rejected, a draw of 2^63 - 1 accepted.  The signed spelling computes t = 0
 * and accepts both.
 */
static void test_threshold_two_pow_63_plus_1(void)
{
    const uint64_t n = ((uint64_t)1 << 63) + 1;
    const uint64_t t = ((uint64_t)1 << 63) - 1;
    pbt_rng r;
    pbt_rng probe;
    uint64_t second;
    ISO_CHECK(((uint64_t)0 - n) % n == t);

    /* Rejected: consumes the next word too. */
    r = at_output(t - 1);
    probe = at_output(t - 1);
    (void)pbt_rng_next(&probe);
    second = pbt_rng_next(&probe);
    ISO_CHECK(pbt_rng_bounded(&r, n) == second % n);
    ISO_CHECK(r.state == probe.state);

    /* Accepted at exactly the threshold: the answer is the word itself (it is < n). */
    r = at_output(t);
    ISO_CHECK(pbt_rng_bounded(&r, n) == t);

    /* The wrong (signed) threshold gives a DIFFERENT answer for the rejected word: the test can fail. */
    pbt_hooks.threshold = signed_threshold;
    ISO_CHECK(signed_threshold(n) != t);
    r = at_output(t - 1);
    ISO_CHECK(pbt_rng_bounded(&r, n) == t - 1); /* the sabotaged engine accepts it */
    pbt_hooks_reset();
    r = at_output(t - 1);
    ISO_CHECK(pbt_rng_bounded(&r, n) != t - 1); /* the real engine does not */

    /* n = 2^64 - 1 (threshold 1) rejects 0 only. */
    r = at_output(0);
    probe = at_output(0);
    (void)pbt_rng_next(&probe);
    second = pbt_rng_next(&probe);
    ISO_CHECK(pbt_rng_bounded(&r, UINT64_MAX) == second % UINT64_MAX);
}

static pbt_big pow2(uint32_t bits)
{
    pbt_big p;
    (void)pbt_big_pow2(&p, bits);
    return p;
}

static void test_bounded_big(void)
{
    pbt_rng r;
    pbt_rng twin;
    pbt_big span;
    pbt_big out;
    pbt_big want;
    int i;

    /* Spans that fit a word are exactly bounded(). */
    span = tu_big("1000");
    pbt_rng_init(&r, 3);
    pbt_rng_init(&twin, 3);
    for (i = 0; i < 50; i++) {
        ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
        ISO_CHECK(pbt_big_low_u64(&out) == pbt_rng_bounded(&twin, 1000));
    }
    /* span 1 consumes a word and returns 0. */
    span = tu_big("1");
    pbt_rng_init(&r, 3);
    pbt_rng_init(&twin, 3);
    ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
    ISO_CHECK(pbt_big_is_zero(&out));
    (void)pbt_rng_next(&twin);
    ISO_CHECK(r.state == twin.state);

    /* Exactly 2^64: one raw word. */
    span = pow2(64);
    pbt_rng_init(&r, 3);
    pbt_rng_init(&twin, 3);
    ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
    ISO_CHECK(pbt_big_low_u64(&out) == pbt_rng_next(&twin));
    ISO_CHECK(r.state == twin.state);

    /* 2^64 + 1: k = 2 words, vectors from the reference. */
    span = tu_big("18446744073709551617");
    pbt_rng_init(&r, 7);
    {
        static const char *const want_dec[] = {"11565343845412132934", "12583808255195414474", "14701863683674376248"};
        for (i = 0; i < 3; i++) {
            char buf[PBT_BIG_DEC_BUF];
            ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
            (void)pbt_big_to_dec(&out, buf, sizeof buf);
            ISO_CHECK_STR_EQ(buf, want_dec[i]);
        }
    }
    /* 3 * 2^126. */
    span = tu_big("255211775190703847597530955573826158592");
    pbt_rng_init(&r, 9);
    {
        static const char *const want_dec[] = {"232196006557297741543928818731175635554",
                                               "90284548084330202170444353266735210592",
                                               "89342299821301620962466552557356346366"};
        for (i = 0; i < 3; i++) {
            char buf[PBT_BIG_DEC_BUF];
            ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
            (void)pbt_big_to_dec(&out, buf, sizeof buf);
            ISO_CHECK_STR_EQ(buf, want_dec[i]);
        }
    }
    /* 2^200 + 12345: k = 4 words. */
    {
        pbt_big extra = tu_big("12345");
        (void)pbt_big_pow2(&span, 200);
        (void)pbt_big_add(&span, &span, &extra);
    }
    pbt_rng_init(&r, 11);
    {
        static const char *const want_dec[] = {"987151865205212676757460180008300172715522250639346979693376",
                                               "1233776509181397232204563196526546835661125572160749509013017"};
        for (i = 0; i < 2; i++) {
            char buf[PBT_BIG_DEC_BUF];
            ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
            (void)pbt_big_to_dec(&out, buf, sizeof buf);
            ISO_CHECK_STR_EQ(buf, want_dec[i]);
        }
    }

    /* The FIRST word drawn is the most significant: span 2^128, offset = w1 << 64 | w2. */
    span = pow2(128);
    pbt_rng_init(&r, 21);
    pbt_rng_init(&twin, 21);
    {
        uint64_t w1 = pbt_rng_next(&twin);
        uint64_t w2 = pbt_rng_next(&twin);
        pbt_big hi;
        pbt_big lo;
        pbt_big_set_u64(&hi, w1);
        (void)pbt_big_shl(&hi, &hi, 64);
        pbt_big_set_u64(&lo, w2);
        (void)pbt_big_add(&want, &hi, &lo);
        ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
        ISO_CHECK(pbt_big_cmp(&out, &want) == 0);
    }

    /* Invalid spans: zero, negative, and wider than the cap. */
    pbt_big_zero(&span);
    ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) != 0);
    span = tu_big("-5");
    ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) != 0);
    span = pow2(1010);
    ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) != 0);
}

/*
 * 3 * 2^126 is the span that exposes an unrejected `raw % span`: that draws the low
 * third of the range with probability 1/2 instead of 1/3 (VER05 section 7.2).
 */
static void test_three_thirds_are_even(void)
{
    pbt_big span = tu_big("255211775190703847597530955573826158592");
    pbt_big third = pow2(126);
    pbt_big two_thirds;
    pbt_rng r;
    int bucket[3] = {0, 0, 0};
    int i;
    (void)pbt_big_add(&two_thirds, &third, &third);
    pbt_rng_init(&r, 2024);
    for (i = 0; i < 6000; i++) {
        pbt_big out;
        ISO_CHECK(pbt_rng_bounded_big(&r, &span, &out) == 0);
        if (pbt_big_cmp(&out, &third) < 0) {
            bucket[0]++;
        } else if (pbt_big_cmp(&out, &two_thirds) < 0) {
            bucket[1]++;
        } else {
            bucket[2]++;
        }
    }
    for (i = 0; i < 3; i++) {
        if (bucket[i] < 1800 || bucket[i] > 2200) {
            printf("  bucket %d: %d of 6000\n", i, bucket[i]);
        }
        ISO_CHECK(bucket[i] > 1800 && bucket[i] < 2200);
    }
}

int main(void)
{
    pbt_hooks_reset();
    test_reference_vectors();
    test_fnv_and_sub_seed();
    test_bounded_basics();
    test_forced_rejection_small();
    test_threshold_two_pow_63_plus_1();
    test_bounded_big();
    test_three_thirds_are_even();
    pbt_hooks_reset();
    return ISO_TEST_RESULT();
}
