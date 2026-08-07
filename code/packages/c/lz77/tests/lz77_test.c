/* Tests for the C LZ77, using the iso_test.h harness. Covers token structure
 * (literals, a backreference) and compress/decompress round-trips. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "lz77.h"

/* Compress then decompress `input`, asserting the result equals the original. */
static void round_trip(const char *input, size_t len) {
    uint8_t *packed = NULL;
    uint8_t *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    ISO_CHECK(lz77_compress((const uint8_t *)input, len, LZ77_DEFAULT_WINDOW,
                            LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH,
                            &packed, &packed_len));
    ISO_CHECK(lz77_decompress(packed, packed_len, &restored, &restored_len));
    ISO_CHECK_EQ_UINT(restored_len, len);
    if (restored_len == len && len > 0) {
        ISO_CHECK_MEM_EQ(restored, input, len);
    }
    free(packed);
    free(restored);
}

int main(void) {
    lz77_token *tokens = NULL;
    size_t count = 0;

    /* Empty input → no tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"", 0, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 0);
    free(tokens);
    tokens = NULL;

    /* "ABCDE" has no repeats → five literal tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"ABCDE", 5, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 5);
    {
        size_t i;
        for (i = 0; i < count; i++) {
            ISO_CHECK_EQ_UINT(tokens[i].offset, 0);
            ISO_CHECK_EQ_UINT(tokens[i].length, 0);
        }
    }
    ISO_CHECK_EQ_INT(tokens[0].next_char, 'A');
    ISO_CHECK_EQ_INT(tokens[4].next_char, 'E');
    free(tokens);
    tokens = NULL;

    /* "AAAAAAA" (7 bytes) → a literal A, then a backreference (offset 1, length
     * 5, next_char A). The backreference covers bytes 1..6 (length 5 + the
     * next_char), so the cursor reaches the end — just two tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"AAAAAAA", 7, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 2);
    ISO_CHECK_EQ_UINT(tokens[0].length, 0); /* literal */
    ISO_CHECK_EQ_UINT(tokens[1].offset, 1);
    ISO_CHECK_EQ_UINT(tokens[1].length, 5);
    ISO_CHECK_EQ_INT(tokens[1].next_char, 'A');
    free(tokens);
    tokens = NULL;

    /* Round-trips of assorted inputs. */
    round_trip("", 0);
    round_trip("A", 1);
    round_trip("ABCDE", 5);
    round_trip("AAAAAAA", 7);
    round_trip("abcabcabcabcabc", 15);
    round_trip("the quick brown fox jumps over the lazy dog. the quick brown fox.",
               64);

    return ISO_TEST_RESULT();
}
