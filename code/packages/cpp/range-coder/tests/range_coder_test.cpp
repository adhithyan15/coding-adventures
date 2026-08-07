// Tests for the C++ range-coder, using the iso_test.h harness. Round trips and
// bit-field vectors are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <utility>
#include <vector>

#include "range_coder.hpp"

namespace rc = ca::range_coder;

static void round_trip(const std::vector<std::pair<bool, std::uint8_t>>& bits) {
    rc::BoolEncoder enc;
    for (auto& bp : bits) {
        enc.write_bit(bp.first, bp.second);
    }
    std::vector<std::uint8_t> bytes = enc.finish();
    rc::BoolDecoder dec(bytes);
    for (auto& bp : bits) {
        ISO_CHECK(dec.read_bit(bp.second) == bp.first);
    }
}

int main() {
    // finish() on a fresh encoder is non-empty.
    {
        rc::BoolEncoder enc;
        ISO_CHECK(!enc.finish().empty());
    }

    // Single-bit round trips.
    round_trip({{true, 128}});
    round_trip({{false, 128}});

    // Mixed sequence.
    round_trip({{true, 128}, {false, 200}, {true, 64}, {false, 128}});

    // Long sequence across many probabilities.
    {
        std::vector<std::pair<bool, std::uint8_t>> bits;
        for (std::size_t i = 0; i < 32; ++i) {
            bits.push_back({((i * 7 + 3) % 2) != 0,
                            static_cast<std::uint8_t>(1 + (i * 8) % 255)});
        }
        round_trip(bits);
    }

    // Skewed probabilities.
    round_trip({{false, 250}, {false, 250}, {false, 250},
                {true, 5}, {false, 250}, {false, 250}});

    // write_bits / read_bits for u8, u16, u32.
    {
        rc::BoolEncoder enc;
        enc.write_bits(0xAB, 8);
        rc::BoolDecoder dec(enc.finish());
        ISO_CHECK_EQ_UINT(dec.read_bits(8), 0xABu);
    }
    {
        rc::BoolEncoder enc;
        enc.write_bits(0xDEAD, 16);
        rc::BoolDecoder dec(enc.finish());
        ISO_CHECK_EQ_UINT(dec.read_bits(16), 0xDEADu);
    }
    {
        rc::BoolEncoder enc;
        enc.write_bits(0xCAFEBABEu, 32);
        rc::BoolDecoder dec(enc.finish());
        ISO_CHECK_EQ_UINT(dec.read_bits(32), 0xCAFEBABEu);
    }

    // write_bits(_, 0) writes nothing; read_bits(0) returns 0.
    {
        rc::BoolEncoder enc;
        enc.write_bits(0xFF, 0);
        rc::BoolDecoder dec(enc.finish());
        ISO_CHECK_EQ_UINT(dec.read_bits(0), 0u);
    }

    // Deterministic output.
    {
        auto encode = []() {
            rc::BoolEncoder e;
            e.write_bit(true, 128);
            e.write_bit(false, 200);
            e.write_bit(true, 64);
            e.write_bit(false, 128);
            return e.finish();
        };
        ISO_CHECK(encode() == encode());
    }

    // Decoder seeding and exhaustion.
    {
        std::vector<std::uint8_t> seed = {0xAB, 0xCD};
        rc::BoolDecoder d(seed);
        ISO_CHECK(d.is_exhausted());
        std::vector<std::uint8_t> three = {0x00, 0x00, 0xFF};
        rc::BoolDecoder d3(three);
        ISO_CHECK(!d3.is_exhausted());
        rc::BoolDecoder d0(nullptr, 0); // empty must not crash
        (void)d0;
        ISO_CHECK(true);
    }

    return ISO_TEST_RESULT();
}
