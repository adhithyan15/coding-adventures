// Tests for the C++ Huffman compression, using the iso_test.h harness.
// Compress/decompress round-trips over varied distributions, plus edge cases.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
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

// Build a CMP04 stream from explicit (symbol, length) pairs for hostile-input tests.
static std::vector<std::uint8_t>
hostile(const std::vector<std::pair<int, int>> &entries, std::uint32_t original,
        std::size_t payload_len, std::uint8_t payload_byte) {
    std::vector<std::uint8_t> out;
    for (int s = 24; s >= 0; s -= 8) {
        out.push_back(static_cast<std::uint8_t>(original >> s));
    }
    const std::uint32_t n = static_cast<std::uint32_t>(entries.size());
    for (int s = 24; s >= 0; s -= 8) {
        out.push_back(static_cast<std::uint8_t>(n >> s));
    }
    for (const auto &e : entries) {
        out.push_back(static_cast<std::uint8_t>(e.first));
        out.push_back(static_cast<std::uint8_t>(e.second));
    }
    out.insert(out.end(), payload_len, payload_byte);
    return out;
}

static bool rejected(const std::vector<std::uint8_t> &data) {
    return throws<std::invalid_argument>([&] { (void)ca::huffman::decompress(data); });
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

    // Hostile input (SEC-003): the code-lengths table is attacker-controlled.
    ISO_CHECK(rejected(hostile({{0, 1}, {1, 255}}, 4, 8, 0xff))); // huge length gap
    ISO_CHECK(rejected(hostile({{0, 0}}, 1, 1, 0)));              // zero length
    ISO_CHECK(rejected(hostile({{0, 1}, {1, 1}, {2, 1}}, 1, 1, 0))); // over-subscribed
    ISO_CHECK(rejected(hostile({{0, 2}, {1, 1}}, 1, 1, 0)));      // unsorted
    {
        std::vector<std::pair<int, int>> many;
        for (int i = 0; i < 257; i++) {
            many.push_back({i & 0xff, 9});
        }
        ISO_CHECK(rejected(hostile(many, 1, 1, 0))); // > 256 entries
    }
    ISO_CHECK(rejected(hostile({{0, 1}, {0, 1}}, 1, 1, 0)));      // duplicate symbol
    ISO_CHECK(rejected(hostile({{0, 1}, {1, 1}}, 0xFFFFFFFFu, 4, 0))); // u32 max length
    ISO_CHECK(rejected(hostile({{0, 1}}, 8, 100000, 0xff)));      // unmatched bits
    {
        // Claims 5 table entries but carries only one.
        auto truncated = hostile({{0, 1}}, 1, 0, 0);
        truncated[7] = 5;
        ISO_CHECK(rejected(truncated));
    }

    // Large input round-trips in linear time.
    {
        std::vector<std::uint8_t> big(400000);
        for (std::size_t i = 0; i < big.size(); i++) {
            big[i] = static_cast<std::uint8_t>(i % 7);
        }
        round_trip(big);
    }

    return ISO_TEST_RESULT();
}
