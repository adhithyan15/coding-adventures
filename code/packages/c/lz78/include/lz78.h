/*
 * lz78.h — the LZ78 (1978) lossless compression algorithm, in pure ISO C17. A
 * faithful port of the Rust `lz78` crate (CMP01).
 * ===========================================================================
 *
 * LZ78 (Lempel & Ziv, 1978) builds an explicit trie dictionary of byte
 * sequences as it encodes. Encoder and decoder build the SAME dictionary
 * independently, so no dictionary is transmitted. Each token is a
 * (dict_index, next_char) pair — dict_index is the ID of the longest matching
 * dictionary prefix (0 for a literal), next_char is the byte that follows.
 *
 * Wire format (CMP01), big-endian:
 *   bytes 0..3  original length (u32)
 *   bytes 4..7  token count (u32)
 *   bytes 8..   token_count * 4:  [dict_index u16][next_char u8][0x00]
 *
 * The `TrieCursor` is a reusable byte-at-a-time trie walker (the crate advertises
 * it for both LZ78 and LZW).
 *
 * ROBUSTNESS: `lz78_decode` / `lz78_decompress` operate on possibly-untrusted
 * bytes. Where the Rust crate would panic on an out-of-range dictionary index or
 * hang on a cyclic one, this port bounds-checks and stops safely; for
 * well-formed streams the output is identical.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef LZ78_H
#define LZ78_H

#include <stddef.h> /* size_t */

typedef enum { LZ78_OK = 0, LZ78_ERR_ALLOC } Lz78Status;

/* One LZ78 token. dict_index 0 is a literal; next_char 0 is also the flush
 * sentinel when the input ends mid-match. */
typedef struct {
    unsigned short dict_index;
    unsigned char next_char;
} Lz78Token;

/* ---- TrieCursor: a byte-at-a-time trie walker ------------------------- */

typedef struct Lz78TrieCursor Lz78TrieCursor;

/* lz78_cursor_new — a cursor over an empty trie (positioned at the root), or
 * NULL on allocation failure. Release with lz78_cursor_free. */
Lz78TrieCursor *lz78_cursor_new(void);
void lz78_cursor_free(Lz78TrieCursor *c);

/* lz78_cursor_step — follow the child edge for `byte`; returns 1 and advances if
 * it exists, else 0 without moving. */
int lz78_cursor_step(Lz78TrieCursor *c, unsigned char byte);

/* lz78_cursor_insert — add a child edge for `byte` at the current position with
 * dictionary id `dict_id` (does not move the cursor). Returns 1, or 0 on
 * allocation failure. */
int lz78_cursor_insert(Lz78TrieCursor *c, unsigned char byte,
                       unsigned short dict_id);

/* lz78_cursor_reset — return the cursor to the root. */
void lz78_cursor_reset(Lz78TrieCursor *c);

/* lz78_cursor_dict_id — dictionary id at the current position (0 at the root). */
unsigned short lz78_cursor_dict_id(const Lz78TrieCursor *c);

/* lz78_cursor_at_root — 1 if the cursor is at the root. */
int lz78_cursor_at_root(const Lz78TrieCursor *c);

/* ---- encode / decode -------------------------------------------------- */

/* lz78_encode — encode `data` (`len` bytes) into a token stream. On LZ78_OK
 * *out_tokens is a malloc'd array of *out_count tokens (free with free();
 * *out_tokens may be NULL when *out_count == 0). `max_dict_size` caps the
 * dictionary (use 65536). */
Lz78Status lz78_encode(const unsigned char *data, size_t len,
                       size_t max_dict_size, Lz78Token **out_tokens,
                       size_t *out_count);

/* lz78_decode — decode a token stream. If has_original_length is nonzero the
 * output is truncated to `original_length` (stripping the flush sentinel);
 * otherwise all bytes are emitted. On LZ78_OK *out_data is malloc'd (free with
 * free(); may be NULL when *out_len == 0). */
Lz78Status lz78_decode(const Lz78Token *tokens, size_t token_count,
                       int has_original_length, size_t original_length,
                       unsigned char **out_data, size_t *out_len);

/* ---- one-shot compress / decompress (wire format) --------------------- */

/* lz78_compress — encode `data` and serialise to the CMP01 wire format. */
Lz78Status lz78_compress(const unsigned char *data, size_t len,
                         size_t max_dict_size, unsigned char **out,
                         size_t *out_len);

/* lz78_decompress — the inverse of lz78_compress. */
Lz78Status lz78_decompress(const unsigned char *data, size_t len,
                           unsigned char **out, size_t *out_len);

#endif /* LZ78_H */
