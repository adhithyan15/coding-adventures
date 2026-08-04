/*
 * huffman_compression.h — Huffman compression, in pure ISO C17. A faithful port
 * of the Rust `huffman-compression` crate (the CMP04 wire format).
 * ===========================================================================
 *
 * Huffman coding assigns each byte a variable-length bit code — short codes for
 * frequent bytes, long codes for rare ones — so the total size shrinks. This
 * port uses CANONICAL codes: the codes are fully determined by the per-symbol
 * code LENGTHS, so the compressed stream only has to carry a lengths table (not
 * the whole tree) for the decompressor to rebuild identical codes.
 *
 * Wire format (big-endian header):
 *     [0..4]      original length   (u32)
 *     [4..8]      symbol count N    (u32)
 *     [8..8+2N]   lengths table     (N × [symbol byte, code-length byte],
 *                                    sorted by (length, symbol))
 *     [8+2N..]    bit stream        (canonical codes, packed LSB-first)
 *
 * `huffman_compress` / `huffman_decompress` allocate their output with malloc
 * and report the length through an out-parameter; the caller frees it.
 *
 * (This is a faithful implementation of the Huffman algorithm; the concrete
 * code lengths may differ from the Rust crate when byte frequencies tie, but a
 * round-trip always reproduces the input exactly.)
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HUFFMAN_COMPRESSION_H
#define HUFFMAN_COMPRESSION_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* huffman_compress — compress `len` bytes of `data`. On success writes a
 * malloc'd buffer to *out and its length to *out_len, and returns 1. Returns 0
 * on allocation failure. Caller frees *out. */
int huffman_compress(const uint8_t *data, size_t len, uint8_t **out,
                     size_t *out_len);

/* huffman_decompress — reverse huffman_compress. On success writes a malloc'd
 * buffer to *out and its length to *out_len, and returns 1. Returns 0 on a
 * malformed stream or allocation failure. Caller frees *out. */
int huffman_decompress(const uint8_t *data, size_t len, uint8_t **out,
                       size_t *out_len);

#endif /* HUFFMAN_COMPRESSION_H */
