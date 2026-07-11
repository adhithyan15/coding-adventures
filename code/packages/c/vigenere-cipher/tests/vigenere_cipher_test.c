/* Tests for the C vigenere-cipher, using the iso_test.h harness. The cipher
 * vectors and cryptanalysis cases are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free, NULL */
#include <string.h> /* strcmp */

#include "vigenere_cipher.h"

/* The crate's long English sample: cryptanalysis needs a few hundred letters to
 * distinguish English from random and avoid multiples of the true key length. */
static const char *LONG_ENGLISH_TEXT =
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

/* Assert vigenere_encrypt(plaintext, key) == expected, then free. */
static void check_encrypt(const char *plaintext, const char *key,
                          const char *expected) {
    char *got = vigenere_encrypt(plaintext, key);
    ISO_CHECK(got != NULL);
    if (got) {
        ISO_CHECK_STR_EQ(got, expected);
        free(got);
    }
}

static void check_decrypt(const char *ciphertext, const char *key,
                          const char *expected) {
    char *got = vigenere_decrypt(ciphertext, key);
    ISO_CHECK(got != NULL);
    if (got) {
        ISO_CHECK_STR_EQ(got, expected);
        free(got);
    }
}

int main(void) {
    /* Encrypt vectors (from cipher.rs tests). */
    check_encrypt("ATTACKATDAWN", "LEMON", "LXFOPVEFRNHR");
    check_encrypt("Hello, World!", "key", "Rijvs, Uyvjn!");
    check_encrypt("attackatdawn", "lemon", "lxfopvefrnhr");
    check_encrypt("ATTACKATDAWN", "LeMoN", "LXFOPVEFRNHR"); /* mixed-case key */
    check_encrypt("ABC", "B", "BCD");
    check_encrypt("A T", "LE", "L X");     /* key skips the space */
    check_encrypt("Hello 123!", "key", "Rijvs 123!");
    check_encrypt("", "key", "");

    /* Decrypt vectors. */
    check_decrypt("LXFOPVEFRNHR", "LEMON", "ATTACKATDAWN");
    check_decrypt("Rijvs, Uyvjn!", "key", "Hello, World!");
    check_decrypt("lxfopvefrnhr", "lemon", "attackatdawn");
    check_decrypt("", "key", "");

    /* Invalid keys are rejected (NULL). */
    ISO_CHECK(!vigenere_key_valid(""));
    ISO_CHECK(!vigenere_key_valid("key1"));
    ISO_CHECK(!vigenere_key_valid("ke y"));
    ISO_CHECK(vigenere_key_valid("LEMON"));
    ISO_CHECK(vigenere_encrypt("hello", "") == NULL);
    ISO_CHECK(vigenere_encrypt("hello", "key1") == NULL);
    ISO_CHECK(vigenere_decrypt("hello", "123") == NULL);

    /* Round trips over assorted inputs. */
    {
        const char *texts[] = {"ATTACKATDAWN", "Hello, World!",
                               "The quick brown fox!", "MiXeD CaSe 123",
                               "ZZZZZZ"};
        const char *keys[] = {"LEMON", "key", "SECRET", "AbCdE", "A"};
        size_t i;
        for (i = 0; i < 5; i++) {
            char *ct = vigenere_encrypt(texts[i], keys[i]);
            char *pt;
            ISO_CHECK(ct != NULL);
            pt = vigenere_decrypt(ct, keys[i]);
            ISO_CHECK(pt != NULL);
            ISO_CHECK_STR_EQ(pt, texts[i]);
            free(ct);
            free(pt);
        }
    }

    /* Cryptanalysis: key-length detection (from analysis.rs tests). */
    {
        char *ct5 = vigenere_encrypt(LONG_ENGLISH_TEXT, "LEMON");
        char *ct6 = vigenere_encrypt(LONG_ENGLISH_TEXT, "SECRET");
        char *ct3 = vigenere_encrypt(LONG_ENGLISH_TEXT, "KEY");
        ISO_CHECK_EQ_UINT(vigenere_find_key_length(ct5, 20), 5u);
        ISO_CHECK_EQ_UINT(vigenere_find_key_length(ct6, 20), 6u);
        ISO_CHECK_EQ_UINT(vigenere_find_key_length(ct3, 20), 3u);
        ISO_CHECK_EQ_UINT(vigenere_find_key_length("A", 20), 1u);
        free(ct5);
        free(ct6);
        free(ct3);
    }

    /* Cryptanalysis: key recovery via chi-squared. */
    {
        char *ct5 = vigenere_encrypt(LONG_ENGLISH_TEXT, "LEMON");
        char *ct6 = vigenere_encrypt(LONG_ENGLISH_TEXT, "SECRET");
        char *ct3 = vigenere_encrypt(LONG_ENGLISH_TEXT, "KEY");
        char *k5 = vigenere_find_key(ct5, 5);
        char *k6 = vigenere_find_key(ct6, 6);
        char *k3 = vigenere_find_key(ct3, 3);
        ISO_CHECK(k5 && strcmp(k5, "LEMON") == 0);
        ISO_CHECK(k6 && strcmp(k6, "SECRET") == 0);
        ISO_CHECK(k3 && strcmp(k3, "KEY") == 0);
        free(ct5);
        free(ct6);
        free(ct3);
        free(k5);
        free(k6);
        free(k3);
    }

    /* Full automatic break. */
    {
        char *ct = vigenere_encrypt(LONG_ENGLISH_TEXT, "LEMON");
        VigenereBreak result;
        ISO_CHECK(vigenere_break(ct, &result));
        ISO_CHECK_STR_EQ(result.key, "LEMON");
        ISO_CHECK_STR_EQ(result.plaintext, LONG_ENGLISH_TEXT);
        vigenere_break_free(&result);
        free(ct);
    }

    /* Break is at least self-consistent for any key it recovers. */
    {
        char *ct = vigenere_encrypt(LONG_ENGLISH_TEXT, "CIPHER");
        VigenereBreak result;
        ISO_CHECK(vigenere_break(ct, &result));
        {
            char *re = vigenere_encrypt(LONG_ENGLISH_TEXT, result.key);
            char *rt = vigenere_decrypt(re, result.key);
            ISO_CHECK_STR_EQ(rt, LONG_ENGLISH_TEXT);
            free(re);
            free(rt);
        }
        vigenere_break_free(&result);
        free(ct);
    }

    return ISO_TEST_RESULT();
}
