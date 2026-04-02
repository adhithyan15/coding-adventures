"""
Tests for the font_parser package.

Tests split into two categories:
1. Integration tests against Inter Regular v4.0 (real font, known values)
2. Unit tests against synthetic font bytes (test kern/cmap logic without
   depending on a specific font's optional tables)
"""

from __future__ import annotations

import struct
from pathlib import Path

import pytest

from font_parser import (
    FontError,
    FontMetrics,
    GlyphMetrics,
    font_metrics,
    glyph_id,
    glyph_metrics,
    kerning,
    load,
)


# ─────────────────────────────────────────────────────────────────────────────
# Helpers
# ─────────────────────────────────────────────────────────────────────────────

FONT_FIXTURE = (
    Path(__file__).parent.parent.parent.parent.parent  # code/
    / "fixtures"
    / "fonts"
    / "Inter-Regular.ttf"
)


def inter_bytes() -> bytes:
    """Load Inter Regular from the shared fixtures directory."""
    return FONT_FIXTURE.read_bytes()


def build_synthetic_font(
    pairs: list[tuple[int, int, int]],
) -> bytes:
    """Build a minimal valid synthetic OpenType font with a kern table.

    Tables included: head, hhea, maxp, cmap (Format 4, sentinel), hmtx, kern.

    Parameters
    ----------
    pairs:
        List of ``(left_glyph, right_glyph, kern_value)`` tuples.
    """

    def w16(v: int) -> bytes:
        return struct.pack(">H", v & 0xFFFF)

    def wi16(v: int) -> bytes:
        return struct.pack(">h", v)

    def w32(v: int) -> bytes:
        return struct.pack(">I", v & 0xFFFFFFFF)

    def tag(s: str) -> bytes:
        return s.encode("ascii")

    # Layout sizes (bytes)
    NUM_TABLES   = 6
    DIR_SIZE     = 12 + NUM_TABLES * 16
    HEAD_LEN     = 54
    HHEA_LEN     = 36
    MAXP_LEN     = 6
    CMAP_LEN     = 36   # 4 + 8 + 24
    HMTX_LEN     = 5 * 4
    N_PAIRS      = len(pairs)
    KERN_LEN     = 4 + 6 + 8 + N_PAIRS * 6

    HEAD_OFF = DIR_SIZE
    HHEA_OFF = HEAD_OFF + HEAD_LEN
    MAXP_OFF = HHEA_OFF + HHEA_LEN
    CMAP_OFF = MAXP_OFF + MAXP_LEN
    HMTX_OFF = CMAP_OFF + CMAP_LEN
    KERN_OFF = HMTX_OFF + HMTX_LEN

    buf = bytearray()

    # ── Offset Table ─────────────────────────────────────────────────────────
    buf += w32(0x00010000)      # sfntVersion
    buf += w16(NUM_TABLES)
    buf += w16(64) + w16(2) + w16(32)   # searchRange, entrySelector, rangeShift

    # ── Table Records (sorted by tag: cmap < head < hhea < hmtx < kern < maxp)
    def rec(t_tag: str, off: int, length: int) -> bytes:
        return tag(t_tag) + w32(0) + w32(off) + w32(length)

    buf += rec("cmap", CMAP_OFF, CMAP_LEN)
    buf += rec("head", HEAD_OFF, HEAD_LEN)
    buf += rec("hhea", HHEA_OFF, HHEA_LEN)
    buf += rec("hmtx", HMTX_OFF, HMTX_LEN)
    buf += rec("kern", KERN_OFF, KERN_LEN)
    buf += rec("maxp", MAXP_OFF, MAXP_LEN)

    assert len(buf) == DIR_SIZE

    # ── head table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    buf += w32(0x00010000)      # version
    buf += w32(0x00010000)      # fontRevision
    buf += w32(0)               # checksumAdjustment
    buf += w32(0x5F0F3CF5)      # magicNumber ← sentinel
    buf += w16(0)               # flags
    buf += w16(1000)            # unitsPerEm
    buf += bytes(16)            # created + modified (8+8)
    buf += wi16(0) * 4          # xMin, yMin, xMax, yMax
    buf += w16(0) + w16(8)      # macStyle, lowestRecPPEM
    buf += wi16(2)              # fontDirectionHint
    buf += wi16(0)              # indexToLocFormat
    buf += wi16(0)              # glyphDataFormat
    assert len(buf) - p_start == HEAD_LEN

    # ── hhea table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    buf += w32(0x00010000)      # version
    buf += wi16(800)            # ascender
    buf += wi16(-200)           # descender
    buf += wi16(0)              # lineGap
    buf += w16(1000)            # advanceWidthMax
    buf += wi16(0) * 3          # minLSB, minRSB, xMaxExtent
    buf += wi16(1) + wi16(0) + wi16(0)   # caretSlopeRise, Run, Offset
    buf += wi16(0) * 4          # reserved
    buf += wi16(0)              # metricDataFormat
    buf += w16(5)               # numberOfHMetrics
    assert len(buf) - p_start == HHEA_LEN

    # ── maxp table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    buf += w32(0x00005000)      # version 0.5
    buf += w16(5)               # numGlyphs
    assert len(buf) - p_start == MAXP_LEN

    # ── cmap table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    buf += w16(0) + w16(1)      # version=0, numSubtables=1
    # Encoding record: platform 3, encoding 1, subtable at offset 12
    buf += w16(3) + w16(1) + w32(12)
    # Format 4, segCount=1 (sentinel only)
    sub_len = 24
    buf += w16(4) + w16(sub_len) + w16(0)   # format, length, language
    buf += w16(2) + w16(2) + w16(0) + w16(0)  # segCountX2, searchRange, entrySelector, rangeShift
    buf += w16(0xFFFF)          # endCode[0] sentinel
    buf += w16(0)               # reservedPad
    buf += w16(0xFFFF)          # startCode[0] sentinel
    buf += wi16(1)              # idDelta[0]
    buf += w16(0)               # idRangeOffset[0]
    assert len(buf) - p_start == CMAP_LEN

    # ── hmtx table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    for _ in range(5):
        buf += w16(600) + wi16(50)
    assert len(buf) - p_start == HMTX_LEN

    # ── kern table ───────────────────────────────────────────────────────────
    p_start = len(buf)
    buf += w16(0) + w16(1)          # version, nTables
    sub_len2 = 6 + 8 + N_PAIRS * 6
    buf += w16(0) + w16(sub_len2) + w16(0x0001)  # subtable: version, length, coverage
    buf += w16(N_PAIRS) + w16(0) + w16(0) + w16(0)  # nPairs, searchRange, entrySelector, rangeShift
    sorted_pairs = sorted(pairs, key=lambda p: (p[0] << 16) | p[1])
    for left_g, right_g, val in sorted_pairs:
        buf += w16(left_g) + w16(right_g) + wi16(val)
    assert len(buf) - p_start == KERN_LEN

    return bytes(buf)


