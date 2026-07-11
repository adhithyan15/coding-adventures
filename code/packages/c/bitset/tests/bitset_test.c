/* Tests for the C bitset, using the header-only iso_test.h harness. Covers
 * set/clear/toggle/test, auto-grow, popcount/any/all/none, the bitwise set
 * operations across differing sizes, and the integer/binary-string conversions. */
#include "iso_test.h"

#include "bitset.h"

int main(void) {
    bitset b, c, out;
    char buf[200];
    uint64_t v;

    /* set / test / clear over a small bitset. */
    ISO_CHECK(bitset_init(&b, 8));
    ISO_CHECK_EQ_UINT(bitset_len(&b), 8);
    ISO_CHECK(!bitset_any(&b));
    ISO_CHECK(bitset_none(&b));
    ISO_CHECK(bitset_set(&b, 1));
    ISO_CHECK(bitset_set(&b, 3));
    ISO_CHECK(bitset_set(&b, 5));
    ISO_CHECK(bitset_test(&b, 3));
    ISO_CHECK(!bitset_test(&b, 2));
    ISO_CHECK_EQ_UINT(bitset_popcount(&b), 3);
    ISO_CHECK(bitset_any(&b));
    bitset_clear(&b, 3);
    ISO_CHECK(!bitset_test(&b, 3));
    ISO_CHECK_EQ_UINT(bitset_popcount(&b), 2);
    /* toggle flips. */
    ISO_CHECK(bitset_toggle(&b, 1)); /* 1 → 0 */
    ISO_CHECK(!bitset_test(&b, 1));
    ISO_CHECK(bitset_toggle(&b, 7)); /* 0 → 1 */
    ISO_CHECK(bitset_test(&b, 7));

    /* Auto-grow: setting a bit past len grows the bitset. */
    ISO_CHECK(bitset_set(&b, 100));
    ISO_CHECK_EQ_UINT(bitset_len(&b), 101);
    ISO_CHECK(bitset_test(&b, 100));
    ISO_CHECK(bitset_capacity(&b) >= 101);
    bitset_free(&b);

    /* from_integer / to_integer round-trip. */
    ISO_CHECK(bitset_from_integer(&b, 0xAB, 0)); /* 10101011 */
    ISO_CHECK_EQ_UINT(bitset_len(&b), 64);
    ISO_CHECK(bitset_to_integer(&b, &v));
    ISO_CHECK_EQ_UINT(v, 0xAB);
    ISO_CHECK_EQ_UINT(bitset_popcount(&b), 5);
    bitset_free(&b);
    /* Zero → empty. */
    ISO_CHECK(bitset_from_integer(&b, 0, 0));
    ISO_CHECK(bitset_is_empty(&b));
    ISO_CHECK(bitset_to_integer(&b, &v));
    ISO_CHECK_EQ_UINT(v, 0);
    bitset_free(&b);
    /* A high bit set → doesn't fit in 64 bits. */
    ISO_CHECK(bitset_from_integer(&b, 1, 1)); /* bit 0 and bit 64 */
    ISO_CHECK_EQ_UINT(bitset_len(&b), 128);
    ISO_CHECK(!bitset_to_integer(&b, &v));
    bitset_free(&b);

    /* from_binary_str / to_binary_str: MSB-first, bit 0 is rightmost. */
    ISO_CHECK(bitset_from_binary_str(&b, "101") == 1); /* value 5 */
    ISO_CHECK(bitset_test(&b, 0));
    ISO_CHECK(!bitset_test(&b, 1));
    ISO_CHECK(bitset_test(&b, 2));
    ISO_CHECK(bitset_to_binary_str(&b, buf, sizeof buf) == 3);
    ISO_CHECK_STR_EQ(buf, "101");
    ISO_CHECK(bitset_to_integer(&b, &v));
    ISO_CHECK_EQ_UINT(v, 5);
    bitset_free(&b);
    /* Invalid character rejected. */
    ISO_CHECK(bitset_from_binary_str(&b, "10201") == -1);

    /* all() is true only when every logical bit is set. */
    ISO_CHECK(bitset_from_binary_str(&b, "1111") == 1);
    ISO_CHECK(bitset_all(&b));
    bitset_clear(&b, 2);
    ISO_CHECK(!bitset_all(&b));
    bitset_free(&b);

    /* Bitwise operations across differing sizes. a = 1100 (bits 2,3),
     * c = 1010 (bits 1,3). */
    ISO_CHECK(bitset_from_binary_str(&b, "1100") == 1);
    ISO_CHECK(bitset_from_binary_str(&c, "1010") == 1);

    ISO_CHECK(bitset_and(&b, &c, &out)); /* 1000 → bit 3 only */
    ISO_CHECK(bitset_to_binary_str(&out, buf, sizeof buf) >= 0);
    ISO_CHECK_STR_EQ(buf, "1000");
    bitset_free(&out);

    ISO_CHECK(bitset_or(&b, &c, &out)); /* 1110 */
    ISO_CHECK(bitset_to_binary_str(&out, buf, sizeof buf) >= 0);
    ISO_CHECK_STR_EQ(buf, "1110");
    bitset_free(&out);

    ISO_CHECK(bitset_xor(&b, &c, &out)); /* 0110 */
    ISO_CHECK(bitset_to_binary_str(&out, buf, sizeof buf) >= 0);
    ISO_CHECK_STR_EQ(buf, "0110");
    bitset_free(&out);

    ISO_CHECK(bitset_and_not(&b, &c, &out)); /* 1100 & ~1010 = 0100 */
    ISO_CHECK(bitset_to_binary_str(&out, buf, sizeof buf) >= 0);
    ISO_CHECK_STR_EQ(buf, "0100");
    bitset_free(&out);

    ISO_CHECK(bitset_not(&b, &out)); /* ~1100 within 4 bits = 0011 */
    ISO_CHECK(bitset_to_binary_str(&out, buf, sizeof buf) >= 0);
    ISO_CHECK_STR_EQ(buf, "0011");
    bitset_free(&out);

    bitset_free(&b);
    bitset_free(&c);

    return ISO_TEST_RESULT();
}
