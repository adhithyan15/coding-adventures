// Tests for the C++ bitset, using the header-only iso_test.h harness. Covers
// set/clear/toggle/test, auto-grow, popcount/any/all/none, the bitwise operators
// across differing sizes, and integer/binary-string conversions.
#include "iso_test.h"

#include <stdexcept>
#include <string>

#include "bitset.hpp"

template <typename Ex, typename F> static bool throws(F body) {
    try {
        body();
    } catch (const Ex &) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

int main() {
    ca::bitset b(8);
    ISO_CHECK_EQ_UINT(b.size(), 8);
    ISO_CHECK(!b.any());
    ISO_CHECK(b.none());
    b.set(1);
    b.set(3);
    b.set(5);
    ISO_CHECK(b.test(3));
    ISO_CHECK(!b.test(2));
    ISO_CHECK_EQ_UINT(b.popcount(), 3);
    b.clear(3);
    ISO_CHECK(!b.test(3));
    ISO_CHECK_EQ_UINT(b.popcount(), 2);
    b.toggle(1); // 1 → 0
    ISO_CHECK(!b.test(1));
    b.toggle(7); // 0 → 1
    ISO_CHECK(b.test(7));

    // Auto-grow.
    b.set(100);
    ISO_CHECK_EQ_UINT(b.size(), 101);
    ISO_CHECK(b.test(100));
    ISO_CHECK(b.capacity() >= 101);

    // from_integer / to_integer.
    ca::bitset i = ca::bitset::from_integer(0xAB);
    ISO_CHECK_EQ_UINT(i.size(), 64);
    ISO_CHECK(i.to_integer().has_value());
    ISO_CHECK_EQ_UINT(i.to_integer().value(), 0xAB);
    ISO_CHECK_EQ_UINT(i.popcount(), 5);
    ISO_CHECK(ca::bitset::from_integer(0, 0).empty());
    ISO_CHECK(!ca::bitset::from_integer(1, 1).to_integer().has_value()); // bit 64 set

    // from_binary_string / to_binary_string: MSB-first, bit 0 rightmost.
    ca::bitset s = ca::bitset::from_binary_string("101"); // value 5
    ISO_CHECK(s.test(0));
    ISO_CHECK(!s.test(1));
    ISO_CHECK(s.test(2));
    ISO_CHECK_STR_EQ(s.to_binary_string().c_str(), "101");
    ISO_CHECK_EQ_UINT(s.to_integer().value(), 5);
    ISO_CHECK(throws<std::invalid_argument>(
        [] { (void)ca::bitset::from_binary_string("10201"); }));

    // all().
    ca::bitset ones = ca::bitset::from_binary_string("1111");
    ISO_CHECK(ones.all());
    ones.clear(2);
    ISO_CHECK(!ones.all());

    // Bitwise operators across sizes. x = 1100 (bits 2,3), y = 1010 (bits 1,3).
    ca::bitset x = ca::bitset::from_binary_string("1100");
    ca::bitset y = ca::bitset::from_binary_string("1010");
    ISO_CHECK_STR_EQ((x & y).to_binary_string().c_str(), "1000");
    ISO_CHECK_STR_EQ((x | y).to_binary_string().c_str(), "1110");
    ISO_CHECK_STR_EQ((x ^ y).to_binary_string().c_str(), "0110");
    ISO_CHECK_STR_EQ(x.and_not(y).to_binary_string().c_str(), "0100");
    ISO_CHECK_STR_EQ((~x).to_binary_string().c_str(), "0011");

    return ISO_TEST_RESULT();
}
