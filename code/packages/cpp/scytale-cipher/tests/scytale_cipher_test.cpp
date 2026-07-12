// Tests for the C++ scytale-cipher, using the iso_test.h harness. Vectors are
// taken from the Rust crate's own tests.
#include "iso_test.h"

#include <string>
#include <vector>

#include "scytale_cipher.hpp"

namespace sc = ca::scytale;

int main() {
    // Encryption vectors.
    ISO_CHECK(sc::encrypt("HELLO WORLD", 3) == std::string("HLWLEOODL R "));
    ISO_CHECK(sc::encrypt("ABCDEF", 2) == std::string("ACEBDF"));
    ISO_CHECK(sc::encrypt("ABCDEF", 3) == std::string("ADBECF"));
    ISO_CHECK(sc::encrypt("ABCD", 4) == std::string("ABCD"));
    ISO_CHECK(sc::encrypt("", 2) == std::string(""));

    // Invalid keys -> nullopt (empty text is always OK, key ignored).
    ISO_CHECK(!sc::encrypt("HELLO", 0).has_value());
    ISO_CHECK(!sc::encrypt("HELLO", 1).has_value());
    ISO_CHECK(!sc::encrypt("HI", 3).has_value());
    ISO_CHECK(sc::encrypt("", 0) == std::string(""));

    // Decryption vectors.
    ISO_CHECK(sc::decrypt("HLWLEOODL R ", 3) == std::string("HELLO WORLD"));
    ISO_CHECK(sc::decrypt("ACEBDF", 2) == std::string("ABCDEF"));
    ISO_CHECK(sc::decrypt("", 2) == std::string(""));
    ISO_CHECK(!sc::decrypt("HELLO", 0).has_value());
    ISO_CHECK(!sc::decrypt("HI", 3).has_value());

    // Padding stripped; no padding needed keeps the length.
    {
        auto ct = sc::encrypt("HELLO", 3);
        ISO_CHECK(ct.has_value());
        ISO_CHECK(sc::decrypt(*ct, 3) == std::string("HELLO"));
        auto ct2 = sc::encrypt("ABCDEF", 2);
        ISO_CHECK(ct2.has_value() && ct2->size() == 6);
    }

    // Round trips.
    {
        struct Case {
            const char* text;
            std::size_t key;
        };
        Case cases[] = {{"HELLO WORLD", 3}, {"ABCDEF", 2}, {"ABCDEF", 3},
                        {"The quick brown fox", 4}, {"12345", 2}};
        for (const Case& c : cases) {
            auto ct = sc::encrypt(c.text, c.key);
            ISO_CHECK(ct.has_value());
            ISO_CHECK(sc::decrypt(*ct, c.key) == std::string(c.text));
        }
    }
    {
        std::string text = "The quick brown fox jumps over the lazy dog!";
        std::size_t n = text.size();
        for (std::size_t key = 2; key <= n / 2; ++key) {
            auto ct = sc::encrypt(text, key);
            ISO_CHECK(ct.has_value());
            ISO_CHECK(sc::decrypt(*ct, key) == text);
        }
    }

    // Brute force finds the original key.
    {
        auto ct = sc::encrypt("HELLO WORLD", 3);
        ISO_CHECK(ct.has_value());
        std::vector<sc::BruteForceResult> results = sc::brute_force(*ct);
        bool found = false;
        for (const auto& r : results) {
            if (r.key == 3) {
                found = true;
                ISO_CHECK(r.text == std::string("HELLO WORLD"));
            }
        }
        ISO_CHECK(found);
    }

    // Brute force returns every candidate key.
    {
        std::vector<sc::BruteForceResult> results = sc::brute_force("ABCDEFGHIJ");
        std::vector<std::size_t> keys;
        for (const auto& r : results) {
            keys.push_back(r.key);
        }
        ISO_CHECK(keys == std::vector<std::size_t>({2, 3, 4, 5}));
    }

    // Brute force on short text is empty.
    ISO_CHECK(sc::brute_force("AB").empty());
    ISO_CHECK(sc::brute_force("ABC").empty());

    // Multibyte UTF-8 characters stay intact through a round trip.
    {
        std::string text = "caf\xc3\xa9 na\xc3\xafve"; // "café naïve"
        auto ct = sc::encrypt(text, 3);
        ISO_CHECK(ct.has_value());
        ISO_CHECK(sc::decrypt(*ct, 3) == text);
    }

    return ISO_TEST_RESULT();
}
