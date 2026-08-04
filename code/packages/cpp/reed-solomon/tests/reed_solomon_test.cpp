// Tests for the C++ Reed-Solomon codec, using the iso_test.h harness. Verifies
// the generator polynomial and end-to-end encode -> corrupt -> decode recovery.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "reed_solomon.hpp"

namespace rs = ca::reed_solomon;

int main() {
    // Generator for n_check = 2 is [8, 6, 1] (little-endian).
    {
        auto g = rs::build_generator(2);
        ISO_CHECK_EQ_UINT(g.size(), 3);
        ISO_CHECK(g[0] == 8 && g[1] == 6 && g[2] == 1);
    }
    // Odd n_check throws.
    {
        bool threw = false;
        try {
            rs::build_generator(3);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Round-trip with no errors.
    {
        std::vector<std::uint8_t> msg = {'H', 'E', 'L', 'L', 'O'};
        auto code = rs::encode(msg, 4);
        ISO_CHECK_EQ_UINT(code.size(), 9);
        ISO_CHECK(std::vector<std::uint8_t>(code.begin(), code.begin() + 5) == msg);
        auto dec = rs::decode(code, 4);
        ISO_CHECK(dec.has_value() && dec.value() == msg);
    }

    // Single-byte error correction.
    {
        std::vector<std::uint8_t> msg = {'H', 'E', 'L', 'L', 'O'};
        auto code = rs::encode(msg, 4);
        code[2] ^= 0x5A;
        auto dec = rs::decode(code, 4);
        ISO_CHECK(dec.has_value() && dec.value() == msg);
    }

    // Two-byte error correction (the maximum for t = 2).
    {
        std::vector<std::uint8_t> msg = {1, 2, 3, 4, 5, 6, 7, 8};
        auto code = rs::encode(msg, 4);
        code[1] ^= 0xFF;
        code[10] ^= 0x33;
        auto dec = rs::decode(code, 4);
        ISO_CHECK(dec.has_value() && dec.value() == msg);
    }

    // Up to t = 4 errors with n_check = 8.
    {
        std::vector<std::uint8_t> msg(16);
        for (int i = 0; i < 16; i++) {
            msg[static_cast<std::size_t>(i)] = static_cast<std::uint8_t>(i * 17 + 3);
        }
        auto code = rs::encode(msg, 8);
        ISO_CHECK_EQ_UINT(code.size(), 24);
        code[0] ^= 0x11;
        code[7] ^= 0x22;
        code[15] ^= 0x44;
        code[23] ^= 0x88;
        auto dec = rs::decode(code, 8);
        ISO_CHECK(dec.has_value() && dec.value() == msg);
    }

    // Too many errors (3 > t = 2) -> nullopt.
    {
        std::vector<std::uint8_t> msg = {1, 2, 3, 4, 5, 6, 7, 8};
        auto code = rs::encode(msg, 4);
        code[0] ^= 0x01;
        code[3] ^= 0x02;
        code[6] ^= 0x04;
        ISO_CHECK(!rs::decode(code, 4).has_value());
    }

    // Too-short codeword throws.
    {
        bool threw = false;
        try {
            rs::decode(std::vector<std::uint8_t>{0, 0}, 4);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Oversized parameters throw (parity with the C sibling's bounds).
    {
        bool threw = false;
        try {
            rs::build_generator(1000);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
        threw = false;
        try {
            rs::decode(std::vector<std::uint8_t>(256, 0), 4);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
