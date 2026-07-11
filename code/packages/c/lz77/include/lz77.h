/*
 * lz77.h — LZ77 sliding-window compression, in pure ISO C17. A faithful port of
 * the Rust `lz77` crate.
 * ===========================================================================
 *
 * LZ77 compresses by replacing repeated byte runs with backreferences into what
 * has already been seen. The output is a stream of tokens, each a triple:
 *
 *     (offset, length, next_char)
 *
 * meaning "copy `length` bytes from `offset` bytes back, then append
 * next_char". A literal is just (0, 0, byte). Decoding replays the tokens,
 * copying byte-by-byte so overlapping matches (offset < length) expand
 * correctly.
 *
 * `compress` / `decompress` bundle encoding with a compact serialisation (a
 * big-endian u32 token count followed by 4 bytes per token). Every function that
 * produces a buffer allocates it with malloc and reports the length through an
 * out-parameter; the caller frees it.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef LZ77_H
#define LZ77_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t */

typedef struct {
    uint16_t offset;
    uint8_t length;
    uint8_t next_char;
} lz77_token;

/* Typical parameters (as used by the crate's tests). */
#define LZ77_DEFAULT_WINDOW 4096
#define LZ77_DEFAULT_MAX_MATCH 255
#define LZ77_DEFAULT_MIN_MATCH 3

/* lz77_encode — tokenise `data`. On success writes a malloc'd token array to
 * *tokens_out and its length to *count_out (both may be NULL/0 for empty input)
 * and returns 1. Returns 0 on allocation failure. Caller frees *tokens_out. */
int lz77_encode(const uint8_t *data, size_t len, size_t window_size,
                size_t max_match, size_t min_match, lz77_token **tokens_out,
                size_t *count_out);

/* lz77_decode — reconstruct the bytes from `tokens`, starting from
 * `initial` (may be NULL/0). Writes a malloc'd buffer to *out and its length to
 * *out_len; returns 1, or 0 on allocation failure. Caller frees *out. */
int lz77_decode(const lz77_token *tokens, size_t count, const uint8_t *initial,
                size_t initial_len, uint8_t **out, size_t *out_len);

/* lz77_serialise / lz77_deserialise — tokens ⇄ bytes (u32 BE count header, then
 * 4 bytes per token). Return 1, or 0 on allocation failure. Caller frees *out /
 * *tokens_out. */
int lz77_serialise(const lz77_token *tokens, size_t count, uint8_t **out,
                   size_t *out_len);
int lz77_deserialise(const uint8_t *data, size_t len, lz77_token **tokens_out,
                     size_t *count_out);

/* lz77_compress — encode then serialise. lz77_decompress — deserialise then
 * decode (from an empty initial buffer). Return 1, or 0 on allocation failure.
 * Caller frees *out. */
int lz77_compress(const uint8_t *data, size_t len, size_t window_size,
                  size_t max_match, size_t min_match, uint8_t **out,
                  size_t *out_len);
int lz77_decompress(const uint8_t *data, size_t len, uint8_t **out,
                    size_t *out_len);

#endif /* LZ77_H */
