// Tests for the C++ LZ77, using the iso_test.h harness. Covers token structure
// and compress/decompress round-trips.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "lz77.hpp"

static std::vector<std::uint8_t> bytes(const std::string &s) {
    return std::vector<std::uint8_t>(s.begin(), s.end());
}

static void round_trip(const std::string &input) {
    auto data = bytes(input);
    auto packed = ca::lz77::compress(data, ca::lz77::default_window,
                                     ca::lz77::default_max_match,
                                     ca::lz77::default_min_match);
    auto restored = ca::lz77::decompress(packed);
    ISO_CHECK(restored == data);
}

int main() {
    using namespace ca::lz77;

    // Empty input → no tokens.
    ISO_CHECK(encode(bytes(""), default_window, default_max_match, default_min_match)
                  .empty());

    // "ABCDE" → five literal tokens.
    auto abcde = encode(bytes("ABCDE"), default_window, default_max_match,
                        default_min_match);
    ISO_CHECK_EQ_UINT(abcde.size(), 5);
    for (const auto &t : abcde) {
        ISO_CHECK_EQ_UINT(t.offset, 0);
        ISO_CHECK_EQ_UINT(t.length, 0);
    }

    // "AAAAAAA" → literal A + backreference (1, 5, 'A') — two tokens.
    auto as = encode(bytes("AAAAAAA"), default_window, default_max_match,
                     default_min_match);
    ISO_CHECK_EQ_UINT(as.size(), 2);
    ISO_CHECK_EQ_UINT(as[0].length, 0);
    ISO_CHECK(as[1] == (token{1, 5, 'A'}));

    // Serialise round-trips through deserialise.
    ISO_CHECK(deserialise(serialise(as)) == as);

    // Compress/decompress round-trips.
    round_trip("");
    round_trip("A");
    round_trip("ABCDE");
    round_trip("AAAAAAA");
    round_trip("abcabcabcabcabc");
    round_trip("the quick brown fox jumps over the lazy dog. the quick brown fox.");

    return ISO_TEST_RESULT();
}
