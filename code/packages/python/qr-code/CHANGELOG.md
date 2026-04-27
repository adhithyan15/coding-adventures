# Changelog — coding-adventures-qr-code

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-24

### Added

- **`encode(data, *, level, version, mode)`** — Main entry point.  Encodes
  any UTF-8 string (or `bytes`) into a `ModuleGrid` following ISO/IEC
  18004:2015.  Supports all four ECC levels (L/M/Q/H) and auto-selects the
  minimum version 1–40 that fits the input.

- **`encode_to_scene(data, *, level, version, mode, config)`** — Convenience
  wrapper that calls `encode()` and passes the result through
  `barcode_2d.layout()` to produce a pixel-ready `PaintScene`.

- **Encoding modes** — Numeric, alphanumeric (45-char QR alphabet), and
  byte (UTF-8).  Mode is auto-selected (most compact that covers the input)
  or can be forced by the caller.

- **Version selection** — `select_version()` iterates versions 1–40 and
  returns the smallest that fits the bit stream (mode indicator + char-count
  field + data bits, rounded to bytes).

- **Reed-Solomon ECC** — `rs_encode()` using the b=0 generator convention
  (`g(x) = ∏(x + αⁱ)` for i=0..n−1) and GF(256) with primitive polynomial
  `0x11D`.  Generators pre-built for all degrees used by the 40×4 capacity
  table (7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30).

- **Block splitting + interleaving** — `compute_blocks()` splits data into
  ISO-spec group-1 / group-2 blocks; `interleave_blocks()` round-robins data
  CWs then ECC CWs across blocks to spread burst errors.

- **Grid construction** — Finder patterns (7×7), separators, timing strips,
  alignment patterns (version-dependent), format info reservation, version
  info reservation (v7+), and the always-dark module.

- **Zigzag data placement** — `place_bits()` fills non-reserved modules in
  the two-column bottom-right-to-top-left zigzag, skipping the col-6 timing
  strip.

- **Masking** — `apply_mask()` implements all 8 ISO mask patterns.
  `compute_penalty()` scores a candidate grid with the 4-rule penalty system.
  The encoder evaluates all 8 masks and selects the one with lowest score.

- **Format information** — `compute_format_bits()` produces the 15-bit
  BCH(15,5) word (generator `0x537`, XOR mask `0x5412`).
  `write_format_info()` places both redundant copies with the correct
  MSB-first bit ordering documented in `lessons.md` (2026-04-23).

- **Version information** — `compute_version_bits()` produces the 18-bit
  BCH(18,6) word (generator `0x1F25`).  `write_version_info()` places both
  6×3 blocks for v7+.

- **Type annotations** — Full `py.typed` PEP 561 marker; all functions have
  explicit parameter and return types.

- **Test suite** — `tests/test_qr_code.py` with >90% line coverage.  Tests
  cover all ECC levels, all three encoding modes, structural pattern
  correctness, format/version information, masking idempotency, penalty
  scoring, error cases, and regression fingerprints.

### Implementation notes

- The implementation follows the TypeScript reference at
  `code/packages/typescript/qr-code/src/index.ts` with idiomatic Python
  adaptations (dataclasses, `tuple[...]` immutable types for public API,
  `list[list[bool]]` mutable working grid).

- Format information bit ordering was corrected per the lesson recorded in
  `lessons.md` (2026-04-23): Copy 1 row 8 uses MSB-first (bit 14 at col 0),
  Copy 1 col 8 uses LSB-first (bit 0 at row 0).

- `ALPHANUM_CHARS` is exactly 45 characters (`"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"`).
  An embedded newline that crept in during initial scaffolding was removed
  before the first commit.

### Dependencies

- `coding-adventures-barcode-2d` — `ModuleGrid`, `layout()`, `PaintScene`
- `coding-adventures-gf256` — GF(256) `multiply()` and `ALOG` table
- `coding-adventures-paint-instructions` — `PaintScene` type (transitive)
