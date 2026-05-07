# Changelog

All notable changes to `coding_adventures_aztec_code` are documented here.

## [0.1.0] - 2026-05-06

### Added

- Initial release of the Elixir Aztec Code encoder (ISO/IEC 24778:2008).

- **GF(16) arithmetic** — discrete log/antilog tables for the primitive
  polynomial `x^4 + x + 1 = 0x13`. Used exclusively for mode message
  Reed-Solomon encoding. Implemented inline (no external package) because
  GF(16) is a small 15-element multiplicative group.

- **GF(256)/0x12D arithmetic** — exp/log tables for the primitive polynomial
  `x^8 + x^5 + x^4 + x^2 + x + 1 = 0x12D`. This is the same polynomial as
  Data Matrix ECC200, and is DIFFERENT from QR Code's `0x11D`. Used for 8-bit
  data codeword Reed-Solomon (b=1 root convention, same as MA02).

- **Reed-Solomon generator polynomial** construction for both GF(16) (mode
  message) and GF(256)/0x12D (data codewords). Uses the b=1 convention:
  roots are `α^1, α^2, ..., α^n`.

- **Binary-Shift from Upper mode encoding** (v0.1.0 byte-mode path). All
  input is wrapped in a single Binary-Shift block: a 5-bit escape codeword
  (31), a length prefix (5 bits if ≤ 31 bytes, else 5+11 bits), and raw
  bytes MSB first.

- **Symbol size selection** — automatically picks the smallest compact
  (1–4 layers) or full (1–32 layers) symbol that fits the data at a
  configurable ECC percentage (default 23%). Uses a 20% conservative
  overhead multiplier for bit-stuffing estimation.

- **Capacity tables** — pre-computed `{total_bits, max_bytes8}` for all
  compact and full layer counts, matching ISO/IEC 24778:2008 Table 1.

- **Padding** — zero-fills partial bytes and extends to the target codeword
  count. Applies the "all-zero codeword avoidance" rule (last byte 0x00
  replaced with 0xFF).

- **Bit stuffing** — inserts a complement bit after every run of 4 identical
  bits, preventing long monotone runs that could interfere with the scanner's
  reference grid. Applied to the combined data+ECC bit stream before grid
  placement.

- **Mode message encoding** — GF(16) RS-protected format information:
  - Compact (28 bits = 7 nibbles): 2 data nibbles + 5 ECC nibbles
  - Full (40 bits = 10 nibbles): 4 data nibbles + 6 ECC nibbles

- **Bullseye finder pattern** — concentric rings centered at the symbol
  center, with Chebyshev distance determining dark/light:
  `d ≤ 1` → DARK, `d=2` → LIGHT, `d=3` → DARK, `d=4` → LIGHT,
  `d=5` → DARK (compact), `d=6` → LIGHT (full), `d=7` → DARK (full).

- **Orientation marks** — four always-dark corner modules of the mode
  message ring (Chebyshev radius `bullseye_radius + 1` from center).

- **Reference grid** (full symbols only) — horizontal and vertical lines
  at multiples of 16 modules from center. Alternating dark/light pattern
  along each line. Placed before the bullseye so the bullseye overwrites
  any overlapping modules.

- **Clockwise data spiral placement** — fills the mode message ring
  remainder first, then spirals outward through data layers one at a time.
  Within each layer, pairs of bits are placed outer-then-inner at each
  position. Reserved modules (bullseye, orientation marks, mode message,
  reference grid) are skipped.

- **`encode/2`** — main public API. Returns
  `{:ok, %{rows: r, cols: c, modules: [[boolean()]]}}` or
  `{:error, :input_too_long}`.

- **`encode!/2`** — raises `ArgumentError` on error.

- **`render_ascii/2`** — debug ASCII art renderer (`█` = dark, ` ` = light).

- **112 tests** with 96.67% line coverage, organised across 16 test groups:
  GF(16) arithmetic, GF(16) RS, GF(256) arithmetic, GF(256) RS, bit
  encoding, symbol selection, padding, bit stuffing, mode message, bullseye,
  reference grid, data placement, integration, and edge cases.
