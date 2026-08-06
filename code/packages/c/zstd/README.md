# zstd (C)

A pure ISO **C17** implementation of **Zstandard (ZStd)**, RFC 8878 — CMP07 in
the compression-algorithm series. A faithful port of the (corrected)
Rust `zstd` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library
only, plus a dependency on the sibling [`c/lzss`](../lzss/) package (CMP02)
for the LZ77 match-finder that produces sequences.

## What it is

Zstandard (Yann Collet, 2015; RFC 8878, 2021) pairs LZ77 back-references with
**FSE (Finite State Entropy)** — a table-based Asymmetric Numeral System
(tANS) that approaches the Shannon entropy limit in a single branch-free
pass, unlike Huffman's integer-bit-per-symbol coding. It is used by the Linux
kernel (btrfs/f2fs/squashfs/zram), Android OTA updates, macOS Software
Update, npm `.tar.zst` packages, and RocksDB/ClickHouse.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.   ← our dep
  CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP06 (Brotli,   2013) — DEFLATE successor; HTTP/2 standard.
  CMP07 (ZStd,     2016) — FSE + LZ77; Linux kernel / npm / macOS. ← YOU ARE HERE
```

## Educational subset (this package's deliberate simplification of RFC 8878)

| Feature        | Full ZStd                  | This package                    |
|-----------------|-----------------------------|----------------------------------|
| Literals        | Huffman or raw              | Raw only                         |
| Sequences FSE   | Custom + predefined tables  | **Predefined tables only**       |
| Repeat offsets  | R1/R2/R3 shortcuts          | Encoder: none, every offset coded in full. Decoder: full support (real `zstd` uses them constantly — see lessons.md Lesson 98) |
| Block types     | Raw / RLE / Compressed      | All three                        |
| Dictionary      | Yes                         | No                                |
| Checksums       | Optional (default on)       | Never emitted; skipped on read   |
| Window size     | Up to 8 MB                  | Fixed Single_Segment (no window descriptor written) |

Despite the simplifications, the wire format is **real**: output round-trips
through the actual `zstd` CLI in both directions, and this decoder correctly
parses frames the real CLI produces (multi-segment header, default checksum
trailer, repeat-offset sequences, etc.) — see TC-9 in the test suite. TC-9's
one fixed corpus is necessary but not sufficient evidence of that, though —
see Lesson 98 below; this package's test suite also includes an extra
high-sequence-count regression, and was fuzzed ad hoc against the real CLI
across varied inputs (random/periodic/constant/ramp byte patterns) before
being pushed.

## A hard-won warning for anyone touching the FSE codec

A repo-wide audit found that **every other language's `zstd` port in this
repository** independently reinvented the same wrong FSE sequences-section
codec (a fabricated table-spread algorithm, the wrong per-sequence field
read order, and a missing last-sequence state-update skip) plus the same
wrong Frame Header Descriptor checksum-flag bit. All of it was **invisible
to ordinary round-trip testing** — each port's own encoder and decoder
always agreed with each other — and was only caught by decompressing real
`zstd` CLI output (and having the CLI decompress the port's own output). See
`lessons.md` Lessons 95, 96, and 97 for the full forensic writeup.

This C port's algorithm is transcribed directly from the now-corrected
`code/packages/rust/zstd/src/lib.rs` (itself validated against the real
`zstd` CLI), and `src/zstd.c` carries the same doc comments explaining
exactly which convention is correct and why, at every spot the bug class
could recur. **If you modify the FSE codec, re-run the test suite — which
includes real `zstd` CLI interop (TC-9) — before trusting any change; a
self-consistent round trip proves nothing about wire-format conformance.**

## API

```c
#include "zstd.h"

uint8_t *comp; size_t comp_len;
zstd_compress(data, len, &comp, &comp_len);

uint8_t *back; size_t back_len;
zstd_decompress(comp, comp_len, &back, &back_len);   /* == data */
free(comp); free(back);
```

- `ZstdStatus`: `ZSTD_OK`, `ZSTD_ERR_ALLOC` (an allocation failed), or
  `ZSTD_ERR_FORMAT` (`zstd_decompress` only — malformed, truncated, or
  unsupported input).
- Output buffers are malloc'd (free them); `zstd_compress` always returns a
  non-NULL buffer (even empty input produces a minimal valid frame);
  `zstd_decompress` returns NULL only when the decompressed length is 0.

**Security — `zstd_decompress` is safe on adversarial input:**
- Frame_Content_Size is an untrusted hint, never used to pre-allocate the
  output buffer; the buffer only grows to fit bytes actually produced, with
  a 256 MB decompression-bomb cap checked incrementally (once per literal
  run and once per match copy, not just once per top-level block).
- A block claiming a size over 128 KB (`1 << 17`) is rejected outright.
- Every back-reference offset is bounds-checked against bytes already
  produced before the copy; out-of-range offsets are a hard error.
- Any Symbol Compression Mode other than Predefined is rejected — this
  decoder never builds an FSE table from untrusted wire bytes.

## Building & testing

```sh
sh tools/run.sh    # POSIX: compiles + runs the tests under every compiler found
```

Runs the 10 mandatory conformance cases from `code/specs/CMP07-zstd.md`
(TC-1..TC-10), plus extra round-trip and malformed-input-safety coverage.
TC-9 (and an extra high-sequence-count regression test) shell out to a real
`zstd` binary via `system()` if one is on `PATH`; they print a notice and
skip gracefully otherwise, rather than failing the suite.
