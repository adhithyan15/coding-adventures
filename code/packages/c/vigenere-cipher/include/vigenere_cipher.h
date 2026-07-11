/*
 * vigenere_cipher.h — the Vigenere polyalphabetic cipher with cryptanalysis,
 * in pure ISO C17. A faithful port of the Rust `vigenere-cipher` crate.
 * ===========================================================================
 *
 * The Vigenere cipher (Bellaso, 1553) shifts each plaintext letter by a
 * different amount taken from a repeating keyword. It resisted cryptanalysis
 * for 300 years until Kasiski (1863) and Friedman (1920s) broke it with two
 * statistical tools, both provided here:
 *
 *   Index of Coincidence — measures how "English-like" a letter distribution
 *     is; splitting the ciphertext by the true key length makes each group a
 *     Caesar cipher on English, whose IC (~0.067) stands out from random
 *     (~0.038). This reveals the KEY LENGTH.
 *
 *   Chi-squared — once the key length is known, each position-group is a
 *     Caesar cipher; the shift whose decrypted letter frequencies best match
 *     English (lowest chi-squared) is that key letter. This reveals the KEY.
 *
 * Character handling (matching the crate): letters are shifted preserving case;
 * non-alphabetic bytes pass through unchanged and do NOT advance the key
 * position; the key must be non-empty and all ASCII letters.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No libm (the statistics use only
 * +, -, *, /); DBL_MAX from <float.h> stands in for the crate's f64::INFINITY.
 */
#ifndef VIGENERE_CIPHER_H
#define VIGENERE_CIPHER_H

#include <stddef.h> /* size_t */

/* ---- core cipher ------------------------------------------------------ */

/* vigenere_key_valid — 1 iff `key` is non-empty and all ASCII letters. */
int vigenere_key_valid(const char *key);

/* vigenere_encrypt — encrypt `plaintext` with `key`. Returns a newly malloc'd
 * NUL-terminated string (caller frees), or NULL if the key is invalid (empty or
 * non-alphabetic) or on allocation failure. */
char *vigenere_encrypt(const char *plaintext, const char *key);

/* vigenere_decrypt — the inverse of vigenere_encrypt. Same return contract. */
char *vigenere_decrypt(const char *ciphertext, const char *key);

/* ---- cryptanalysis ---------------------------------------------------- */

/* vigenere_find_key_length — estimate the key length of `ciphertext` by the
 * Index of Coincidence, trying candidate lengths 2..max_length and returning
 * the smallest whose average IC is within 90% of the best. Returns 1 for text
 * with fewer than two letters. */
size_t vigenere_find_key_length(const char *ciphertext, size_t max_length);

/* vigenere_find_key — recover the key for a known `key_length` by chi-squared
 * frequency analysis of each position-group. Returns a newly malloc'd
 * NUL-terminated key of length `key_length` (caller frees), or NULL on
 * allocation failure. */
char *vigenere_find_key(const char *ciphertext, size_t key_length);

/* The recovered key and plaintext from vigenere_break (both malloc'd). */
typedef struct {
    char *key;
    char *plaintext;
} VigenereBreak;

/* vigenere_break — automatically break `ciphertext`: find the key length, then
 * the key, then decrypt. On success returns 1 and fills *out with malloc'd
 * strings (release with vigenere_break_free); returns 0 on allocation failure. */
int vigenere_break(const char *ciphertext, VigenereBreak *out);

/* vigenere_break_free — free the strings held by a VigenereBreak. Safe on NULL
 * and on an already-freed result. */
void vigenere_break_free(VigenereBreak *r);

#endif /* VIGENERE_CIPHER_H */
