/*
 * lzss.h — the LZSS lossless compression algorithm, in pure ISO C17. A faithful
 * port of the Rust `lzss` crate (CMP02).
 * ===========================================================================
 *
 * LZSS (Storer & Szymanski, 1982) is the sliding-window LZ77 variant used by
 * DEFLATE, LZ4, and friends. At each position it searches the last
 * `window_size` bytes for the longest match of the upcoming bytes; a match of at
 * least `min_match` bytes becomes a back-reference token, otherwise a single
 * literal byte is emitted. Matches may overlap the cursor, so runs like
 * "AAAA..." encode as one short back-reference.
 *
 *   Literal(b)                 a single byte
 *   Match{offset, length}      copy `length` bytes from `offset` positions back
 *
 * Wire format (CMP02), big-endian: a u32 original length, a u32 block count,
 * then blocks of a 1-byte flag (bit b set => token b of the block is a match)
 * followed by each token's data (match = 2-byte offset + 1-byte length; literal
 * = 1 byte).
 *
 * ROBUSTNESS: `lzss_decode` / `lzss_decompress` operate on possibly-untrusted
 * bytes. Malformed match tokens (offset 0 or beyond the output) are skipped, the
 * block count is capped to the payload size, and the output is bounded by the
 * declared length — no out-of-bounds access and no unbounded allocation.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef LZSS_H
#define LZSS_H

#include <stddef.h> /* size_t */

#define LZSS_DEFAULT_WINDOW_SIZE 4096
#define LZSS_DEFAULT_MAX_MATCH 255
#define LZSS_DEFAULT_MIN_MATCH 3

typedef enum { LZSS_OK = 0, LZSS_ERR_ALLOC } LzssStatus;

/* One LZSS token: a literal byte or a back-reference match. */
typedef struct {
    int is_match;          /* 0 = literal, 1 = match */
    unsigned char literal; /* when !is_match */
    unsigned short offset; /* when is_match: distance back (1..window) */
    unsigned char length;  /* when is_match: bytes to copy */
} LzssToken;

/* ---- encode / decode -------------------------------------------------- */

/* lzss_encode — encode `data` (`len` bytes) into a token stream. On LZSS_OK
 * *out_tokens is a malloc'd array of *out_count tokens (free with free();
 * NULL when *out_count == 0). */
LzssStatus lzss_encode(const unsigned char *data, size_t len,
                       size_t window_size, size_t max_match, size_t min_match,
                       LzssToken **out_tokens, size_t *out_count);

/* lzss_decode — decode a token stream. If has_original_length is nonzero the
 * output is truncated to `original_length`. On LZSS_OK *out is malloc'd (free
 * with free(); NULL when *out_len == 0). */
LzssStatus lzss_decode(const LzssToken *tokens, size_t count,
                       int has_original_length, size_t original_length,
                       unsigned char **out, size_t *out_len);

/* ---- wire format ------------------------------------------------------ */

LzssStatus lzss_serialise(const LzssToken *tokens, size_t count,
                          size_t original_length, unsigned char **out,
                          size_t *out_len);

LzssStatus lzss_deserialise(const unsigned char *data, size_t len,
                            LzssToken **out_tokens, size_t *out_count,
                            size_t *out_original_length);

/* ---- one-shot compress / decompress (default parameters) -------------- */

LzssStatus lzss_compress(const unsigned char *data, size_t len,
                         unsigned char **out, size_t *out_len);

LzssStatus lzss_decompress(const unsigned char *data, size_t len,
                           unsigned char **out, size_t *out_len);

#endif /* LZSS_H */
