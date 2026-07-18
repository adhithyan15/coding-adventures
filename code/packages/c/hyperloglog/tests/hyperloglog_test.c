/*
 * hyperloglog_test.c — tests for the approximate cardinality estimator.
 * ===========================================================================
 *
 * Mirrors the Rust unit tests (empty→0, duplicates stay tiny, ~1000 distinct
 * items estimate within tolerance, merge unions registers, precision mismatch
 * errors, the static helpers, invalid precision rejected) and adds the C
 * ownership / NULL-argument paths. Runs under ASan+UBSan so any leak or
 * out-of-bounds register access fails the build. The estimator is randomised
 * only by the hash, so the tolerance bands match the Rust ones.
 */
#include "hyperloglog/hyperloglog.h"
#include "iso_test.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h>
#include <string.h>

/* Rust: empty_count_is_zero. */
static void test_empty_count_is_zero(void) {
    hll *h = NULL;
    ISO_CHECK_EQ_INT(hll_create_default(&h), HLL_OK);
    ISO_CHECK_EQ_UINT(hll_count(h), 0);
    ISO_CHECK_EQ_INT(hll_is_empty(h), 1);
    ISO_CHECK_EQ_INT(hll_precision(h), HLL_DEFAULT_PRECISION);
    ISO_CHECK_EQ_UINT(hll_num_registers(h), (size_t)1 << HLL_DEFAULT_PRECISION);
    hll_destroy(h);
}

/* Rust: duplicates_do_not_change_the_estimate_much. */
static void test_duplicates_stay_tiny(void) {
    hll *h = NULL;
    int i;
    ISO_CHECK_EQ_INT(hll_create_default(&h), HLL_OK);
    for (i = 0; i < 1000; i++) {
        hll_add_str(h, "same");
    }
    ISO_CHECK_MSG(hll_count(h) < 10, "1000 duplicates must estimate < 10 distinct");
    ISO_CHECK_EQ_INT(hll_is_empty(h), 0);
    hll_destroy(h);
}

/* Rust: small_accuracy_is_reasonable. */
static void test_small_accuracy(void) {
    hll *h = NULL;
    int i;
    size_t est;
    ISO_CHECK_EQ_INT(hll_create_default(&h), HLL_OK);
    for (i = 0; i < 1000; i++) {
        char buf[32];
        snprintf(buf, sizeof(buf), "item-%d", i);
        hll_add_str(h, buf);
    }
    est = hll_count(h);
    ISO_CHECK_MSG(est >= 900 && est <= 1100, "1000 distinct items estimate in [900,1100]");
    hll_destroy(h);
}

/* Rust: merge_combines_registers. */
static void test_merge_combines(void) {
    hll *left = NULL, *right = NULL, *merged = NULL;
    int i;
    size_t lc, rc, mc;
    ISO_CHECK_EQ_INT(hll_create(10, &left), HLL_OK);
    ISO_CHECK_EQ_INT(hll_create(10, &right), HLL_OK);
    for (i = 0; i < 200; i++) {
        char buf[32];
        snprintf(buf, sizeof(buf), "%d", i);
        hll_add_str(left, buf);
        snprintf(buf, sizeof(buf), "%d", i + 1000);
        hll_add_str(right, buf);
    }
    ISO_CHECK_EQ_INT(hll_merge(left, right, &merged), HLL_OK);
    lc = hll_count(left);
    rc = hll_count(right);
    mc = hll_count(merged);
    ISO_CHECK_MSG(mc >= lc, "merged estimate >= left");
    ISO_CHECK_MSG(mc >= rc, "merged estimate >= right");
    hll_destroy(left);
    hll_destroy(right);
    hll_destroy(merged);
}

/* Rust: merge_precision_mismatch_errors. */
static void test_merge_precision_mismatch(void) {
    hll *left = NULL, *right = NULL, *merged = NULL;
    ISO_CHECK_EQ_INT(hll_create(10, &left), HLL_OK);
    ISO_CHECK_EQ_INT(hll_create(14, &right), HLL_OK);
    ISO_CHECK_EQ_INT(hll_merge(left, right, &merged), HLL_ERR_PRECISION_MISMATCH);
    ISO_CHECK(merged == NULL); /* no sketch produced on error */
    hll_destroy(left);
    hll_destroy(right);
}

