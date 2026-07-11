/*
 * caesar_cipher.h — the Caesar cipher (encrypt / decrypt / ROT13) plus a
 * frequency-analysis attack, in pure ISO C17. A faithful port of the Rust
 * `caesar-cipher` crate.
 * ===========================================================================
 *
 * The Caesar cipher replaces each letter of the alphabet with the letter a
 * fixed number of positions further along, wrapping around at the end:
 *
 *     Plain:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
 *     Shift 3: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
 *
 * Only ASCII letters are shifted; digits, punctuation, and spaces pass through
 * unchanged. Case is preserved. Decryption is encryption by the negative shift.
 *
 * C has no growable string type, so the transforming functions write their
 * NUL-terminated result into a caller-provided buffer and report how many
 * characters they produced (or -1 if the buffer was too small). Because the
 * cipher is a 1:1 character mapping, an output buffer of `strlen(text) + 1`
 * bytes is always sufficient.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef CAESAR_CIPHER_H
#define CAESAR_CIPHER_H

#include <stddef.h> /* size_t */

/* caesar_encrypt — shift every ASCII letter of `text` forward by `shift`
 * positions (mod 26; negative shifts allowed). Writes a NUL-terminated result
 * into `out` (total capacity `out_size`, including the NUL).
 * Returns the number of characters written (excluding the NUL), or -1 if `out`
 * is too small (in which case `out` is left untouched). */
long caesar_encrypt(const char *text, int shift, char *out, size_t out_size);

/* caesar_decrypt — the inverse of caesar_encrypt (shift backwards). Same
 * contract as caesar_encrypt. */
long caesar_decrypt(const char *text, int shift, char *out, size_t out_size);

/* caesar_rot13 — encrypt with a shift of 13. ROT13 is its own inverse. */
long caesar_rot13(const char *text, char *out, size_t out_size);

/* caesar_letter_counts — tally how many times each letter A–Z appears in
 * `text`, case-insensitively, into counts[0..25] (A=0 … Z=25). Non-letters are
 * ignored. */
void caesar_letter_counts(const char *text, size_t counts[26]);

/* caesar_chi_squared — the chi-squared statistic of `text`'s letter
 * distribution against standard English letter frequencies. Lower means a
 * better fit to English. Returns a very large sentinel when `text` contains no
 * letters (no frequency signal). */
double caesar_chi_squared(const char *text);

/* caesar_frequency_analysis — break a Caesar cipher without knowing the shift:
 * decrypt with every shift 1..25 and pick the candidate whose letter
 * distribution best fits English (lowest chi-squared). Writes the winning
 * plaintext into `out` (capacity `out_size`) and returns the winning shift
 * (1..25), or -1 if `out` is too small. */
int caesar_frequency_analysis(const char *ciphertext, char *out,
                              size_t out_size);

#endif /* CAESAR_CIPHER_H */
