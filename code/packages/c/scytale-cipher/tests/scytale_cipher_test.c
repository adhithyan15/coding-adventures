/* Tests for the C scytale-cipher, using the iso_test.h harness. Vectors are
 * taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free, NULL */
#include <string.h> /* strlen */

#include "scytale_cipher.h"

static void check_encrypt(const char *text, size_t key, const char *expected) {
    char *got = scytale_encrypt(text, key);
    ISO_CHECK(got != NULL);
    if (got) {
        ISO_CHECK_STR_EQ(got, expected);
        free(got);
    }
}

static void check_decrypt(const char *text, size_t key, const char *expected) {
    char *got = scytale_decrypt(text, key);
    ISO_CHECK(got != NULL);
    if (got) {
        ISO_CHECK_STR_EQ(got, expected);
        free(got);
    }
}

int main(void) {
    /* Encryption vectors. */
    check_encrypt("HELLO WORLD", 3, "HLWLEOODL R ");
    check_encrypt("ABCDEF", 2, "ACEBDF");
    check_encrypt("ABCDEF", 3, "ADBECF");
    check_encrypt("ABCD", 4, "ABCD"); /* key == length: identity */
    check_encrypt("", 2, "");         /* empty text */

    /* Invalid keys -> NULL (empty check happens first, so "" is always OK). */
    ISO_CHECK(scytale_encrypt("HELLO", 0) == NULL);
    ISO_CHECK(scytale_encrypt("HELLO", 1) == NULL);
    ISO_CHECK(scytale_encrypt("HI", 3) == NULL); /* key > length */
    {
        char *e = scytale_encrypt("", 0); /* empty text ignores the key */
        ISO_CHECK(e != NULL && e[0] == '\0');
        free(e);
    }

    /* Decryption vectors. */
    check_decrypt("HLWLEOODL R ", 3, "HELLO WORLD");
    check_decrypt("ACEBDF", 2, "ABCDEF");
    check_decrypt("", 2, "");
    ISO_CHECK(scytale_decrypt("HELLO", 0) == NULL);
    ISO_CHECK(scytale_decrypt("HI", 3) == NULL);

    /* Padding is stripped on decrypt. */
    {
        char *ct = scytale_encrypt("HELLO", 3);
        char *pt;
        ISO_CHECK(ct != NULL);
        pt = scytale_decrypt(ct, 3);
        ISO_CHECK(pt != NULL);
        ISO_CHECK_STR_EQ(pt, "HELLO");
        free(ct);
        free(pt);
    }

    /* No padding needed: length stays a multiple of the key. */
    {
        char *ct = scytale_encrypt("ABCDEF", 2);
        ISO_CHECK(ct != NULL);
        ISO_CHECK_EQ_UINT(strlen(ct), 6u);
        free(ct);
    }

    /* Round trips over assorted inputs and all valid keys. */
    {
        const char *texts[] = {"HELLO WORLD", "ABCDEF", "ABCDEF",
                               "The quick brown fox", "12345"};
        size_t keys[] = {3, 2, 3, 4, 2};
        size_t i;
        for (i = 0; i < 5; i++) {
            char *ct = scytale_encrypt(texts[i], keys[i]);
            char *pt;
            ISO_CHECK(ct != NULL);
            pt = scytale_decrypt(ct, keys[i]);
            ISO_CHECK(pt != NULL);
            ISO_CHECK_STR_EQ(pt, texts[i]);
            free(ct);
            free(pt);
        }
    }
    {
        const char *text = "The quick brown fox jumps over the lazy dog!";
        size_t n = strlen(text); /* ASCII, so byte count == char count */
        size_t key;
        for (key = 2; key <= n / 2; key++) {
            char *ct = scytale_encrypt(text, key);
            char *pt;
            ISO_CHECK(ct != NULL);
            pt = scytale_decrypt(ct, key);
            ISO_CHECK(pt != NULL);
            ISO_CHECK_STR_EQ(pt, text);
            free(ct);
            free(pt);
        }
    }

    /* Brute force finds the original key. */
    {
        char *ct = scytale_encrypt("HELLO WORLD", 3);
        size_t count = 0, i;
        int found = 0;
        ScytaleBrute *results;
        ISO_CHECK(ct != NULL);
        results = scytale_brute_force(ct, &count);
        ISO_CHECK(results != NULL);
        for (i = 0; i < count; i++) {
            if (results[i].key == 3) {
                found = 1;
                ISO_CHECK_STR_EQ(results[i].text, "HELLO WORLD");
            }
        }
        ISO_CHECK(found);
        scytale_brute_free(results, count);
        free(ct);
    }

    /* Brute force returns every candidate key (2..n/2). */
    {
        size_t count = 0;
        ScytaleBrute *results = scytale_brute_force("ABCDEFGHIJ", &count);
        ISO_CHECK_EQ_UINT(count, 4u); /* keys 2,3,4,5 */
        if (count == 4) {
            ISO_CHECK_EQ_UINT(results[0].key, 2u);
            ISO_CHECK_EQ_UINT(results[1].key, 3u);
            ISO_CHECK_EQ_UINT(results[2].key, 4u);
            ISO_CHECK_EQ_UINT(results[3].key, 5u);
        }
        scytale_brute_free(results, count);
    }

    /* Brute force on short text is empty. */
    {
        size_t count = 99;
        ScytaleBrute *r1 = scytale_brute_force("AB", &count);
        ISO_CHECK(r1 == NULL && count == 0);
        {
            size_t c2 = 99;
            ScytaleBrute *r2 = scytale_brute_force("ABC", &c2);
            ISO_CHECK(r2 == NULL && c2 == 0);
            scytale_brute_free(r2, c2);
        }
        scytale_brute_free(r1, count);
    }

    /* Multibyte UTF-8 characters stay intact through a round trip. */
    {
        const char *text = "caf\xc3\xa9 na\xc3\xafve"; /* "café naïve" */
        char *ct = scytale_encrypt(text, 3);
        char *pt;
        ISO_CHECK(ct != NULL);
        pt = scytale_decrypt(ct, 3);
        ISO_CHECK(pt != NULL);
        ISO_CHECK_STR_EQ(pt, text);
        free(ct);
        free(pt);
    }

    return ISO_TEST_RESULT();
}
