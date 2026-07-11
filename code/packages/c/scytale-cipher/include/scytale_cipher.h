/*
 * scytale_cipher.h — the Scytale transposition cipher, in pure ISO C17. A
 * faithful port of the Rust `scytale-cipher` crate.
 * ===========================================================================
 *
 * The Scytale (Sparta, ~700 BCE) is a transposition cipher: it does not replace
 * any characters, it only reorders them. Historically a strip of leather was
 * wound around a rod of a given diameter (the key), the message written along
 * the rod, then the strip unwound.
 *
 * Encryption writes the text row-by-row into a grid `key` columns wide, pads the
 * final row with spaces, then reads the grid column-by-column:
 *
 *   "HELLO WORLD", key 3     grid (4 rows x 3 cols)      read down the columns
 *                            H E L                       H L W L | E O O D | L _ R _
 *                            L O _                       -> "HLWLEOODL R "
 *                            W O R
 *                            L D _
 *
 * Decryption rebuilds the grid (handling a short final row, as arises during
 * brute force) and reads it row-by-row, then strips trailing pad spaces.
 *
 * UNICODE. Like the crate (which works on `char`s), this port transposes whole
 * CHARACTERS, not bytes: the input is split into UTF-8 character units and the
 * units are reordered, so multibyte characters stay intact. Malformed bytes are
 * treated as single-byte units, so any input round-trips losslessly.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef SCYTALE_CIPHER_H
#define SCYTALE_CIPHER_H

#include <stddef.h> /* size_t */

/* scytale_encrypt — encrypt `text` with `key`. Returns a newly malloc'd
 * NUL-terminated string (caller frees). An empty `text` yields an empty string;
 * otherwise returns NULL if the key is invalid (key < 2 or key > character
 * count) or on allocation failure. */
char *scytale_encrypt(const char *text, size_t key);

/* scytale_decrypt — the inverse of scytale_encrypt (trailing pad spaces are
 * stripped). Same return contract. */
char *scytale_decrypt(const char *text, size_t key);

/* One brute-force decryption: the tried key and the resulting plaintext. */
typedef struct {
    size_t key;
    char *text; /* malloc'd, NUL-terminated */
} ScytaleBrute;

/* scytale_brute_force — try every key from 2 to (character count)/2 and return
 * the decryptions. Sets *count to the number of results and returns a malloc'd
 * array (NULL and *count == 0 when the text has fewer than 4 characters, or on
 * allocation failure). Release with scytale_brute_free. */
ScytaleBrute *scytale_brute_force(const char *text, size_t *count);

/* scytale_brute_free — free a brute-force result array and its strings. */
void scytale_brute_free(ScytaleBrute *results, size_t count);

#endif /* SCYTALE_CIPHER_H */
