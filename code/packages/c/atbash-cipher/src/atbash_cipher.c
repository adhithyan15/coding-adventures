/*
 * atbash_cipher.c — implementation of the Atbash cipher (see atbash_cipher.h).
 * A faithful port of the Rust `atbash-cipher` crate.
 */
#include "atbash_cipher.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* strlen */

char atbash_char(char ch) {
    unsigned char c = (unsigned char)ch;
    if (c >= 'A' && c <= 'Z') {
        /* position 0..25, reversed to 25..0, back to a letter */
        return (char)('A' + (25 - (c - 'A')));
    }
    if (c >= 'a' && c <= 'z') {
        return (char)('a' + (25 - (c - 'a')));
    }
    return ch; /* non-letter passes through unchanged */
}

char *atbash_encrypt(const char *text) {
    size_t len = strlen(text), i;
    char *result = malloc(len + 1); /* len < SIZE_MAX (it is a strlen) */
    if (!result) {
        return NULL;
    }
    for (i = 0; i < len; i++) {
        result[i] = atbash_char(text[i]);
    }
    result[len] = '\0';
    return result;
}

char *atbash_decrypt(const char *text) {
    /* Atbash is self-inverse: decryption is encryption. */
    return atbash_encrypt(text);
}
