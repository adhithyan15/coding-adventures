// Tests for the C++ lzss, using the iso_test.h harness. Vectors and round trips
// are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "lzss.hpp"

namespace lzss = ca::lzss;
using Bytes = std::vector<std::uint8_t>;
using lzss::Token;

static Bytes bytes(const char* s) {
    Bytes b;
    for (const char* p = s; *p; ++p) {
        b.push_back(static_cast<std::uint8_t>(*p));
    }
    return b;
}
static std::vector<Token> enc(const char* s) {
    return lzss::encode(bytes(s), lzss::DEFAULT_WINDOW_SIZE,
                        lzss::DEFAULT_MAX_MATCH, lzss::DEFAULT_MIN_MATCH);
}

int main() {
    // Empty encode.
    ISO_CHECK(enc("").empty());

    // Single byte.
    {
        auto t = enc("A");
        ISO_CHECK((t == std::vector<Token>{Token::lit('A')}));
    }

    // No repetition.
    {
        auto t = enc("ABCDE");
        ISO_CHECK_EQ_UINT(t.size(), 5u);
        bool all_lit = true;
        for (auto& tok : t) {
            if (tok.is_match) all_lit = false;
        }
        ISO_CHECK(all_lit);
    }

    // "AABCBBABC": 7 tokens, last is Match{5,3}.
    {
        auto t = enc("AABCBBABC");
        ISO_CHECK_EQ_UINT(t.size(), 7u);
        if (t.size() == 7) {
            ISO_CHECK((t[6] == Token::match(5, 3)));
        }
    }

    // "ABABAB".
    {
        std::vector<Token> want = {Token::lit('A'), Token::lit('B'),
                                   Token::match(2, 4)};
        ISO_CHECK((enc("ABABAB") == want));
    }

    // "AAAAAAA".
    {
        std::vector<Token> want = {Token::lit('A'), Token::match(1, 6)};
        ISO_CHECK((enc("AAAAAAA") == want));
    }

    // min_match large forces literals.
    {
        auto t = lzss::encode(bytes("ABABAB"), lzss::DEFAULT_WINDOW_SIZE,
                              lzss::DEFAULT_MAX_MATCH, 100);
        bool all_lit = true;
        for (auto& tok : t) {
            if (tok.is_match) all_lit = false;
        }
        ISO_CHECK(all_lit);
    }

    // Match offset within a small window.
    {
        auto t = lzss::encode(bytes("ABCABCABCABC"), 8, lzss::DEFAULT_MAX_MATCH,
                              lzss::DEFAULT_MIN_MATCH);
        bool ok = true;
        for (auto& tok : t) {
            if (tok.is_match && tok.offset > 8) ok = false;
        }
        ISO_CHECK(ok);
    }

    // Match length bounded by max_match.
    {
        auto t = lzss::encode(Bytes(100, 'A'), lzss::DEFAULT_WINDOW_SIZE, 5,
                              lzss::DEFAULT_MIN_MATCH);
        bool ok = true;
        for (auto& tok : t) {
            if (tok.is_match && tok.length > 5) ok = false;
        }
        ISO_CHECK(ok);
    }

    // decode vectors.
    {
        ISO_CHECK(lzss::decode({}, std::size_t(0)).empty());
        ISO_CHECK(lzss::decode({Token::lit('A')}, std::size_t(1)) ==
                  bytes("A"));
        std::vector<Token> ov = {Token::lit('A'), Token::match(1, 6)};
        ISO_CHECK(lzss::decode(ov, std::size_t(7)) == bytes("AAAAAAA"));
        std::vector<Token> ab = {Token::lit('A'), Token::lit('B'),
                                 Token::match(2, 4)};
        ISO_CHECK(lzss::decode(ab, std::size_t(6)) == bytes("ABABAB"));
    }

    // Round trips (text + binary).
    {
        const char* texts[] = {"",         "A",        "ABCDE",
                               "AAAAAAA",  "ABABABAB", "AABCBBABC",
                               "hello world hello world", "the quick brown fox"};
        for (const char* s : texts) {
            Bytes d = bytes(s);
            ISO_CHECK(lzss::decompress(lzss::compress(d)) == d);
        }
    }
    {
        Bytes all256;
        for (int i = 0; i < 256; ++i) {
            all256.push_back(static_cast<std::uint8_t>(i));
        }
        ISO_CHECK(lzss::decompress(lzss::compress(all256)) == all256);
        Bytes reps;
        for (int i = 0; i < 3000; ++i) {
            reps.push_back(static_cast<std::uint8_t>("ABC"[i % 3]));
        }
        ISO_CHECK(lzss::decompress(lzss::compress(reps)) == reps);
    }

    // Repetitive data compresses.
    {
        Bytes d(10000, 'A');
        ISO_CHECK(lzss::compress(d).size() < d.size());
    }

    // Malformed decompress must not crash.
    {
        Bytes bad = {0,    0,    0,    8,  0,  0, 0, 99,
                     0x01, 0xFF, 0xFF, 5,  0x00, 66};
        Bytes out = lzss::decompress(bad);
        (void)out;
        ISO_CHECK(true);
    }

    return ISO_TEST_RESULT();
}
