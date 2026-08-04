/* Tests for the C Bloom filter, using the iso_test.h harness. Mirrors the Rust
 * crate's test suite: no false negatives, consistent stats, the sizing formulas,
 * and parameter validation. */
#include "iso_test.h"

#include <math.h> /* INFINITY, NAN (constants only — no libm needed) */
#include <stdio.h>
#include <string.h>

#include "bloom_filter.h"

static void add_str(bloom_filter *bf, const char *s) {
    bloom_add(bf, s, strlen(s));
}
static int contains_str(const bloom_filter *bf, const char *s) {
    return bloom_contains(bf, s, strlen(s));
}

int main(void) {
    /* A fresh filter is empty. */
    {
        bloom_filter bf;
        ISO_CHECK_EQ_INT(bloom_init(&bf, 1000, 0.01), BLOOM_OK);
        ISO_CHECK_EQ_UINT(bloom_bits_set(&bf), 0);
        ISO_CHECK_EQ_DBL(bloom_fill_ratio(&bf), 0.0, 1e-12);
        ISO_CHECK(!bloom_is_over_capacity(&bf));
        bloom_free(&bf);
    }

    /* add() then contains() must be true. */
    {
        bloom_filter bf;
        ISO_CHECK_EQ_INT(bloom_init(&bf, 1000, 0.01), BLOOM_OK);
        add_str(&bf, "hello");
        ISO_CHECK(contains_str(&bf, "hello"));
        bloom_free(&bf);
    }

    /* No false negatives: every inserted item is later found. */
    {
        bloom_filter bf;
        char buf[32];
        int i;
        int found = 1;
        ISO_CHECK_EQ_INT(bloom_init(&bf, 1000, 0.01), BLOOM_OK);
        for (i = 0; i < 200; i++) {
            snprintf(buf, sizeof buf, "item-%d", i);
            add_str(&bf, buf);
        }
        for (i = 0; i < 200; i++) {
            snprintf(buf, sizeof buf, "item-%d", i);
            if (!contains_str(&bf, buf)) {
                found = 0;
            }
        }
        ISO_CHECK_MSG(found, "no inserted item may be missing");
        bloom_free(&bf);
    }

    /* Stats are consistent after an insertion. */
    {
        bloom_filter bf;
        ISO_CHECK_EQ_INT(bloom_init(&bf, 100, 0.01), BLOOM_OK);
        add_str(&bf, "alpha");
        ISO_CHECK(bloom_bit_count(&bf) > 0);
        ISO_CHECK(bloom_hash_count(&bf) >= 1);
        ISO_CHECK(bloom_bits_set(&bf) > 0);
        ISO_CHECK(bloom_size_bytes(&bf) > 0);
        ISO_CHECK(bloom_fill_ratio(&bf) > 0.0);
        ISO_CHECK(bloom_estimated_false_positive_rate(&bf) >= 0.0);
        bloom_free(&bf);
    }

    /* Sizing helpers match the reference expectations. */
    {
        size_t m = bloom_optimal_m(1000000, 0.01);
        size_t k = bloom_optimal_k(m, 1000000);
        ISO_CHECK(m > 9000000);
        ISO_CHECK_EQ_UINT(k, 7);
        ISO_CHECK(bloom_capacity_for_memory(1000000, 0.01) > 0);
    }

    /* Invalid parameters are rejected (no allocation performed). */
    {
        bloom_filter bf;
        ISO_CHECK_EQ_INT(bloom_init(&bf, 0, 0.01), BLOOM_INVALID_EXPECTED_ITEMS);
        ISO_CHECK_EQ_INT(bloom_init(&bf, 1, 0.0),
                         BLOOM_INVALID_FALSE_POSITIVE_RATE);
        ISO_CHECK_EQ_INT(bloom_init(&bf, 1, 1.0),
                         BLOOM_INVALID_FALSE_POSITIVE_RATE);
        ISO_CHECK_EQ_INT(bloom_init_params(&bf, 0, 1), BLOOM_INVALID_BIT_COUNT);
        ISO_CHECK_EQ_INT(bloom_init_params(&bf, 1, 0), BLOOM_INVALID_HASH_COUNT);
    }

    /* Explicit-parameter construction works and stores nothing initially. */
    {
        bloom_filter bf;
        ISO_CHECK_EQ_INT(bloom_init_params(&bf, 1024, 3), BLOOM_OK);
        ISO_CHECK_EQ_UINT(bloom_bit_count(&bf), 1024);
        ISO_CHECK_EQ_UINT(bloom_hash_count(&bf), 3);
        ISO_CHECK_EQ_UINT(bloom_size_bytes(&bf), 128);
        add_str(&bf, "x");
        ISO_CHECK(contains_str(&bf, "x"));
        ISO_CHECK(!contains_str(&bf, "definitely-not-present-zzz"));
        bloom_free(&bf);
    }

    /* Non-finite sizing inputs are handled gracefully (no hang, no UB). */
    {
        ISO_CHECK_EQ_UINT(bloom_optimal_m(1000, INFINITY), 0);
        ISO_CHECK_EQ_UINT(bloom_optimal_m(1000, NAN), 0);
        ISO_CHECK_EQ_UINT(bloom_capacity_for_memory(1024, INFINITY), 0);
        ISO_CHECK_EQ_UINT(bloom_capacity_for_memory(0, NAN), 0);
    }

    return ISO_TEST_RESULT();
}