# ─────────────────────────────────────────────────────────────────────────────
# Tests: load()
# ─────────────────────────────────────────────────────────────────────────────


class TestLoad:
    def test_empty_buffer_raises_buffer_too_short(self) -> None:
        with pytest.raises(FontError) as exc_info:
            load(b"")
        assert exc_info.value.kind == "BufferTooShort"

    def test_wrong_magic_raises_invalid_magic(self) -> None:
        buf = bytearray(256)
        struct.pack_into(">I", buf, 0, 0xDEADBEEF)
        with pytest.raises(FontError) as exc_info:
            load(bytes(buf))
        assert exc_info.value.kind == "InvalidMagic"

    def test_load_inter_regular_succeeds(self) -> None:
        font = load(inter_bytes())
        assert font is not None

    def test_load_synthetic_font_succeeds(self) -> None:
        font = load(build_synthetic_font([(1, 2, -140)]))
        assert font is not None

    def test_load_accepts_bytearray(self) -> None:
        font = load(bytearray(inter_bytes()))
        assert font is not None

    def test_load_accepts_memoryview(self) -> None:
        font = load(memoryview(inter_bytes()))
        assert font is not None


# ─────────────────────────────────────────────────────────────────────────────
# Tests: font_metrics()
# ─────────────────────────────────────────────────────────────────────────────


class TestFontMetrics:
    def setup_method(self) -> None:
        self.font = load(inter_bytes())

    def test_units_per_em_is_2048(self) -> None:
        assert font_metrics(self.font).units_per_em == 2048

    def test_family_name_is_inter(self) -> None:
        assert font_metrics(self.font).family_name == "Inter"

    def test_subfamily_name_is_regular(self) -> None:
        assert font_metrics(self.font).subfamily_name == "Regular"

    def test_ascender_is_positive(self) -> None:
        assert font_metrics(self.font).ascender > 0

    def test_descender_is_non_positive(self) -> None:
        assert font_metrics(self.font).descender <= 0

    def test_num_glyphs_is_large(self) -> None:
        assert font_metrics(self.font).num_glyphs > 100

    def test_x_height_is_positive(self) -> None:
        m = font_metrics(self.font)
        assert m.x_height is not None
        assert m.x_height > 0

    def test_cap_height_is_positive(self) -> None:
        m = font_metrics(self.font)
        assert m.cap_height is not None
        assert m.cap_height > 0

    def test_synthetic_font_units_per_em(self) -> None:
        font = load(build_synthetic_font([]))
        assert font_metrics(font).units_per_em == 1000

    def test_synthetic_font_unknown_family_name(self) -> None:
        font = load(build_synthetic_font([]))
        assert font_metrics(font).family_name == "(unknown)"

    def test_return_type_is_font_metrics(self) -> None:
        m = font_metrics(self.font)
        assert isinstance(m, FontMetrics)


# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_id()
# ─────────────────────────────────────────────────────────────────────────────


class TestGlyphId:
    def setup_method(self) -> None:
        self.font = load(inter_bytes())

    def test_glyph_id_for_a_is_not_none(self) -> None:
        assert glyph_id(self.font, 0x0041) is not None

    def test_glyph_id_for_v_is_not_none(self) -> None:
        assert glyph_id(self.font, 0x0056) is not None

    def test_glyph_id_for_space_is_not_none(self) -> None:
        assert glyph_id(self.font, 0x0020) is not None

    def test_glyph_ids_for_a_and_v_differ(self) -> None:
        assert glyph_id(self.font, 0x0041) != glyph_id(self.font, 0x0056)

    def test_codepoint_above_ffff_returns_none(self) -> None:
        assert glyph_id(self.font, 0x10000) is None

    def test_negative_codepoint_returns_none(self) -> None:
        assert glyph_id(self.font, -1) is None

    def test_ffff_does_not_raise(self) -> None:
        # Sentinel region — should not raise regardless of return value.
        _ = glyph_id(self.font, 0xFFFF)


# ─────────────────────────────────────────────────────────────────────────────
# Tests: glyph_metrics()
# ─────────────────────────────────────────────────────────────────────────────


class TestGlyphMetrics:
    def setup_method(self) -> None:
        self.font = load(inter_bytes())

    def test_advance_width_for_a_is_positive(self) -> None:
        gid = glyph_id(self.font, 0x0041)
        assert gid is not None
        gm = glyph_metrics(self.font, gid)
        assert gm is not None
        assert gm.advance_width > 0

    def test_advance_width_in_reasonable_range(self) -> None:
        gid = glyph_id(self.font, 0x0041)
        assert gid is not None
        gm = glyph_metrics(self.font, gid)
        assert gm is not None
        assert 100 <= gm.advance_width <= 2400

    def test_out_of_range_glyph_returns_none(self) -> None:
        m = font_metrics(self.font)
        assert glyph_metrics(self.font, m.num_glyphs) is None

    def test_negative_glyph_id_returns_none(self) -> None:
        assert glyph_metrics(self.font, -1) is None

    def test_return_type_is_glyph_metrics(self) -> None:
        gid = glyph_id(self.font, 0x0041)
        assert gid is not None
        gm = glyph_metrics(self.font, gid)
        assert isinstance(gm, GlyphMetrics)


# ─────────────────────────────────────────────────────────────────────────────
# Tests: kerning()
# ─────────────────────────────────────────────────────────────────────────────


class TestKerning:
    def test_inter_no_kern_table_returns_zero(self) -> None:
        # Inter v4.0 uses GPOS, not the legacy kern table.
        font = load(inter_bytes())
        gid_a = glyph_id(font, 0x0041)
        gid_v = glyph_id(font, 0x0056)
        assert gid_a is not None and gid_v is not None
        assert kerning(font, gid_a, gid_v) == 0

    def test_synthetic_pair_1_2_negative(self) -> None:
        font = load(build_synthetic_font([(1, 2, -140), (3, 4, 80)]))
        assert kerning(font, 1, 2) == -140

    def test_synthetic_pair_3_4_positive(self) -> None:
        font = load(build_synthetic_font([(1, 2, -140), (3, 4, 80)]))
        assert kerning(font, 3, 4) == 80

    def test_absent_pair_returns_zero(self) -> None:
        font = load(build_synthetic_font([(1, 2, -140), (3, 4, 80)]))
        assert kerning(font, 1, 4) == 0

    def test_reversed_pair_returns_zero(self) -> None:
        font = load(build_synthetic_font([(1, 2, -140)]))
        assert kerning(font, 2, 1) == 0

    def test_no_kern_table_returns_zero(self) -> None:
        font = load(inter_bytes())
        assert kerning(font, 0, 0) == 0
