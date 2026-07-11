/* Tests for the C rng, using the iso_test.h harness. Pinned to the crate's
 * reference values (seed = 1) plus determinism, range, and float checks. */
#include "iso_test.h"

#include "rng.h"

int main(void) {
    /* Reference values: first three next_u32() for seed = 1. */
    {
        rng_lcg g;
        rng_lcg_init(&g, 1);
        ISO_CHECK_EQ_UINT(rng_lcg_next_u32(&g), 1817669548u);
        ISO_CHECK_EQ_UINT(rng_lcg_next_u32(&g), 2187888307u);
        ISO_CHECK_EQ_UINT(rng_lcg_next_u32(&g), 2784682393u);
    }
    {
        rng_xorshift64 g;
        rng_xorshift64_init(&g, 1);
        ISO_CHECK_EQ_UINT(rng_xorshift64_next_u32(&g), 1082269761u);
        ISO_CHECK_EQ_UINT(rng_xorshift64_next_u32(&g), 201397313u);
        ISO_CHECK_EQ_UINT(rng_xorshift64_next_u32(&g), 1854285353u);
    }
    {
        rng_pcg32 g;
        rng_pcg32_init(&g, 1);
        ISO_CHECK_EQ_UINT(rng_pcg32_next_u32(&g), 1412771199u);
        ISO_CHECK_EQ_UINT(rng_pcg32_next_u32(&g), 1791099446u);
        ISO_CHECK_EQ_UINT(rng_pcg32_next_u32(&g), 124312908u);
    }

    /* Determinism: same seed -> identical sequence. */
    {
        rng_pcg32 a, b;
        int i;
        int same = 1;
        rng_pcg32_init(&a, 42);
        rng_pcg32_init(&b, 42);
        for (i = 0; i < 10; i++) {
            if (rng_pcg32_next_u32(&a) != rng_pcg32_next_u32(&b)) {
                same = 0;
            }
        }
        ISO_CHECK_MSG(same, "same seed must produce the same sequence");
    }

    /* Different seeds -> different sequences. */
    {
        rng_lcg a, b;
        int i;
        int differ = 0;
        rng_lcg_init(&a, 1);
        rng_lcg_init(&b, 2);
        for (i = 0; i < 5; i++) {
            if (rng_lcg_next_u32(&a) != rng_lcg_next_u32(&b)) {
                differ = 1;
            }
        }
        ISO_CHECK_MSG(differ, "different seeds must diverge");
    }

    /* Xorshift64 seed 0 is remapped to 1 and never yields 0. */
    {
        rng_xorshift64 z, o;
        int i;
        int never_zero = 1;
        rng_xorshift64_init(&z, 0);
        rng_xorshift64_init(&o, 1);
        /* seed 0 behaves like seed 1. */
        ISO_CHECK_EQ_UINT(rng_xorshift64_next_u32(&z), rng_xorshift64_next_u32(&o));
        for (i = 0; i < 100; i++) {
            if (rng_xorshift64_next_u32(&z) == 0) {
                never_zero = 0;
            }
        }
        ISO_CHECK_MSG(never_zero, "Xorshift64 state must not collapse to zero");
    }

    /* next_u64 packs two u32 draws: (hi << 32) | lo. */
    {
        rng_lcg a, b;
        uint64_t combined;
        uint32_t hi, lo;
        rng_lcg_init(&a, 7);
        rng_lcg_init(&b, 7);
        combined = rng_lcg_next_u64(&a);
        hi = rng_lcg_next_u32(&b);
        lo = rng_lcg_next_u32(&b);
        ISO_CHECK(combined == (((uint64_t)hi << 32) | lo));
    }

    /* next_float lands in [0, 1). */
    {
        rng_pcg32 g;
        int i;
        int in_range = 1;
        rng_pcg32_init(&g, 123);
        for (i = 0; i < 1000; i++) {
            double f = rng_pcg32_next_float(&g);
            if (!(f >= 0.0 && f < 1.0)) {
                in_range = 0;
            }
        }
        ISO_CHECK_MSG(in_range, "next_float must be in [0, 1)");
    }

    /* next_int_in_range stays within [min, max] and hits the ends. */
    {
        rng_xorshift64 g;
        int i;
        int in_range = 1;
        int saw_min = 0, saw_max = 0;
        rng_xorshift64_init(&g, 55);
        for (i = 0; i < 5000; i++) {
            int64_t v = rng_xorshift64_next_int_in_range(&g, -3, 3);
            if (v < -3 || v > 3) {
                in_range = 0;
            }
            if (v == -3) {
                saw_min = 1;
            }
            if (v == 3) {
                saw_max = 1;
            }
        }
        ISO_CHECK_MSG(in_range, "range draws must stay in [min, max]");
        ISO_CHECK(saw_min && saw_max);
        /* A single-value range is always that value. */
        ISO_CHECK_EQ_INT((int)rng_xorshift64_next_int_in_range(&g, 42, 42), 42);
    }

    return ISO_TEST_RESULT();
}
