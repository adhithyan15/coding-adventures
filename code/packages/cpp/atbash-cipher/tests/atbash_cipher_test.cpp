// Tests for the C++ atbash-cipher, using the iso_test.h harness. Vectors are
// taken from the Rust crate's own tests.
#include "iso_test.h"

#include <string>

#include "atbash_cipher.hpp"

namespace atb = ca::atbash;

int main() {
    // Single-character substitution.
    ISO_CHECK(atb::atbash_char('A') == 'Z');
    ISO_CHECK(atb::atbash_char('Z') == 'A');
    ISO_CHECK(atb::atbash_char('M') == 'N');
    ISO_CHECK(atb::atbash_char('N') == 'M');
    ISO_CHECK(atb::atbash_char('a') == 'z');
    ISO_CHECK(atb::atbash_char('z') == 'a');
    ISO_CHECK(atb::atbash_char('5') == '5');
    ISO_CHECK(atb::atbash_char('!') == '!');

    // Basic encryption + case + punctuation.
    ISO_CHECK(atb::encrypt("HELLO") == "SVOOL");
    ISO_CHECK(atb::encrypt("hello") == "svool");
    ISO_CHECK(atb::encrypt("Hello, World! 123") == "Svool, Dliow! 123");
    ISO_CHECK(atb::encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ") ==
              "ZYXWVUTSRQPONMLKJIHGFEDCBA");
    ISO_CHECK(atb::encrypt("abcdefghijklmnopqrstuvwxyz") ==
              "zyxwvutsrqponmlkjihgfedcba");
    ISO_CHECK(atb::encrypt("AbCdEf") == "ZyXwVu");

    // Non-alphabetic passthrough.
    ISO_CHECK(atb::encrypt("12345") == "12345");
    ISO_CHECK(atb::encrypt("!@#$%") == "!@#$%");
    ISO_CHECK(atb::encrypt("   ") == "   ");
    ISO_CHECK(atb::encrypt("A1B2C3") == "Z1Y2X3");
    ISO_CHECK(atb::encrypt("A\nB\tC") == "Z\nY\tX");
    ISO_CHECK(atb::encrypt("") == "");

    // decrypt vectors and self-inverse.
    ISO_CHECK(atb::decrypt("SVOOL") == "HELLO");
    ISO_CHECK(atb::decrypt("svool") == "hello");

    {
        const char* cases[] = {
            "HELLO", "hello", "Hello, World! 123", "",
            "The quick brown fox jumps over the lazy dog! 42"};
        for (int i = 0; i < 5; ++i) {
            ISO_CHECK(atb::encrypt(atb::encrypt(cases[i])) ==
                      std::string(cases[i]));
        }
    }

    // No letter maps to itself.
    for (int i = 0; i < 26; ++i) {
        char up = static_cast<char>('A' + i);
        char lo = static_cast<char>('a' + i);
        ISO_CHECK(atb::atbash_char(up) != up);
        ISO_CHECK(atb::atbash_char(lo) != lo);
    }

    // encrypt == decrypt for the same input.
    {
        const char* cases[] = {"HELLO", "svool", "Test!", ""};
        for (int i = 0; i < 4; ++i) {
            ISO_CHECK(atb::encrypt(cases[i]) == atb::decrypt(cases[i]));
        }
    }

    return ISO_TEST_RESULT();
}
