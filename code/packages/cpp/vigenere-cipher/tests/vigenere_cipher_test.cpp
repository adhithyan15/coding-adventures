// Tests for the C++ vigenere-cipher, using the iso_test.h harness. The cipher
// vectors and cryptanalysis cases are taken from the Rust crate's own tests.
#include "iso_test.h"

#include <string>

#include "vigenere_cipher.hpp"

namespace vig = ca::vigenere;

// The crate's long English sample for cryptanalysis.
static const std::string LONG_ENGLISH_TEXT =
    "The quick brown fox jumps over the lazy dog and then runs around the "
    "entire neighborhood looking for more adventures to embark upon while "
    "the sun slowly sets behind the distant mountains casting long shadows "
    "across the valley below where the river winds its way through ancient "
    "forests filled with towering oak trees and singing birds that herald "
    "the coming of spring with their melodious songs echoing through the "
    "canopy above where squirrels chase each other from branch to branch "
    "gathering acorns and other nuts for the long winter months ahead when "
    "the ground will be covered in a thick blanket of pristine white snow "
    "and the children will build snowmen and throw snowballs at each other "
    "laughing and playing until their parents call them inside for dinner "
    "where warm soup and fresh bread await them on the old wooden table";

static void check_encrypt(const std::string& pt, const std::string& key,
                          const std::string& expected) {
    auto got = vig::encrypt(pt, key);
    ISO_CHECK(got.has_value());
    if (got) {
        ISO_CHECK(*got == expected);
    }
}

static void check_decrypt(const std::string& ct, const std::string& key,
                          const std::string& expected) {
    auto got = vig::decrypt(ct, key);
    ISO_CHECK(got.has_value());
    if (got) {
        ISO_CHECK(*got == expected);
    }
}

int main() {
    // Encrypt vectors (from cipher.rs tests).
    check_encrypt("ATTACKATDAWN", "LEMON", "LXFOPVEFRNHR");
    check_encrypt("Hello, World!", "key", "Rijvs, Uyvjn!");
    check_encrypt("attackatdawn", "lemon", "lxfopvefrnhr");
    check_encrypt("ATTACKATDAWN", "LeMoN", "LXFOPVEFRNHR");
    check_encrypt("ABC", "B", "BCD");
    check_encrypt("A T", "LE", "L X");
    check_encrypt("Hello 123!", "key", "Rijvs 123!");
    check_encrypt("", "key", "");

    // Decrypt vectors.
    check_decrypt("LXFOPVEFRNHR", "LEMON", "ATTACKATDAWN");
    check_decrypt("Rijvs, Uyvjn!", "key", "Hello, World!");
    check_decrypt("lxfopvefrnhr", "lemon", "attackatdawn");
    check_decrypt("", "key", "");

    // Invalid keys are rejected (nullopt).
    ISO_CHECK(!vig::key_valid(""));
    ISO_CHECK(!vig::key_valid("key1"));
    ISO_CHECK(!vig::key_valid("ke y"));
    ISO_CHECK(vig::key_valid("LEMON"));
    ISO_CHECK(!vig::encrypt("hello", "").has_value());
    ISO_CHECK(!vig::encrypt("hello", "key1").has_value());
    ISO_CHECK(!vig::decrypt("hello", "123").has_value());

    // Round trips.
    {
        const char* texts[] = {"ATTACKATDAWN", "Hello, World!",
                               "The quick brown fox!", "MiXeD CaSe 123",
                               "ZZZZZZ"};
        const char* keys[] = {"LEMON", "key", "SECRET", "AbCdE", "A"};
        for (int i = 0; i < 5; ++i) {
            auto ct = vig::encrypt(texts[i], keys[i]);
            ISO_CHECK(ct.has_value());
            auto pt = vig::decrypt(*ct, keys[i]);
            ISO_CHECK(pt.has_value());
            ISO_CHECK(*pt == std::string(texts[i]));
        }
    }

    // Cryptanalysis: key-length detection.
    {
        auto ct5 = vig::encrypt(LONG_ENGLISH_TEXT, "LEMON");
        auto ct6 = vig::encrypt(LONG_ENGLISH_TEXT, "SECRET");
        auto ct3 = vig::encrypt(LONG_ENGLISH_TEXT, "KEY");
        ISO_CHECK_EQ_UINT(vig::find_key_length(*ct5, 20), 5u);
        ISO_CHECK_EQ_UINT(vig::find_key_length(*ct6, 20), 6u);
        ISO_CHECK_EQ_UINT(vig::find_key_length(*ct3, 20), 3u);
        ISO_CHECK_EQ_UINT(vig::find_key_length("A", 20), 1u);
    }

    // Cryptanalysis: key recovery.
    {
        auto ct5 = vig::encrypt(LONG_ENGLISH_TEXT, "LEMON");
        auto ct6 = vig::encrypt(LONG_ENGLISH_TEXT, "SECRET");
        auto ct3 = vig::encrypt(LONG_ENGLISH_TEXT, "KEY");
        ISO_CHECK(vig::find_key(*ct5, 5) == "LEMON");
        ISO_CHECK(vig::find_key(*ct6, 6) == "SECRET");
        ISO_CHECK(vig::find_key(*ct3, 3) == "KEY");
    }

    // Full automatic break.
    {
        auto ct = vig::encrypt(LONG_ENGLISH_TEXT, "LEMON");
        vig::BreakResult result = vig::break_cipher(*ct);
        ISO_CHECK(result.key == "LEMON");
        ISO_CHECK(result.plaintext == LONG_ENGLISH_TEXT);
    }

    // Break is at least self-consistent for the key it recovers.
    {
        auto ct = vig::encrypt(LONG_ENGLISH_TEXT, "CIPHER");
        vig::BreakResult result = vig::break_cipher(*ct);
        auto re = vig::encrypt(LONG_ENGLISH_TEXT, result.key);
        auto rt = vig::decrypt(*re, result.key);
        ISO_CHECK(*rt == LONG_ENGLISH_TEXT);
    }

    return ISO_TEST_RESULT();
}