/* Rust: helper_functions_work (the public subset). */
static void test_helper_functions(void) {
    double er;
    ISO_CHECK_EQ_UINT(hll_memory_bytes(14), 12288);
    ISO_CHECK_EQ_INT(hll_optimal_precision(0.01), 14);
    er = hll_error_rate_for_precision(14);
    ISO_CHECK_EQ_DBL(er, 0.00812, 0.001);

    /* Bounds behaviour of optimal_precision: clamps into [4,16]. */
    ISO_CHECK(hll_optimal_precision(1.0) == HLL_MIN_PRECISION);   /* huge error → min */
    ISO_CHECK(hll_optimal_precision(1e-9) == HLL_MAX_PRECISION);  /* tiny error → max */
    ISO_CHECK(hll_optimal_precision(0.0) == HLL_MAX_PRECISION);   /* non-positive → max */
    ISO_CHECK(hll_optimal_precision(-1.0) == HLL_MAX_PRECISION);
}

/* Rust: invalid_precision_is_rejected. */
static void test_invalid_precision(void) {
    hll *h = NULL;
    ISO_CHECK_EQ_INT(hll_create(3, &h), HLL_ERR_INVALID_PRECISION);
    ISO_CHECK(h == NULL);
    ISO_CHECK_EQ_INT(hll_create(17, &h), HLL_ERR_INVALID_PRECISION);
    ISO_CHECK(h == NULL);
    /* The exact bounds are valid. */
    ISO_CHECK_EQ_INT(hll_create(HLL_MIN_PRECISION, &h), HLL_OK);
    hll_destroy(h);
    ISO_CHECK_EQ_INT(hll_create(HLL_MAX_PRECISION, &h), HLL_OK);
    hll_destroy(h);
}

/* Empty and binary payloads are valid observations. */
static void test_add_edge_inputs(void) {
    hll *h = NULL;
    unsigned char bin[4];
    bin[0] = 0x00; bin[1] = 0xFF; bin[2] = 0x00; bin[3] = 0x7F;
    ISO_CHECK_EQ_INT(hll_create(12, &h), HLL_OK);
    ISO_CHECK_EQ_INT(hll_add_bytes(h, NULL, 0), HLL_OK);   /* empty input hashes fine */
    ISO_CHECK_EQ_INT(hll_add_bytes(h, bin, 4), HLL_OK);    /* embedded NUL ok */
    ISO_CHECK(hll_count(h) >= 1);
    hll_destroy(h);
}

static void test_invalid_params(void) {
    hll *h = NULL;
    unsigned char b = 'x';
    ISO_CHECK_EQ_INT(hll_create(10, NULL), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_create_default(NULL), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_create(10, &h), HLL_OK);
    ISO_CHECK_EQ_INT(hll_add_bytes(NULL, &b, 1), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_add_bytes(h, NULL, 5), HLL_ERR_INVALID); /* len>0 needs bytes */
    ISO_CHECK_EQ_INT(hll_add_str(h, NULL), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_add_str(NULL, "x"), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_merge(NULL, h, &h), HLL_ERR_INVALID);
    ISO_CHECK_EQ_INT(hll_merge(h, NULL, &h), HLL_ERR_INVALID);
    /* NULL-tolerant read paths. */
    ISO_CHECK_EQ_UINT(hll_count(NULL), 0);
    ISO_CHECK_EQ_INT(hll_is_empty(NULL), 1);
    ISO_CHECK_EQ_INT(hll_precision(NULL), 0);
    ISO_CHECK_EQ_UINT(hll_num_registers(NULL), 0);
    ISO_CHECK(hll_error_rate(NULL) == 0.0);
    hll_destroy(NULL);
    hll_destroy(h);
}

int main(void) {
    test_empty_count_is_zero();
    test_duplicates_stay_tiny();
    test_small_accuracy();
    test_merge_combines();
    test_merge_precision_mismatch();
    test_helper_functions();
    test_invalid_precision();
    test_add_edge_inputs();
    test_invalid_params();
    return ISO_TEST_RESULT();
}
