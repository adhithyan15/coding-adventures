# Changelog — coding-adventures-data-matrix (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-26

### Added

- Initial release of the Lua Data Matrix ECC200 encoder (ISO/IEC 16022:2006).
- `encode(data, opts)` — encode any string to a `ModuleGrid`.
  - Supports all 30 square symbol sizes (10×10 through 144×144).
  - Supports all 6 rectangular sizes (8×18 through 16×48).
  - Automatic symbol selection: smallest fit by data capacity.
  - `opts.shape` selects between `"square"` (default), `"rectangular"`, or `"any"`.
- ASCII data encoding with two-digit pair packing (130 + d1×10 + d2),
  single-byte literal (value + 1), and UPPER_SHIFT (235) for bytes 128–255.
- ISO §5.2.3 pad codeword scrambling: first pad literal 129, subsequent pads
  use `129 + (149 × k mod 253) + 1` with wrap at 254.
- Reed-Solomon ECC over GF(256) with primitive polynomial `0x12D`,
  using the `b=1` convention (roots α¹…αⁿ) — distinct from QR Code's
  `0x11D` / `b=0` field.
- LFSR-based polynomial division for systematic RS encoding.
- Block splitting + round-robin data + ECC interleaving for multi-block
  symbols (e.g. 32×32 uses 2 blocks; 144×144 uses 10 blocks).
- Grid initialisation with the L-shaped finder (solid left + bottom),
  alternating timing borders on the top + right edges, and 2-module-wide
  alignment borders between data regions in multi-region symbols.
- Utah diagonal placement algorithm including all four corner patterns
  and the four ISO Annex F boundary-wrap rules.
- Logical → physical coordinate mapping that accounts for the outer
  border and inter-region alignment borders.
- ISO right-and-bottom fill rule for any modules unvisited by the
  diagonal walk (`(r + c) mod 2 == 1` becomes dark).
- Comprehensive `spec/data_matrix_spec.lua` covering:
  - Module exports + version
  - GF(256)/0x12D field axioms (zero, identity, commutativity, distributive
    law, generator order = 255, log/exp inverse property, known products)
  - ASCII encoding (single chars, digit pairs, extended bytes, edge cases)
  - Pad codewords (ISO worked example, length, scrambled value range)
  - Symbol selection (smallest fit, shape filtering, capacity boundary,
    InputTooLongError, unknown shape error)
  - Generator polynomials (degree, monic, root verification at α¹…αⁿ)
  - RS block encoding (length, zero-data passthrough, syndrome=0 invariant)
  - Block interleaving (single-block passthrough, two-block 32×32 layout)
  - Grid initialisation (L-finder invariants, timing alternation, alignment
    border placement)
  - Utah placement (correct dimensions, all cells boolean)
  - Logical → physical mapping (single + multi region offsets)
  - Full encode pipeline (size selection, structural invariants, determinism,
    rectangular shape, empty input, dark-module sanity)
  - Error handling (InputTooLong, unknown shape, non-string input)

### Implementation notes

- Lua 5.4+ bitwise operators used throughout (`~` for XOR, `<<` / `>>` shifts,
  `&` for AND).  All grid coordinates are 1-indexed in the public ModuleGrid.
- The Utah algorithm internally uses 0-indexed coordinates to mirror the Go
  reference implementation; only the boundary translation
  (`logical_to_physical`) and the final write into the grid table convert
  to Lua's 1-indexed convention.
- GF(256)/0x12D log/antilog tables are built locally rather than relying on
  `coding-adventures-gf256`.  The shared module is hard-coded to QR Code's
  `0x11D` polynomial for its module-level tables; using its `new_field` API
  with `0x12D` would work but rejects the log/antilog optimisation since
  `new_field` always uses Russian-peasant multiplication.  The local 0x12D
  tables also keep this module dependency-free for the encoding path.
- Per-RS-block ECC count is the same for every block in a given symbol size,
  so we cache one generator polynomial per `ecc_per_block` value at module
  load and re-use across encodes.
