// Tests for the C++ wasm-leb128, using the iso_test.h harness. Vectors are
// taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "wasm_leb128.hpp"

namespace leb = ca::leb128;
using Bytes = std::vector<std::uint8_t>;

int main() {
    // ---- unsigned decoding -------------------------------------------
    {
        ISO_CHECK(leb::decode_unsigned(Bytes{0x00}, 0) ==
                  std::make_pair(std::uint64_t(0), std::size_t(1)));
        ISO_CHECK(leb::decode_unsigned(Bytes{0x03}, 0) ==
                  std::make_pair(std::uint64_t(3), std::size_t(1)));
        ISO_CHECK(leb::decode_unsigned(Bytes{0xE5, 0x8E, 0x26}, 0) ==
                  std::make_pair(std::uint64_t(624485), std::size_t(3)));
        ISO_CHECK(leb::decode_unsigned(Bytes{0xFF, 0xFF, 0xFF, 0xFF, 0x0F}, 0) ==
                  std::make_pair(std::uint64_t(4294967295u), std::size_t(5)));
        ISO_CHECK(leb::decode_unsigned(Bytes{0x00, 0x00, 0xE5, 0x8E, 0x26}, 2) ==
                  std::make_pair(std::uint64_t(624485), std::size_t(3)));
    }

    // unsigned decode errors (throw)
    {
        bool threw = false;
        try {
            leb::decode_unsigned(Bytes{0x80, 0x80}, 0);
        } catch (const leb::Error& e) {
            threw = true;
            ISO_CHECK_EQ_UINT(e.offset, 0u);
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            leb::decode_unsigned(Bytes{0x01}, 5);
        } catch (const leb::Error& e) {
            threw = true;
            ISO_CHECK_EQ_UINT(e.offset, 5u);
        }
        ISO_CHECK(threw);
    }

    // ---- signed decoding ---------------------------------------------
    {
        ISO_CHECK(leb::decode_signed(Bytes{0x00}, 0) ==
                  std::make_pair(std::int64_t(0), std::size_t(1)));
        ISO_CHECK(leb::decode_signed(Bytes{0x7E}, 0) ==
                  std::make_pair(std::int64_t(-2), std::size_t(1)));
        ISO_CHECK(leb::decode_signed(Bytes{0xFF, 0xFF, 0xFF, 0xFF, 0x07}, 0) ==
                  std::make_pair(std::int64_t(2147483647), std::size_t(5)));
        ISO_CHECK(leb::decode_signed(Bytes{0x80, 0x80, 0x80, 0x80, 0x78}, 0) ==
                  std::make_pair(std::int64_t(-2147483648LL), std::size_t(5)));
        ISO_CHECK(leb::decode_signed(Bytes{0x00, 0x00, 0x00, 0x7E}, 3) ==
                  std::make_pair(std::int64_t(-2), std::size_t(1)));
    }

    // signed decode error
    {
        bool threw = false;
        try {
            leb::decode_signed(Bytes{0x80, 0x80}, 0);
        } catch (const leb::Error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ---- encoding ----------------------------------------------------
    {
        ISO_CHECK((leb::encode_unsigned(0) == Bytes{0x00}));
        ISO_CHECK((leb::encode_unsigned(3) == Bytes{0x03}));
        ISO_CHECK((leb::encode_unsigned(624485) == Bytes{0xE5, 0x8E, 0x26}));
        ISO_CHECK((leb::encode_unsigned(4294967295u) ==
                   Bytes{0xFF, 0xFF, 0xFF, 0xFF, 0x0F}));
        ISO_CHECK((leb::encode_signed(0) == Bytes{0x00}));
        ISO_CHECK((leb::encode_signed(-2) == Bytes{0x7E}));
        ISO_CHECK((leb::encode_signed(-2147483648LL) ==
                   Bytes{0x80, 0x80, 0x80, 0x80, 0x78}));
        ISO_CHECK((leb::encode_signed(2147483647) ==
                   Bytes{0xFF, 0xFF, 0xFF, 0xFF, 0x07}));
    }

    // ---- round trips -------------------------------------------------
    {
        std::uint64_t uvals[] = {0u,   1u,          127u,
                                 128u, 255u,        624485u,
                                 4294967295u, 0xFFFFFFFFFFFFFFFFull};
        for (std::uint64_t v : uvals) {
            Bytes enc = leb::encode_unsigned(v);
            auto dec = leb::decode_unsigned(enc, 0);
            ISO_CHECK(dec.first == v);
            ISO_CHECK_EQ_UINT(dec.second, enc.size());
        }
        std::int64_t svals[] = {0,   1,           -1,        -2,
                                63,  -64,         127,       -128,
                                2147483647LL, -2147483648LL,
                                9223372036854775807LL,
                                (-9223372036854775807LL - 1)};
        for (std::int64_t v : svals) {
            Bytes enc = leb::encode_signed(v);
            auto dec = leb::decode_signed(enc, 0);
            ISO_CHECK(dec.first == v);
            ISO_CHECK_EQ_UINT(dec.second, enc.size());
        }
    }

    // An overlong sequence overflows the 70-bit limit.
    {
        Bytes overlong(10, 0x80);
        overlong.push_back(0x00);
        bool threw = false;
        try {
            leb::decode_unsigned(overlong, 0);
        } catch (const leb::Error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
