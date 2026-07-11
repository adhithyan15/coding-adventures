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

    return ISO_TEST_RESULT();
}
