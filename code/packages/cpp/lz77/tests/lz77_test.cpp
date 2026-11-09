// Tests for the C++ LZ77, using the iso_test.h harness. Covers token structure
// and compress/decompress round-trips.
#include "iso_test.h"

#include <initializer_list>
#include <stdexcept>

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


// ---- Security regression tests: hostile headers ---------------------------
// Each case is a few bytes of input whose header claims an enormous count or
// length.  An implementation that sized an allocation or loop from the claim
// would allocate gigabytes or read out of bounds; these checks (best run under
// ASan/UBSan, see the CHANGELOG) assert the call returns promptly with the data
// actually present.  Also: round trips over every input of length <= 2 and a
// seeded random sample.
namespace hostile {
using Bytes = std::vector<std::uint8_t>;

static Bytes mk(std::initializer_list<int> v) {
    Bytes b;
    for (int x : v) {
        b.push_back(static_cast<std::uint8_t>(x));
    }
    return b;
}

static std::uint32_t rng_state = 0x5EEDu;
static unsigned rng() {
    std::uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    rng_state = x;
    return x;
}


static bool rt(const Bytes &d) {
    return ca::lz77::decompress(ca::lz77::compress(d, ca::lz77::default_window,
                                                   ca::lz77::default_max_match,
                                                   ca::lz77::default_min_match)) == d;
}

static void run_round_trips() {
    ISO_CHECK(rt(Bytes{}));
    for (unsigned a = 0; a < 256; a++) {
        ISO_CHECK(rt(mk({static_cast<int>(a)})));
        for (unsigned b = 0; b < 256; b++) {
            ISO_CHECK(rt(mk({static_cast<int>(a), static_cast<int>(b)})));
        }
    }
    for (unsigned i = 0; i < 300; i++) {
        unsigned n = 3 + rng() % 597;
        unsigned alpha = (i % 4 == 0) ? 2 : (i % 4 == 1) ? 4 : (i % 4 == 2) ? 16 : 256;
        Bytes d;
        for (unsigned j = 0; j < n; j++) {
            d.push_back(static_cast<std::uint8_t>(rng() % alpha));
        }
        ISO_CHECK(rt(d));
    }
}
static void run() {
    using ca::lz77::decompress;
    ISO_CHECK(decompress(mk({255, 255, 255, 255})).empty());
    // Count far larger than the data: only the present tokens decode.
    ISO_CHECK(decompress(mk({255, 255, 255, 255, 0, 0, 0, 'h', 0, 0, 0, 'i', 0, 0, 0})) ==
              mk({'h', 'i'}));
    // Count smaller than the data: stops at the count.
    ISO_CHECK(decompress(mk({0, 0, 0, 1, 0, 0, 0, 'h', 0, 0, 0, 'i'})) == mk({'h'}));
    // Invalid back-references stop decoding (previously: out-of-bounds read).
    ISO_CHECK(decompress(mk({0, 0, 0, 1, 0, 5, 3, 'x'})).empty());
    ISO_CHECK(decompress(mk({0, 0, 0, 1, 0, 0, 3, 'x'})).empty());
    ISO_CHECK(decompress(mk({0, 0, 0, 2, 0, 0, 0, 'a', 0, 9, 2, 'z'})) == mk({'a'}));
    run_round_trips();
}
} // namespace hostile

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

    hostile::run();

    return ISO_TEST_RESULT();
}
