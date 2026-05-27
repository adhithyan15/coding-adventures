# Changelog — rans

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this package adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-05-26

### Added

- **`AnsTable`** — precomputed rANS frequency table for a fixed alphabet (up to
  256 symbols). Accepts raw (unnormalized) counts; normalizes to `M = 2^k` using
  the largest-remainder method so that every symbol has `freq ≥ 1`. Builds a flat
  M-entry O(1) decode lookup table `(decode_sym, decode_freq, decode_cumfreq)`.
- **`RansEncoder`** — streaming rANS encoder. Symbols are pushed in **reverse**
  logical order via `put(symbol)`; `finish()` returns the compressed byte stream
  as a 4-byte big-endian initial state followed by renormalization bytes.
- **`RansDecoder`** — streaming rANS decoder. Constructed from the byte stream
  produced by `RansEncoder::finish()`; symbols are extracted in forward order via
  `get()`.
- **`is_exhausted()`** on `RansDecoder` — returns `true` when all input bytes have
  been consumed (the final state may still hold symbols).
- **25 unit tests** covering:
  - `AnsTable` construction invariants: M is a power of two, frequencies sum to M,
    decode table slots cover full `[0, M)` range, alphabet size matches.
  - Round-trips for 1, 2, 4, 256-symbol alphabets (uniform and skewed).
  - Long sequences (128 symbols), single-symbol sequences, adjacent symbol pairs.
  - Unequal count normalization.
  - Compression ratio sanity check (highly skewed alphabet compresses well below
    uncompressed size).
  - Determinism regression: identical inputs always produce identical byte streams.
  - Error cases: empty alphabet, all-zero counts, > 256 symbols, data too short.
- **4 doc-tests** — in-doc examples for `AnsTable`, `RansEncoder`, `RansDecoder`,
  and the top-level module.
- **`VERSION = "0.1.0"`** constant.

### Implementation notes

- Zero external dependencies.
- The finish-time state serialization bug (MSB/LSB reversal of the 4-byte state
  header) was caught and fixed during initial testing: the state bytes must be
  pushed in LSB-first order before the overall `pending` buffer is reversed, so
  they emerge MSB-first (big-endian) in the final output.
- Largest-remainder normalization with zero-frequency priority: any symbol that
  rounds down to `freq=0` is unconditionally given one count before distributing
  the remainder by fractional part — this guarantees every input-alphabet symbol
  is reachable from the decode table.
