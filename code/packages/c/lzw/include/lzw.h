/*
 * lzw.h — LZW compression with variable-width codes, in pure ISO C17. A faithful
 * port of the Rust `lzw` crate.
 * ===========================================================================
 *
 * LZW builds a dictionary of byte sequences as it goes, starting from the 256
 * single bytes and adding one entry per step, so common substrings collapse to
 * a single code. Codes start at 9 bits and grow (up to 16) as the dictionary
 * fills; the stream begins with CLEAR (256) and ends with STOP (257), and a
 * dictionary-full condition emits another CLEAR and starts over.
 *
 * The wire format is a 4-byte big-endian original length followed by the
 * LSB-first bit-packed code stream (the length lets the decoder trim the
 * zero-padding of the final partial byte).
 *
 * `lzw_compress` and `lzw_decompress` allocate their output with malloc and
 * report the length through an out-parameter; the caller frees it.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef LZW_H
#define LZW_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* lzw_compress — compress `len` bytes of `data`. On success writes a malloc'd
 * buffer to *out and its length to *out_len, and returns 1. Returns 0 on
 * allocation failure. Caller frees *out. */
int lzw_compress(const uint8_t *data, size_t len, uint8_t **out, size_t *out_len);

/* lzw_decompress — reverse lzw_compress. On success writes a malloc'd buffer to
 * *out and its length to *out_len, and returns 1. Returns 0 on a malformed
 * stream or allocation failure. Caller frees *out. */
int lzw_decompress(const uint8_t *data, size_t len, uint8_t **out,
                   size_t *out_len);

#endif /* LZW_H */
