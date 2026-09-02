# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added — `MathVariants`: how a delimiter grows

`glyph_construction(font, glyph_id, axis)` and `min_connector_overlap(font)`.
This is what makes `\left(` wrap a tall fraction instead of staying the size of
an ordinary parenthesis, and TEX-3 cannot lay out a delimited group without it.

Two mechanisms, and a renderer needs both. **Variants** are complete larger
glyphs the designer drew; **assembly** is for when none is big enough — pieces
stacked with overlap, which is how a brace spanning half a page is built from a
top, a bottom, a middle cusp and a repeated extender.

Checked against fontTools reading STIX Two Math, like the constants: a
parenthesis's 13 variants and 3-part assembly, a brace's 5-part assembly with
two extenders, and that asking on the wrong axis returns nothing rather than an
unrelated construction.

The oracle earned it twice. The `MathVariants` offset is at byte 8 of the MATH
header, not 6 — 6 is `MathGlyphInfo`, and reading it there does not fail, it
returns plausible numbers from the wrong table (a minimum connector overlap of
8, which is itself an offset). And mutating the assembly's italics correction
to a bare `int16` — the `MathValue` stride trap this module already documents
for `MathConstants` — shifts every part record two bytes early and empties the
assembly.

### Added — raw table access for FNT02

`FontFile::data`, `FontFile::table`, `FontFile::index_to_loc_format` and
`FontFile::num_glyphs`. `glyph-parser` reads `glyf` and `loca`, which are
outline data and so deliberately outside this crate's metrics-only scope — but
it should not re-derive the table directory to find them. Keeping the directory
in exactly one crate is the point: a second implementation of "where does
`glyf` start" is how two readers come to disagree about the same font.

### Added — OpenType `MATH` table

`math_constants`, `italic_correction`, and `has_math_table`. This is the whole
remaining font dependency for maths typesetting: every Appendix G decision — how
far a fraction bar sits above the baseline, how thick it is, how much clearance
a radical needs — needs a number from this table, and a renderer that invents
them produces output that looks *almost* right.

Fourteen constants are exposed, chosen as the set fractions, radicals and
scripts consume, rather than transcribing all ~56 speculatively: an untested
constant is a liability, not a feature.

Verified against **fontTools** reading the same font, not against our own
understanding. `tests/fixtures/stix-two-math.MATH` is the real table extracted
from STIX Two Math (OFL 1.1, provenance recorded beside it), and every expected
value came from the independent implementation. A hand-built fixture and a
hand-written reader share their author's reading of the specification, so both
are wrong together and agree perfectly — the shape this repository has hit four
times.

That oracle earned its place immediately: several `MathValue` record indices
were wrong on first writing, recalled from memory rather than derived. The real
table exposed them at once.

The hazard worth naming, and pinned by its own test: almost every
`MathConstants` entry is a `MathValue` — an `int16` followed by an `Offset16`
device table. Reading them as bare `int16` *appears* to work, because the first
field really is the value, while halving the stride so every constant after the
first is wrong.

`math_constants` returns `Ok(None)` for a font with no `MATH` table, keeping
*absent* distinct from *corrupt*. Conflating them means silently typesetting
with default constants.

Rust only for now; the other ports of FNT00 do not carry this yet.


## [0.1.0] - 2026-04-01

### Added

- `load(bytes)` — parse raw font bytes, validate magic numbers, collect table offsets
- `font_metrics(font)` — extract global metrics from head/hhea/maxp/name/OS2 tables
- `glyph_id(font, codepoint)` — map Unicode BMP codepoints to glyph IDs via cmap Format 4
- `glyph_metrics(font, glyph_id)` — per-glyph advance width and left side bearing from hmtx
- `kerning(font, left, right)` — kern Format 0 binary search for glyph pair adjustments
- `FontMetrics` struct with units_per_em, ascender, descender, line_gap, x_height, cap_height, num_glyphs, family_name, subfamily_name
- `GlyphMetrics` struct with advance_width and left_side_bearing
- `FontError` enum: InvalidMagic, InvalidHeadMagic, TableNotFound, BufferTooShort, UnsupportedCmapFormat
- Zero dependencies — pure Rust, no `unsafe`, WASM-bake-able
- 26 unit tests covering all public functions and error paths
- Tested against Inter Regular v4.0 (SIL OFL): units_per_em=2048, A+V kern negative
