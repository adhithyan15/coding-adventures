// Tests for the C++ lz78, using the iso_test.h harness. Token vectors and round
// trips are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "lz78.hpp"

namespace lz78 = ca::lz78;
using Bytes = std::vector<std::uint8_t>;
using lz78::Token;

static Bytes bytes(const char* s) {
    Bytes b;
    for (const char* p = s; *p; ++p) {
        b.push_back(static_cast<std::uint8_t>(*p));
    }
    return b;
}

int main() {
    const std::size_t MAX_DICT = 65536;

    // Empty input.
    {
        ISO_CHECK(lz78::encode(Bytes{}, MAX_DICT).empty());
        ISO_CHECK(lz78::decode(std::vector<Token>{}, std::size_t(0)).empty());
    }

    // Single byte.
    {
        auto t = lz78::encode(bytes("A"), MAX_DICT);
        ISO_CHECK((t == std::vector<Token>{{0, 65}}));
    }

    // No repetition.
    {
        auto t = lz78::encode(bytes("ABCDE"), MAX_DICT);
        ISO_CHECK_EQ_UINT(t.size(), 5u);
        bool all_lit = true;
        for (auto& tok : t) {
            if (tok.dict_index != 0) all_lit = false;
        }
        ISO_CHECK(all_lit);
    }

    // Token vectors.
    {
        std::vector<Token> want = {{0, 65}, {1, 66}, {0, 67},
                                   {0, 66}, {4, 65}, {4, 67}};
        ISO_CHECK((lz78::encode(bytes("AABCBBABC"), MAX_DICT) == want));
    }
    {
        std::vector<Token> want = {{0, 65}, {0, 66}, {1, 66}, {3, 0}};
        ISO_CHECK((lz78::encode(bytes("ABABAB"), MAX_DICT) == want));
    }
    {
        ISO_CHECK_EQ_UINT(lz78::encode(bytes("AAAAAAA"), MAX_DICT).size(), 4u);
    }

    // Round trips (text + binary).
    {
        const char* texts[] = {"",         "A",       "ABCDE",
                               "AAAAAAA",  "ABABABAB", "AABCBBABC",
                               "hello world", "ababababab"};
        for (const char* s : texts) {
            Bytes d = bytes(s);
            ISO_CHECK(lz78::decompress(lz78::compress(d, MAX_DICT)) == d);
        }
    }
    {
        std::vector<Bytes> cases = {
            {0, 0, 0}, {255, 255, 255}, {0, 1, 2, 0, 1, 2}, {0, 0, 0, 255, 255}};
        Bytes all256;
        for (int i = 0; i < 256; ++i) {
            all256.push_back(static_cast<std::uint8_t>(i));
        }
        cases.push_back(all256);
        for (const Bytes& d : cases) {
            ISO_CHECK(lz78::decompress(lz78::compress(d, MAX_DICT)) == d);
        }
    }

    // max_dict_size respected.
    {
        auto t = lz78::encode(bytes("ABCABCABCABCABC"), 10);
        bool ok = true;
        for (auto& tok : t) {
            if (tok.dict_index >= 10) ok = false;
        }
        ISO_CHECK(ok);
    }
    {
        auto t = lz78::encode(bytes("AAAA"), 1);
        bool ok = true;
        for (auto& tok : t) {
            if (tok.dict_index != 0) ok = false;
        }
        ISO_CHECK(ok);
    }

    // Wire size == 8 + tokens*4.
    {
        auto t = lz78::encode(bytes("AB"), MAX_DICT);
        auto c = lz78::compress(bytes("AB"), MAX_DICT);
        ISO_CHECK_EQ_UINT(c.size(), 8u + t.size() * 4u);
    }

    // Deterministic.
    {
        Bytes d = bytes("hello world test repeated");
        ISO_CHECK(lz78::compress(d, MAX_DICT) == lz78::compress(d, MAX_DICT));
    }

    // Repetitive data compresses.
    {
        Bytes d;
        for (int i = 0; i < 3000; ++i) {
            d.push_back(static_cast<std::uint8_t>("ABC"[i % 3]));
        }
        ISO_CHECK(lz78::compress(d, MAX_DICT).size() < d.size());
    }
    {
        Bytes d(10000, 65);
        auto c = lz78::compress(d, MAX_DICT);
        ISO_CHECK(c.size() < d.size());
        ISO_CHECK(lz78::decompress(c) == d);
    }

    // TrieCursor doctest behaviour.
    {
        lz78::TrieCursor c;
        ISO_CHECK(!c.step('A'));
        c.insert('A', 1);
        c.reset();
        ISO_CHECK(c.step('A'));
        ISO_CHECK(c.dict_id() == 1);
        ISO_CHECK(!c.at_root());
    }

    // Malformed decompress input must not crash (bounds/cycle guards).
    {
        Bytes bad = {0,    0,    0,  4,  // orig_len 4
                     0,    0,    0,  2,  // token_count 2
                     0xFF, 0xFF, 65, 0,  // dict_index 65535 (OOB)
                     0,    1,    66, 0}; // dict_index 1
        Bytes out = lz78::decompress(bad); // must simply not crash
        (void)out;
        ISO_CHECK(true);
    }

    return ISO_TEST_RESULT();
}
