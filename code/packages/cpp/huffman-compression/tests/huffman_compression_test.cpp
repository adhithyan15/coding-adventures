// Tests for the C++ Huffman compression, using the iso_test.h harness.
// Compress/decompress round-trips over varied distributions, plus edge cases.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "huffman_compression.hpp"

static std::vector<std::uint8_t> bytes(const std::string &s) {
    return std::vector<std::uint8_t>(s.begin(), s.end());
}

static void round_trip(const std::vector<std::uint8_t> &input) {
    auto packed = ca::huffman::compress(input);
    auto restored = ca::huffman::decompress(packed);
    ISO_CHECK(restored == input);
}

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
    // Empty → 8-byte header.
    {
        auto packed = ca::huffman::compress({});
        ISO_CHECK_EQ_UINT(packed.size(), 8);
        ISO_CHECK(ca::huffman::decompress(packed).empty());
    }

    round_trip(std::vector<std::uint8_t>(50, 'X'));           // single symbol
    round_trip(bytes("aaaaaaaaaaaabbbbbbcccdde"));            // skewed
    round_trip(bytes("the quick brown fox jumps over the lazy dog"));
    round_trip(bytes("ababababab"));                          // two symbols

    // Full byte range, uneven counts.
    {
        std::vector<std::uint8_t> buf(2000);
        for (std::size_t i = 0; i < buf.size(); i++) {
            buf[i] = static_cast<std::uint8_t>((i * 7 + i / 3) & 0xff);
        }
        round_trip(buf);
    }

    // Header carries the original length.
    {
        auto packed = ca::huffman::compress(bytes("hello world"));
        std::size_t recorded = (static_cast<std::size_t>(packed[0]) << 24) |
                               (static_cast<std::size_t>(packed[1]) << 16) |
                               (static_cast<std::size_t>(packed[2]) << 8) | packed[3];
        ISO_CHECK_EQ_UINT(recorded, 11);
    }

    // Malformed input throws.
    ISO_CHECK(throws<std::invalid_argument>(
        [] { (void)ca::huffman::decompress(std::vector<std::uint8_t>{1, 2, 3}); }));

    return ISO_TEST_RESULT();
}
