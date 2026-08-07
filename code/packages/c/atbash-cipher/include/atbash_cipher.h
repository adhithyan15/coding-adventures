/*
 * atbash_cipher.h — the Atbash substitution cipher, in pure ISO C17. A faithful
 * port of the Rust `atbash-cipher` crate.
 * ===========================================================================
 *
 * Atbash is one of the oldest known ciphers (originally for the Hebrew
 * alphabet). It simply reverses the alphabet — A maps to Z, B to Y, ..., Z to
 * A — leaving case intact and passing every non-letter through unchanged:
 *
 *   Forward:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
 *   Reversed: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
 *
 * For a letter at position p (A=0 .. Z=25) the new position is `25 - p`.
 *
 * SELF-INVERSE. Applying Atbash twice returns the original text, because
 * `25 - (25 - p) = p`, so `atbash_decrypt` is literally `atbash_encrypt`.
 *
 * This port operates byte-by-byte: only ASCII letters are substituted, and all
 * other bytes (including the bytes of any UTF-8 sequence) pass through
 * unchanged — exactly matching the crate, which transforms only ASCII letters
 * and passes every other `char` through.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef ATBASH_CIPHER_H
#define ATBASH_CIPHER_H

/* atbash_char — apply the Atbash substitution to one byte: an ASCII letter is
 * reversed within its case, any other byte is returned unchanged. */
char atbash_char(char ch);

/* atbash_encrypt — return a newly malloc'd NUL-terminated string with Atbash
 * applied to every byte of `text` (caller frees), or NULL on allocation
 * failure. */
char *atbash_encrypt(const char *text);

/* atbash_decrypt — the inverse of atbash_encrypt. Since Atbash is self-inverse,
 * this is identical to atbash_encrypt; both exist for API clarity. */
char *atbash_decrypt(const char *text);

#endif /* ATBASH_CIPHER_H */
