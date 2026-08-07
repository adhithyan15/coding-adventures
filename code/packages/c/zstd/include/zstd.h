/*
 * zstd.h — Zstandard (ZStd) lossless compression, RFC 8878, in pure ISO C17.
 * ===========================================================================
 * CMP07 in the compression-algorithm series. A faithful port of the (fixed)
 * Rust `zstd` crate, itself an educational SUBSET of RFC 8878:
 *
 *   CMP00 (LZ77)     — Sliding-window back-references
 *   CMP01 (LZ78)     — Explicit dictionary (trie)
 *   CMP02 (LZSS)     — LZ77 + flag bits                    ← `c/lzss` (our dep)
 *   CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
 *   CMP04 (Huffman)  — Entropy coding
 *   CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
 *   CMP06 (Brotli)   — DEFLATE + context modelling + static dict
 *   CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed        ← this package
 *
 * ZStd (Yann Collet, 2015; RFC 8878, 2021) pairs LZ77 back-references with
 * **FSE (Finite State Entropy)** — a table-based Asymmetric Numeral System
 * (tANS, Jarek Duda 2013) that approaches the Shannon entropy limit in a
 * single branch-free pass, unlike Huffman's integer-bit-per-symbol coding.
 *
 * WHAT THIS PORT IMPLEMENTS (the repo's deliberate educational subset):
 *   - Raw literals only (no Huffman literal coding).
 *   - Sequences use PREDEFINED FSE tables only (RFC 8878 Appendix B), never
 *     custom per-frame tables.
 *   - The ENCODER never emits repeat-offset (R1/R2/R3) shortcuts — every
 *     offset it writes is coded in full. The DECODER, however, fully
 *     understands repeat-offset sequences (RFC 8878 §3.1.1.3.2.1.1,
 *     including the Literals_Length==0 shift rule) — real `zstd` encoders
 *     use them constantly, and a decoder that didn't accept them would fail
 *     to decode a large fraction of real-world `.zst` files despite passing
 *     every self-consistency test. See lessons.md Lesson 98.
 *   - No dictionary support, no content checksum emitted (though a real
 *     zstd frame's checksum, if present, is correctly skipped on read).
 *   - Frame is always Single_Segment with an 8-byte Frame_Content_Size.
 *   - Blocks capped at 128 KB, exactly as RFC 8878 requires as a maximum.
 *
 * Despite the simplifications, the wire format is REAL: output round-trips
 * through the actual `zstd` CLI in both directions (see tests/zstd_test.c
 * TC-9), and input from the real CLI decompresses correctly here.
 *
 * ── A hard-won warning for anyone porting or modifying this file ──────────
 * A repo-wide audit (2026-08-03/04) found that EVERY existing `zstd` port in
 * this repository — Rust, Go, Python, TypeScript, Swift, Dart, Elixir, Lua,
 * Perl, C#, F#, Haskell, Java, Kotlin — independently reinvented the SAME
 * wrong FSE sequences-section codec (a fabricated two-pass table-spread
 * algorithm, the wrong per-sequence field read order, and a missing
 * last-sequence state-update skip) and the SAME wrong Frame Header
 * Descriptor checksum-flag bit. Every one of those bugs was *invisible* to
 * ordinary round-trip testing — encoder and decoder always agreed with
 * themselves — and was only caught by decompressing real `zstd` CLI output
 * (and having the CLI decompress ours). See `lessons.md` Lesson 95, 96, 97
 * for the full forensic writeup. This C port's algorithm is transcribed
 * directly from the now-corrected `code/packages/rust/zstd/src/lib.rs`
 * (validated against the real `zstd` CLI); if you touch the FSE codec here,
 * re-run TC-9 (`sh tools/run.sh`, requires a real `zstd` binary on PATH)
 * before trusting any change.
 *
 * Frame layout (RFC 8878 §3):
 * ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
 * │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
 * │ 4B LE  │ 1B  │ 8 B LE (always, here)│ ...    │ 4B (skip-only)   │
 * └────────┴─────┴──────────────────────┴────────┴──────────────────┘
 *
 * Block header (3 bytes, little-endian bitfield):
 *   bit 0      = Last_Block flag
 *   bits [2:1] = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
 *   bits [23:3] = Block_Size (21-bit)
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors, via the shared iso-harness. Standard
 * library only (uses `system()` in the test suite for real CLI interop —
 * that test degrades gracefully, rather than failing, when no `zstd` binary
 * is reachable on PATH).
 *
 * SECURITY (decompression is presumed hostile input):
 *   - Frame_Content_Size is an untrusted hint — never pre-allocated; the
 *     output buffer only ever grows to fit bytes actually produced.
 *   - Total decompressed output is capped at 256 MB (a decompression-bomb
 *     guard checked incrementally, once per literal run and once per match
 *     copy — not just once per top-level block).
 *   - A block's declared wire size is rejected outright if it claims more
 *     than 128 KB (1 << 17): `Block_Size > (1 << 17)` is malformed.
 *   - Every back-reference offset is bounds-checked against bytes already
 *     produced before the copy; out-of-range offsets are a hard error, not
 *     a truncated or garbage read.
 *   - Symbol Compression Modes other than Predefined (0) are rejected — this
 *     port never builds a custom FSE table from untrusted wire bytes.
 */
#ifndef ZSTD_H
#define ZSTD_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* Status codes, mirroring `c/lzss`'s LzssStatus:
 *   ZSTD_OK          success; *output is malloc'd (free it), or NULL if
 *                    *output_len == 0.
 *   ZSTD_ERR_ALLOC   an allocation failed partway through; no output is
 *                    returned (*output is NULL, *output_len is 0).
 *   ZSTD_ERR_FORMAT  (zstd_decompress only) the input is not a well-formed
 *                    or supported ZStd frame — bad magic, truncated data,
 *                    an unsupported feature (Huffman literals, non-Predefined
 *                    FSE tables, reserved block type), or a wire value that
 *                    fails a safety check (oversized block, out-of-range
 *                    offset, decompression-bomb budget exceeded).
 */
typedef enum { ZSTD_OK = 0, ZSTD_ERR_ALLOC, ZSTD_ERR_FORMAT } ZstdStatus;

/* zstd_compress — compress `input` (`input_len` bytes) into a ZStd frame.
 * On ZSTD_OK, *output is a malloc'd buffer of *output_len bytes (free with
 * free(); never NULL — even empty input produces a minimal valid frame). The
 * output is a real RFC 8878 frame decompressible by the `zstd` CLI. */
ZstdStatus zstd_compress(const uint8_t *input, size_t input_len,
                          uint8_t **output, size_t *output_len);

/* zstd_decompress — decompress a ZStd frame (possibly untrusted / from the
 * real `zstd` CLI). On ZSTD_OK, *output is malloc'd (free with free(); NULL
 * when *output_len == 0). See the security notes above: this function is
 * safe to call on adversarial input — it never over-reads the input buffer
 * and never grows the output past the 256 MB decompression-bomb cap. */
ZstdStatus zstd_decompress(const uint8_t *input, size_t input_len,
                            uint8_t **output, size_t *output_len);

#endif /* ZSTD_H */
