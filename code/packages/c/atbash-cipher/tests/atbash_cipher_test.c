/* Tests for the C atbash-cipher, using the iso_test.h harness. Vectors are
 * taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* strlen */

#include "atbash_cipher.h"

/* Assert atbash_encrypt(text) == expected, then free. */
static void check_encrypt(const char *text, const char *expected) {
    char *got = atbash_encrypt(text);
    ISO_CHECK(got != NULL);
    if (got) {
        ISO_CHECK_STR_EQ(got, expected);
        free(got);
    }
}

int main(void) {
    /* Single-character substitution (from cipher.rs doctests). */
    ISO_CHECK(atbash_char('A') == 'Z');
    ISO_CHECK(atbash_char('Z') == 'A');
    ISO_CHECK(atbash_char('M') == 'N');
    ISO_CHECK(atbash_char('N') == 'M');
    ISO_CHECK(atbash_char('a') == 'z');
    ISO_CHECK(atbash_char('z') == 'a');
    ISO_CHECK(atbash_char('5') == '5');
    ISO_CHECK(atbash_char('!') == '!');

    /* Basic encryption + case + punctuation. */
    check_encrypt("HELLO", "SVOOL");
    check_encrypt("hello", "svool");
    check_encrypt("Hello, World! 123", "Svool, Dliow! 123");
    check_encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "ZYXWVUTSRQPONMLKJIHGFEDCBA");
    check_encrypt("abcdefghijklmnopqrstuvwxyz", "zyxwvutsrqponmlkjihgfedcba");
    check_encrypt("AbCdEf", "ZyXwVu");

    /* Non-alphabetic passthrough. */
    check_encrypt("12345", "12345");
    check_encrypt("!@#$%", "!@#$%");
    check_encrypt("   ", "   ");
    check_encrypt("A1B2C3", "Z1Y2X3");
    check_encrypt("A\nB\tC", "Z\nY\tX");
    check_encrypt("", "");

    /* decrypt == encrypt (self-inverse), and the SVOOL vectors. */
    {
        char *d = atbash_decrypt("SVOOL");
        ISO_CHECK(d != NULL);
        if (d) {
            ISO_CHECK_STR_EQ(d, "HELLO");
            free(d);
        }
    }

    /* Self-inverse: encrypt twice restores the input, for several strings. */
    {
        const char *cases[] = {
            "HELLO", "hello", "Hello, World! 123", "",
            "The quick brown fox jumps over the lazy dog! 42"};
        size_t i;
        for (i = 0; i < 5; i++) {
            char *once = atbash_encrypt(cases[i]);
            char *twice;
            ISO_CHECK(once != NULL);
            twice = atbash_encrypt(once);
            ISO_CHECK(twice != NULL);
            ISO_CHECK_STR_EQ(twice, cases[i]);
            free(once);
            free(twice);
        }
    }

    /* No letter maps to itself (25 - p == p has no integer solution in 0..25). */
    {
        int i;
        for (i = 0; i < 26; i++) {
            char up = (char)('A' + i);
            char lo = (char)('a' + i);
            ISO_CHECK(atbash_char(up) != up);
            ISO_CHECK(atbash_char(lo) != lo);
        }
    }

    /* encrypt(text) == decrypt(text) for the same input. */
    {
        const char *cases[] = {"HELLO", "svool", "Test!", ""};
        size_t i;
        for (i = 0; i < 4; i++) {
            char *e = atbash_encrypt(cases[i]);
            char *d = atbash_decrypt(cases[i]);
            ISO_CHECK(e && d);
            ISO_CHECK_STR_EQ(e, d);
            free(e);
            free(d);
        }
    }

    return ISO_TEST_RESULT();
}
