// Tests for the C++ rans, using the iso_test.h harness. Table vectors and round
// trips are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "rans.hpp"

namespace rans = ca::rans;
using Counts = std::vector<std::uint32_t>;
using Syms = std::vector<std::uint8_t>;

static void round_trip(const rans::AnsTable& t, const Syms& syms) {
    rans::RansEncoder enc(t);
    for (std::size_t i = syms.size(); i > 0; --i) {
        enc.put(syms[i - 1]);
    }
    std::vector<std::uint8_t> bytes = enc.finish();
    ISO_CHECK(bytes.size() >= 8);
    rans::RansDecoder dec(t, bytes);
    for (std::uint8_t expected : syms) {
        ISO_CHECK(dec.get() == expected);
    }
}

int main() {
    // Table [1,1].
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{1, 1});
        ISO_CHECK_EQ_UINT(t.m(), 2u);
        ISO_CHECK_EQ_UINT(t.log2m(), 1u);
        ISO_CHECK_EQ_UINT(t.alphabet_size(), 2u);
        ISO_CHECK(t.freq(0) == std::optional<std::uint32_t>(1));
        ISO_CHECK(t.freq(1) == std::optional<std::uint32_t>(1));
        ISO_CHECK(t.cumfreq(0) == std::optional<std::uint32_t>(0));
        ISO_CHECK(t.cumfreq(1) == std::optional<std::uint32_t>(1));
        ISO_CHECK(!t.freq(2).has_value());
        ISO_CHECK(!t.cumfreq(2).has_value());
    }

    // Table [3,1].
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{3, 1});
        ISO_CHECK_EQ_UINT(t.m(), 4u);
        ISO_CHECK_EQ_UINT(t.log2m(), 2u);
        ISO_CHECK(t.freq(0) == std::optional<std::uint32_t>(3));
        ISO_CHECK(t.freq(1) == std::optional<std::uint32_t>(1));
    }

    // Sum to M; power of two.
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{10, 5, 3});
        ISO_CHECK_EQ_UINT(t.alphabet_size(), 3u);
        std::uint32_t m = t.m();
        ISO_CHECK((m & (m - 1)) == 0);
        std::uint32_t sum = 0;
        for (std::size_t i = 0; i < 3; ++i) {
            sum += t.freq(i).value();
        }
        ISO_CHECK_EQ_UINT(sum, m);
    }

    // log2m for M = 8, 16.
    {
        rans::AnsTable t8 = rans::AnsTable::build(Counts{5, 3});
        ISO_CHECK_EQ_UINT(t8.m(), 8u);
        ISO_CHECK_EQ_UINT(t8.log2m(), 3u);
        rans::AnsTable t16 = rans::AnsTable::build(Counts{10, 6});
        ISO_CHECK_EQ_UINT(t16.m(), 16u);
        ISO_CHECK_EQ_UINT(t16.log2m(), 4u);
    }

    // Error cases (throw).
    {
        auto throws = [](const Counts& c) {
            try {
                rans::AnsTable::build(c);
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        ISO_CHECK(throws(Counts{}));
        ISO_CHECK(throws(Counts{0, 0, 0}));
        ISO_CHECK(throws(Counts(257, 1)));
    }

    // Decoder rejects short data.
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{1, 1});
        bool threw = false;
        try {
            rans::RansDecoder d(t, std::vector<std::uint8_t>(7, 0));
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Round trips.
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{3, 1});
        round_trip(t, Syms{0, 0, 1, 0});
        round_trip(t, Syms{0});
        round_trip(t, Syms{1, 0, 1, 1, 0, 0, 1, 0});
    }
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{120, 8});
        Syms skewed;
        for (int i = 0; i < 16; ++i) {
            skewed.push_back(static_cast<std::uint8_t>(i == 7 ? 1 : 0));
        }
        round_trip(t, skewed);
    }
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{5, 3, 2});
        round_trip(t, Syms{0, 1, 2, 0, 1, 0, 2, 1, 0, 0});
    }

    // Malformed input (all-zero state) must not hang the decoder.
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{1, 1});
        rans::RansDecoder d(t, std::vector<std::uint8_t>(8, 0));
        for (int i = 0; i < 100; ++i) {
            (void)d.get(); // must not spin forever
        }
        ISO_CHECK(true);
    }

    // Deterministic encoding.
    {
        rans::AnsTable t = rans::AnsTable::build(Counts{1, 1});
        auto encode = [&t]() {
            rans::RansEncoder e(t);
            Syms seq = {0, 1, 1, 0};
            for (std::size_t i = seq.size(); i > 0; --i) {
                e.put(seq[i - 1]);
            }
            return e.finish();
        };
        ISO_CHECK(encode() == encode());
    }

    return ISO_TEST_RESULT();
}
