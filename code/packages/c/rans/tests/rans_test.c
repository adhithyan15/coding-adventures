/* Tests for the C rans, using the iso_test.h harness. Table vectors and round
 * trips are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */

#include "rans.h"

/* Encode `syms` (length k) in reverse, then decode forward and compare. */
static void round_trip(const AnsTable *t, const unsigned char *syms, size_t k) {
    RansEncoder e;
    unsigned char *bytes = NULL;
    size_t blen = 0, i;
    RansDecoder d;

    rans_encoder_init(&e, t);
    for (i = k; i > 0; i--) {
        rans_encoder_put(&e, syms[i - 1]);
    }
    ISO_CHECK_EQ_INT((int)rans_encoder_finish(&e, &bytes, &blen), (int)RANS_OK);
    ISO_CHECK(blen >= 8);
    rans_decoder_init(&d, t, bytes, blen);
    for (i = 0; i < k; i++) {
        unsigned char got = rans_decoder_get(&d);
        ISO_CHECK(got == syms[i]);
    }
    free(bytes);
}

int main(void) {
    /* Table [1,1]: M=2, freq 1/1, cumfreq 0/1. */
    {
        unsigned int counts[] = {1, 1};
        AnsTable t;
        unsigned int v;
        ISO_CHECK_EQ_INT((int)ans_table_new(counts, 2, &t), (int)RANS_OK);
        ISO_CHECK_EQ_UINT(ans_table_m(&t), 2u);
        ISO_CHECK_EQ_UINT(ans_table_log2m(&t), 1u);
        ISO_CHECK_EQ_UINT(ans_table_alphabet_size(&t), 2u);
        ISO_CHECK(ans_table_freq(&t, 0, &v) && v == 1);
        ISO_CHECK(ans_table_freq(&t, 1, &v) && v == 1);
        ISO_CHECK(ans_table_cumfreq(&t, 0, &v) && v == 0);
        ISO_CHECK(ans_table_cumfreq(&t, 1, &v) && v == 1);
        ISO_CHECK(!ans_table_freq(&t, 2, &v));    /* out of range -> None */
        ISO_CHECK(!ans_table_cumfreq(&t, 2, &v));
        ans_table_free(&t);
    }

    /* Table [3,1]: M=4, freq 3/1. */
    {
        unsigned int counts[] = {3, 1};
        AnsTable t;
        unsigned int v;
        ans_table_new(counts, 2, &t);
        ISO_CHECK_EQ_UINT(ans_table_m(&t), 4u);
        ISO_CHECK_EQ_UINT(ans_table_log2m(&t), 2u);
        ISO_CHECK(ans_table_freq(&t, 0, &v) && v == 3);
        ISO_CHECK(ans_table_freq(&t, 1, &v) && v == 1);
        ans_table_free(&t);
    }

    /* Frequencies sum to M; M is a power of two. */
    {
        unsigned int counts[] = {10, 5, 3};
        AnsTable t;
        unsigned int sum = 0, i, v, m;
        ans_table_new(counts, 3, &t);
        ISO_CHECK_EQ_UINT(ans_table_alphabet_size(&t), 3u);
        m = ans_table_m(&t);
        ISO_CHECK((m & (m - 1)) == 0); /* power of two */
        for (i = 0; i < 3; i++) {
            ans_table_freq(&t, i, &v);
            sum += v;
        }
        ISO_CHECK_EQ_UINT(sum, m);
        ans_table_free(&t);
    }

    /* log2m for M = 8 and 16. */
    {
        unsigned int c8[] = {5, 3};  /* total 8 -> M=8 */
        unsigned int c16[] = {10, 6}; /* total 16 -> M=16 */
        AnsTable t;
        ans_table_new(c8, 2, &t);
        ISO_CHECK_EQ_UINT(ans_table_m(&t), 8u);
        ISO_CHECK_EQ_UINT(ans_table_log2m(&t), 3u);
        ans_table_free(&t);
        ans_table_new(c16, 2, &t);
        ISO_CHECK_EQ_UINT(ans_table_m(&t), 16u);
        ISO_CHECK_EQ_UINT(ans_table_log2m(&t), 4u);
        ans_table_free(&t);
    }

    /* Error cases. */
    {
        AnsTable t;
        unsigned int zero[] = {0, 0, 0};
        unsigned int big[257];
        size_t i;
        ISO_CHECK_EQ_INT((int)ans_table_new(NULL, 0, &t), (int)RANS_ERR_EMPTY);
        ISO_CHECK_EQ_INT((int)ans_table_new(zero, 3, &t),
                         (int)RANS_ERR_ALL_ZERO);
        for (i = 0; i < 257; i++) {
            big[i] = 1;
        }
        ISO_CHECK_EQ_INT((int)ans_table_new(big, 257, &t),
                         (int)RANS_ERR_ALPHABET_TOO_LARGE);
    }

    /* Decoder rejects short data. */
    {
        unsigned int counts[] = {1, 1};
        AnsTable t;
        RansDecoder d;
        unsigned char seven[] = {0, 0, 0, 0, 0, 0, 0};
        ans_table_new(counts, 2, &t);
        ISO_CHECK_EQ_INT((int)rans_decoder_init(&d, &t, seven, 7),
                         (int)RANS_ERR_SHORT_DATA);
        ISO_CHECK_EQ_INT((int)rans_decoder_init(&d, &t, NULL, 0),
                         (int)RANS_ERR_SHORT_DATA);
        ans_table_free(&t);
    }

    /* Round trips. */
    {
        unsigned int counts[] = {3, 1};
        AnsTable t;
        unsigned char seq1[] = {0, 0, 1, 0};
        unsigned char seq2[] = {0};
        unsigned char seq3[] = {1, 0, 1, 1, 0, 0, 1, 0};
        ans_table_new(counts, 2, &t);
        round_trip(&t, seq1, 4);
        round_trip(&t, seq2, 1);
        round_trip(&t, seq3, 8);
        ans_table_free(&t);
    }
    {
        unsigned int counts[] = {120, 8};
        AnsTable t;
        unsigned char skewed[16];
        size_t i;
        ans_table_new(counts, 2, &t);
        for (i = 0; i < 16; i++) {
            skewed[i] = (unsigned char)(i == 7 ? 1 : 0); /* mostly symbol 0 */
        }
        round_trip(&t, skewed, 16);
        ans_table_free(&t);
    }
    {
        unsigned int counts[] = {5, 3, 2};
        AnsTable t;
        unsigned char seq[] = {0, 1, 2, 0, 1, 0, 2, 1, 0, 0};
        ans_table_new(counts, 3, &t);
        round_trip(&t, seq, 10);
        ans_table_free(&t);
    }

    /* Malformed input (all-zero state) must not hang the decoder. Reaching the
     * assertion after several get() calls proves the renorm loop terminated. */
    {
        unsigned int counts[] = {1, 1};
        AnsTable t;
        RansDecoder d;
        unsigned char zeros[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        size_t i;
        ans_table_new(counts, 2, &t);
        ISO_CHECK_EQ_INT((int)rans_decoder_init(&d, &t, zeros, 8), (int)RANS_OK);
        for (i = 0; i < 100; i++) {
            (void)rans_decoder_get(&d); /* must not spin forever */
        }
        ISO_CHECK(1);
        ans_table_free(&t);
    }

    /* Deterministic encoding. */
    {
        unsigned int counts[] = {1, 1};
        AnsTable t;
        RansEncoder e1, e2;
        unsigned char *b1 = NULL, *b2 = NULL;
        size_t l1 = 0, l2 = 0, i;
        unsigned char seq[] = {0, 1, 1, 0};
        ans_table_new(counts, 2, &t);
        rans_encoder_init(&e1, &t);
        rans_encoder_init(&e2, &t);
        for (i = 4; i > 0; i--) {
            rans_encoder_put(&e1, seq[i - 1]);
            rans_encoder_put(&e2, seq[i - 1]);
        }
        rans_encoder_finish(&e1, &b1, &l1);
        rans_encoder_finish(&e2, &b2, &l2);
        ISO_CHECK_EQ_UINT(l1, l2);
        {
            int same = (l1 == l2);
            for (i = 0; same && i < l1; i++) {
                if (b1[i] != b2[i]) {
                    same = 0;
                }
            }
            ISO_CHECK(same);
        }
        free(b1);
        free(b2);
        ans_table_free(&t);
    }

    return ISO_TEST_RESULT();
}
