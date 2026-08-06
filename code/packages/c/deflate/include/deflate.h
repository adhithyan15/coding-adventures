/*
 * deflate.h — DEFLATE lossless compression (RFC 1951), in pure ISO C17. A
 * faithful port of the Rust `deflate` crate (CMP05).
 * ===========================================================================
 *
 * DEFLATE (Phil Katz, 1989; specified as RFC 1951 by L. Peter Deutsch, 1996) is
 * the compression layer inside ZIP, gzip, PNG, and zlib. It composes two
 * earlier techniques from this series:
 *
 *   1. LZSS tokenization (CMP02, `c/lzss`) — replace repeated substrings with
 *      back-references into a 32768-byte sliding window (the full RFC 1951
 *      window; offset 1..32768, length 3..255).
 *   2. Huffman coding (CMP04) — entropy-code the resulting Literal/Match token
 *      stream. `deflate_compress` builds BOTH a fixed-table encoding (RFC 1951
 *      §3.2.6, no table transmitted) and a dynamic, data-adapted encoding
 *      (length-limited to 15 bits via the package-merge algorithm), then emits
 *      whichever is smaller in exact bits.
 *
 * Wire format — standard RFC 1951 raw DEFLATE, no envelope:
 *
 *     [3 bits]  block header: BFINAL=1, BTYPE=01 (fixed) or 10 (dynamic)
 *     [...]     (dynamic only) HLIT/HDIST/HCLEN + code-length trees, RLE'd
 *     [...]     token stream: literals / (length,distance) matches
 *     [n bits]  end-of-block: LL symbol 256
 *
 * `deflate_compress` always emits a single BFINAL=1 block and never produces a
 * stream larger than the fixed-only encoding. `deflate_decompress` (RFC 1951
 * `inflate`) reads ALL THREE block types — stored (BTYPE=00), fixed (01), and
 * dynamic (10) — so it decodes streams from `zlib`, `gzip`, and real ZIP/PNG
 * files, not only its own output. This asymmetry ("encode conservatively,
 * decode liberally") is why the decoder's length/distance tables cover the
 * FULL RFC 1951 alphabet (length symbol 285, distance codes 0-29 reaching
 * 32768) even though our own encoder never needs the top entries.
 *
 * ROBUSTNESS: `deflate_decompress` treats its input as untrusted bytes. Output
 * is capped at DEFLATE_MAX_OUTPUT (256 MiB) to bound decompression-bomb blast
 * radius, every back-reference distance is checked against the bytes decoded
 * so far, every Huffman/length/distance symbol is range-checked before table
 * lookup, and no allocation is sized directly from an attacker-controlled
 * declared length. Malformed input yields DEFLATE_ERR_MALFORMED rather than
 * undefined behaviour.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Standard library only.
 *
 * Dependency: `c/lzss` (CMP02) supplies the LZSS tokenizer (`lzss_encode`,
 * `LzssToken`) used for the LZ pass; DEFLATE builds its own Huffman coder
 * (fixed tables need no tree, and the dynamic-code alphabets are specific to
 * DEFLATE), so there is no separate huffman-tree dependency.
 */
#ifndef DEFLATE_H
#define DEFLATE_H

#include <stddef.h> /* size_t */

typedef enum {
    DEFLATE_OK = 0,
    DEFLATE_ERR_ALLOC,     /* out of memory */
    DEFLATE_ERR_MALFORMED  /* input is not a well-formed RFC 1951 stream */
} DeflateStatus;

/* deflate_compress — compress `len` bytes of `data` into a standard RFC 1951
 * raw DEFLATE stream: a single final block, fixed (BTYPE=01) or dynamic
 * (BTYPE=10) Huffman, whichever is smaller in exact bits. Decodable by any
 * conforming inflater (this library's `deflate_decompress`, zlib, gzip, unzip).
 *
 * On DEFLATE_OK, *out is a malloc'd buffer of *out_len bytes (free with
 * deflate_free() or free()); *out is NULL only when allocation never
 * happened, which cannot occur on success (the shortest possible output, for
 * empty input, is still 2 bytes). Returns DEFLATE_ERR_ALLOC on allocation
 * failure, in which case *out is NULL and *out_len is 0. Always succeeds
 * (returns DEFLATE_OK) for well-formed calls given enough memory — there is no
 * input that `deflate_compress` rejects. */
DeflateStatus deflate_compress(const unsigned char *data, size_t len,
                               unsigned char **out, size_t *out_len);

/* deflate_decompress — decode a raw RFC 1951 DEFLATE bit stream (the standard
 * `inflate`). Reads all three block types (stored / fixed Huffman / dynamic
 * Huffman) across as many blocks as the stream contains, stopping at the
 * BFINAL=1 block. Rejects malformed streams (truncated input, invalid Huffman
 * codes, out-of-range length/distance symbols, back-references beyond the
 * output produced so far, stored-block LEN/NLEN mismatches) with
 * DEFLATE_ERR_MALFORMED, and caps output at DEFLATE_MAX_OUTPUT bytes to bound
 * decompression bombs.
 *
 * On DEFLATE_OK, *out is a malloc'd buffer of *out_len bytes (free with
 * deflate_free() or free(); NULL when *out_len == 0). On any error *out is
 * NULL and *out_len is 0. */
DeflateStatus deflate_decompress(const unsigned char *data, size_t len,
                                 unsigned char **out, size_t *out_len);

/* deflate_free — free a buffer returned by deflate_compress / deflate_decompress
 * (via *out). Equivalent to free(); provided so callers never need to track
 * which allocator produced a buffer crossing this library's API boundary. A
 * no-op on NULL. */
void deflate_free(unsigned char *buf);

/* Upper bound on decompressed output size (256 MiB). A highly compressible
 * stream can expand ~1000:1, so a small malicious input could otherwise
 * exhaust memory; this caps the blast radius while comfortably exceeding any
 * legitimate single ZIP entry / PNG scanline / gzip member this library is
 * expected to handle in one call. Callers needing more must stream. */
#define DEFLATE_MAX_OUTPUT ((size_t)256 * 1024 * 1024)

#endif /* DEFLATE_H */
