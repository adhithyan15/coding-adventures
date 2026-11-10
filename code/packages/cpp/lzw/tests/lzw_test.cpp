// Tests for the C++ LZW, using the iso_test.h harness. Compress/decompress
// round-trips exercising dictionary growth, the KwKwK case, and larger data.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "lzw.hpp"

static std::vector<std::uint8_t> bytes(const std::string &s) {
    return std::vector<std::uint8_t>(s.begin(), s.end());
}

static void round_trip(const std::vector<std::uint8_t> &input) {
    auto packed = ca::lzw::compress(input);
    ISO_CHECK(packed.size() >= 4);
    auto restored = ca::lzw::decompress(packed);
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
    return ca::lzw::decompress(ca::lzw::compress(d)) == d;
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
    using ca::lzw::decompress;
    // Garbage headers either throw or return promptly; never over-allocate.
    for (std::size_t n : {std::size_t(4), std::size_t(8), std::size_t(64)}) {
        try {
            (void)decompress(Bytes(n, 0xFF));
        } catch (const std::invalid_argument &) {
        }
    }
    Bytes text = mk({'h', 'e', 'l', 'l', 'o', ' ', 'h', 'e', 'l', 'l', 'o', ' ',
                     'h', 'e', 'l', 'l', 'o'});
    Bytes c = ca::lzw::compress(text);
    c[0] = c[1] = c[2] = c[3] = 0xFF;
    ISO_CHECK(decompress(c) == text); // declared 2^32-1: output is what the codes give
    c[0] = c[1] = c[2] = 0;
    c[3] = 5;
    ISO_CHECK(decompress(c) == mk({'h', 'e', 'l', 'l', 'o'})); // truncates, stops early
    run_round_trips();
}
} // namespace hostile

int main() {
    round_trip(bytes(""));
    round_trip(bytes("A"));
    round_trip(bytes("AB"));
    round_trip(bytes("TOBEORNOTTOBEORTOBEORNOT"));
    round_trip(std::vector<std::uint8_t>(1000, 'A')); // long run → KwKwK

    // Repeating pattern → dictionary growth.
    {
        std::vector<std::uint8_t> buf(900);
        for (std::size_t i = 0; i < buf.size(); i++) {
            buf[i] = static_cast<std::uint8_t>("abc"[i % 3]);
        }
        round_trip(buf);
    }

    // Full byte alphabet.
    {
        std::vector<std::uint8_t> buf(1024);
        for (std::size_t i = 0; i < buf.size(); i++) {
            buf[i] = static_cast<std::uint8_t>(i & 0xff);
        }
        round_trip(buf);
    }

    // Header carries the big-endian original length.
    {
        auto packed = ca::lzw::compress(bytes("hello"));
        std::size_t recorded = (static_cast<std::size_t>(packed[0]) << 24) |
                               (static_cast<std::size_t>(packed[1]) << 16) |
                               (static_cast<std::size_t>(packed[2]) << 8) |
                               packed[3];
        ISO_CHECK_EQ_UINT(recorded, 5);
    }

    // Malformed input throws.
    ISO_CHECK(throws<std::invalid_argument>(
        [] { (void)ca::lzw::decompress(std::vector<std::uint8_t>{1, 2}); }));

    hostile::run();

    return ISO_TEST_RESULT();
}
