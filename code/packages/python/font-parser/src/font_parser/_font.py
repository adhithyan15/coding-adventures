"""
Internal font parsing implementation.

This module translates the OpenType/TrueType binary format into Python
dataclasses. Everything is big-endian byte arithmetic — no external
dependencies, no OS calls, no ctypes.

Design note: Python's ``struct.unpack_from`` handles big-endian decoding
cleanly (format character '>' = network / big-endian). We use it throughout
instead of manual bit shifts.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from typing import Optional


# ─────────────────────────────────────────────────────────────────────────────
# Error class
# ─────────────────────────────────────────────────────────────────────────────


class FontError(Exception):
    """Raised when font bytes cannot be parsed.

    The ``kind`` attribute is a short string discriminant for programmatic
    handling without string-matching on the message:

    - ``"InvalidMagic"`` — sfntVersion not recognised
    - ``"InvalidHeadMagic"`` — head.magicNumber != 0x5F0F3CF5
    - ``"TableNotFound"`` — required table missing from directory
    - ``"BufferTooShort"`` — byte read went past end of buffer
    - ``"UnsupportedCmapFormat"`` — no Format 4 cmap for platform 3 / enc 1
    """

    def __init__(self, kind: str, message: str) -> None:
        super().__init__(message)
        self.kind = kind


# ─────────────────────────────────────────────────────────────────────────────
# Public metric types
# ─────────────────────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class FontMetrics:
    """Global typographic metrics for a font.

    All integer fields are in **design units**. Convert to pixels::

        pixels = design_units * font_size_px / units_per_em

    For Inter Regular at 16 px: 16 / 2048 = 0.0078125 px per design unit.

    Attributes
    ----------
    units_per_em:
        Design units per em square. Inter = 2048; older fonts often 1000.
    ascender:
        Baseline-to-top distance (positive).
    descender:
        Baseline-to-bottom distance (negative, e.g. -512 for Inter).
    line_gap:
        Extra inter-line spacing (often 0). Natural line height =
        ``ascender - descender + line_gap``.
    x_height:
        Height of lowercase 'x' in design units.
        ``None`` if OS/2 version < 2 or OS/2 table absent.
    cap_height:
        Height of uppercase 'H' in design units.
        ``None`` if OS/2 version < 2 or OS/2 table absent.
    num_glyphs:
        Total glyph count.
    family_name:
        Font family name, e.g. ``"Inter"``.
    subfamily_name:
        Style name, e.g. ``"Regular"`` or ``"Bold Italic"``.
    """

    units_per_em: int
    ascender: int
    descender: int
    line_gap: int
    x_height: Optional[int]
    cap_height: Optional[int]
    num_glyphs: int
    family_name: str
    subfamily_name: str


@dataclass(frozen=True)
class GlyphMetrics:
    """Horizontal metrics for a single glyph, in design units.

    Attributes
    ----------
    advance_width:
        Distance to advance the pen after rendering this glyph.
    left_side_bearing:
        Space between the pen and the left ink edge. Usually positive;
        negative for glyphs that protrude to the left (rare).
    """

    advance_width: int
    left_side_bearing: int


# ─────────────────────────────────────────────────────────────────────────────
# FontFile
# ─────────────────────────────────────────────────────────────────────────────


@dataclass
class _Tables:
    """Pre-parsed table offsets.  ``None`` = table absent in font."""

    head: int
    hhea: int
    maxp: int
    cmap: int
    hmtx: int
    kern: Optional[int]
    name: Optional[int]
    os2: Optional[int]


class FontFile:
    """Opaque handle to a parsed font file.

    Created by :func:`load`. Pass to the metric functions.
    Stores a ``bytes`` copy of the font data and the pre-parsed table
    directory so that individual queries are pure byte reads.

    Attributes are prefixed with ``_`` to signal they are internal.
    """

    def __init__(self, data: bytes, tables: _Tables) -> None:
        self._data: bytes = data
        self._tables: _Tables = tables

    def __repr__(self) -> str:
        return f"FontFile(<{len(self._data)} bytes>)"


# ─────────────────────────────────────────────────────────────────────────────
# Big-endian reading helpers
# ─────────────────────────────────────────────────────────────────────────────


def _u8(buf: bytes, off: int) -> int:
    """Read one unsigned byte."""
    if off >= len(buf):
        raise FontError("BufferTooShort", f"read u8 at offset {off} out of bounds")
    return buf[off]


def _u16(buf: bytes, off: int) -> int:
    """Read a 16-bit big-endian unsigned integer."""
    try:
        (v,) = struct.unpack_from(">H", buf, off)
        return v
    except struct.error:
        raise FontError("BufferTooShort", f"read u16 at offset {off} out of bounds") from None


def _i16(buf: bytes, off: int) -> int:
    """Read a 16-bit big-endian signed integer."""
    try:
        (v,) = struct.unpack_from(">h", buf, off)
        return v
    except struct.error:
        raise FontError("BufferTooShort", f"read i16 at offset {off} out of bounds") from None


def _u32(buf: bytes, off: int) -> int:
    """Read a 32-bit big-endian unsigned integer."""
    try:
        (v,) = struct.unpack_from(">I", buf, off)
        return v
    except struct.error:
        raise FontError("BufferTooShort", f"read u32 at offset {off} out of bounds") from None


# ─────────────────────────────────────────────────────────────────────────────
# Table directory
# ─────────────────────────────────────────────────────────────────────────────


def _find_table(buf: bytes, num_tables: int, tag: bytes) -> Optional[int]:
    """Return the absolute byte offset of a named table, or ``None``."""
    # Table records start at offset 12, 16 bytes each.
    # tag(4) + checksum(4) + offset(4) + length(4)
    for i in range(num_tables):
        rec = 12 + i * 16
        if buf[rec : rec + 4] == tag:
            return _u32(buf, rec + 8)
    return None


def _require_table(buf: bytes, num_tables: int, tag: bytes, name: str) -> int:
    """Find a required table or raise :class:`FontError`."""
    off = _find_table(buf, num_tables, tag)
    if off is None:
        raise FontError("TableNotFound", f"required table '{name}' not found in font")
    return off


# ─────────────────────────────────────────────────────────────────────────────
# load
# ─────────────────────────────────────────────────────────────────────────────


def load(data: bytes | bytearray | memoryview) -> FontFile:
    """Parse raw font bytes and return a :class:`FontFile` handle.

    Parameters
    ----------
    data:
        Raw bytes from a ``.ttf`` or ``.otf`` file.

    Raises
    ------
    FontError
        If the bytes are not a valid OpenType/TrueType font, if a required
        table is missing, or if any byte read goes out of bounds.

    Examples
    --------
    >>> data = open("Inter-Regular.ttf", "rb").read()
    >>> font = load(data)
    >>> font_metrics(font).units_per_em
    2048
    """
    # Normalise to bytes for consistent slicing.
    buf: bytes = bytes(data)

    if len(buf) < 12:
        raise FontError("BufferTooShort", "buffer is too small to be a valid font")

    # sfntVersion: 0x00010000 (TrueType) or 0x4F54544F ("OTTO", CFF).
    sfnt = _u32(buf, 0)
    if sfnt not in (0x00010000, 0x4F54544F):
        raise FontError(
            "InvalidMagic",
            f"invalid sfntVersion 0x{sfnt:08X}; expected 0x00010000 or 0x4F54544F",
        )

    num_tables = _u16(buf, 4)

    tables = _Tables(
        head=_require_table(buf, num_tables, b"head", "head"),
        hhea=_require_table(buf, num_tables, b"hhea", "hhea"),
        maxp=_require_table(buf, num_tables, b"maxp", "maxp"),
        cmap=_require_table(buf, num_tables, b"cmap", "cmap"),
        hmtx=_require_table(buf, num_tables, b"hmtx", "hmtx"),
        kern=_find_table(buf, num_tables, b"kern"),
        name=_find_table(buf, num_tables, b"name"),
        os2=_find_table(buf, num_tables, b"OS/2"),
    )

    # Validate head.magicNumber sentinel (at offset 12 within head table).
    magic = _u32(buf, tables.head + 12)
    if magic != 0x5F0F3CF5:
        raise FontError(
            "InvalidHeadMagic",
            f"invalid head.magicNumber 0x{magic:08X}; expected 0x5F0F3CF5",
        )

    return FontFile(buf, tables)


# ─────────────────────────────────────────────────────────────────────────────
# font_metrics
# ─────────────────────────────────────────────────────────────────────────────


def font_metrics(font: FontFile) -> FontMetrics:
    """Return global typographic metrics for the font.

    Prefers ``OS/2`` typographic values over ``hhea`` values when the OS/2
    table is present — consistent with CSS, Core Text, and DirectWrite.

    Parameters
    ----------
    font:
        A :class:`FontFile` returned by :func:`load`.
    """
    buf = font._data
    t = font._tables

    # head: unitsPerEm at offset 18.
    units_per_em: int = _u16(buf, t.head + 18)

    # hhea: fallback values.
    hhea_ascender: int  = _i16(buf, t.hhea + 4)
    hhea_descender: int = _i16(buf, t.hhea + 6)
    hhea_line_gap: int  = _i16(buf, t.hhea + 8)

    # maxp: numGlyphs at offset 4.
    num_glyphs: int = _u16(buf, t.maxp + 4)

    # OS/2: prefer typo metrics; fall back to hhea.
    ascender  = hhea_ascender
    descender = hhea_descender
    line_gap  = hhea_line_gap
    x_height: Optional[int]   = None
    cap_height: Optional[int] = None

    if t.os2 is not None:
        base = t.os2
        version   = _u16(buf, base)
        ascender  = _i16(buf, base + 68)
        descender = _i16(buf, base + 70)
        line_gap  = _i16(buf, base + 72)
        if version >= 2:
            x_height   = _i16(buf, base + 86)
            cap_height = _i16(buf, base + 88)

    family_name    = _read_name(buf, t.name, 1) or "(unknown)"
    subfamily_name = _read_name(buf, t.name, 2) or "(unknown)"

    return FontMetrics(
        units_per_em=units_per_em,
        ascender=ascender,
        descender=descender,
        line_gap=line_gap,
        x_height=x_height,
        cap_height=cap_height,
        num_glyphs=num_glyphs,
        family_name=family_name,
        subfamily_name=subfamily_name,
    )


# ─────────────────────────────────────────────────────────────────────────────
# glyph_id — cmap Format 4 lookup
# ─────────────────────────────────────────────────────────────────────────────


def glyph_id(font: FontFile, codepoint: int) -> Optional[int]:
    """Map a Unicode codepoint to a glyph ID.

    Only covers the Basic Multilingual Plane (0x0000–0xFFFF).
    Returns ``None`` for codepoints above 0xFFFF or not in the font.

    Parameters
    ----------
    font:
        A :class:`FontFile` returned by :func:`load`.
    codepoint:
        Unicode codepoint (integer).

    Algorithm
    ---------
    Format 4 encodes BMP codepoints as sorted segments.  Binary-search
    ``endCode[]``, verify ``startCode``, then resolve via direct delta or
    the idRangeOffset self-relative pointer trick.
    """
    if not (0 <= codepoint <= 0xFFFF):
        return None

    cp = codepoint
    buf = font._data
    cmap_off = font._tables.cmap

    # ── Find the Format 4 subtable ──────────────────────────────────────────
    num_subtables = _u16(buf, cmap_off + 2)
    subtable_abs: Optional[int] = None

    for i in range(num_subtables):
        rec = cmap_off + 4 + i * 8
        platform_id = _u16(buf, rec)
        encoding_id = _u16(buf, rec + 2)
        sub_off     = _u32(buf, rec + 4)

        if platform_id == 3 and encoding_id == 1:
            subtable_abs = cmap_off + sub_off
            break  # best possible
        if platform_id == 0 and subtable_abs is None:
            subtable_abs = cmap_off + sub_off

    if subtable_abs is None:
        return None

    if _u16(buf, subtable_abs) != 4:
        return None  # not Format 4

    # ── Format 4 header ─────────────────────────────────────────────────────
    seg_count_x2 = _u16(buf, subtable_abs + 6)
    seg_count     = seg_count_x2 // 2

    end_codes_base       = subtable_abs + 14
    start_codes_base     = subtable_abs + 16 + seg_count * 2
    id_delta_base        = subtable_abs + 16 + seg_count * 4
    id_range_offset_base = subtable_abs + 16 + seg_count * 6

    # ── Binary search on endCode[] ──────────────────────────────────────────
    lo, hi = 0, seg_count
    while lo < hi:
        mid = (lo + hi) // 2
        if _u16(buf, end_codes_base + mid * 2) < cp:
            lo = mid + 1
        else:
            hi = mid

    if lo >= seg_count:
        return None

    end_code   = _u16(buf, end_codes_base   + lo * 2)
    start_code = _u16(buf, start_codes_base + lo * 2)

    if not (start_code <= cp <= end_code):
        return None

    id_delta       = _i16(buf, id_delta_base        + lo * 2)
    id_range_offset = _u16(buf, id_range_offset_base + lo * 2)

    if id_range_offset == 0:
        # Direct delta: (cp + idDelta) mod 65536.
        glyph = (cp + id_delta) & 0xFFFF
    else:
        # Indirect — idRangeOffset is a self-relative byte offset.
        # Absolute byte offset of glyphIdArray[(cp - startCode)]:
        #   (id_range_offset_base + lo*2) + id_range_offset + (cp - startCode)*2
        abs_off = (
            (id_range_offset_base + lo * 2)
            + id_range_offset
            + (cp - start_code) * 2
        )
        glyph = _u16(buf, abs_off)

    return None if glyph == 0 else glyph


# ─────────────────────────────────────────────────────────────────────────────
# glyph_metrics — hmtx lookup
# ─────────────────────────────────────────────────────────────────────────────


def glyph_metrics(font: FontFile, gid: int) -> Optional[GlyphMetrics]:
    """Return horizontal metrics for a glyph ID.

    Returns ``None`` if ``gid`` is out of range.

    hmtx layout::

        hMetrics[0 .. numberOfHMetrics]   — (advanceWidth u16, lsb i16) × N
        leftSideBearings[0 ..]            — lsb i16 only (shared advance)
    """
    buf = font._data
    t   = font._tables

    num_glyphs   = _u16(buf, t.maxp + 4)
    num_h_metrics = _u16(buf, t.hhea + 34)
    hmtx_off     = t.hmtx

    if not (0 <= gid < num_glyphs):
        return None

    if gid < num_h_metrics:
        base = hmtx_off + gid * 4
        return GlyphMetrics(
            advance_width=_u16(buf, base),
            left_side_bearing=_i16(buf, base + 2),
        )
    else:
        last_advance = _u16(buf, hmtx_off + (num_h_metrics - 1) * 4)
        lsb_off = hmtx_off + num_h_metrics * 4 + (gid - num_h_metrics) * 2
        return GlyphMetrics(
            advance_width=last_advance,
            left_side_bearing=_i16(buf, lsb_off),
        )


# ─────────────────────────────────────────────────────────────────────────────
# kerning — kern Format 0 lookup
# ─────────────────────────────────────────────────────────────────────────────


def kerning(font: FontFile, left: int, right: int) -> int:
    """Return the kerning adjustment for a glyph pair (design units).

    Returns ``0`` if the font has no ``kern`` table or the pair is absent.
    Negative = tighter spacing; positive = wider.

    Uses binary search on Format 0 sorted pairs.
    Composite key: ``(left << 16) | right``.
    """
    buf = font._data

    if font._tables.kern is None:
        return 0

    kern_off = font._tables.kern
    n_tables = _u16(buf, kern_off + 2)

    pos = kern_off + 4
    for _ in range(n_tables):
        if pos + 6 > len(buf):
            break
        length   = _u16(buf, pos + 2)
        coverage = _u16(buf, pos + 4)
        sub_format = coverage >> 8

        if sub_format == 0:
            n_pairs    = _u16(buf, pos + 6)
            pairs_base = pos + 14  # 6 (subtable hdr) + 8 (format0 hdr)
            target = (left << 16) | right

            lo, hi = 0, n_pairs
            while lo < hi:
                mid = (lo + hi) // 2
                pair_off  = pairs_base + mid * 6
                pair_left  = _u16(buf, pair_off)
                pair_right = _u16(buf, pair_off + 2)
                key = (pair_left << 16) | pair_right

                if key == target:
                    return _i16(buf, pair_off + 4)
                elif key < target:
                    lo = mid + 1
                else:
                    hi = mid

        pos += length

    return 0


# ─────────────────────────────────────────────────────────────────────────────
# name table reading
# ─────────────────────────────────────────────────────────────────────────────


def _read_name(buf: bytes, name_off: Optional[int], name_id: int) -> Optional[str]:
    """Read a string from the ``name`` table by nameID.

    Prefers platform 3 / encoding 1 (Windows Unicode BMP, UTF-16 BE).
    Falls back to platform 0 (Unicode) if the Windows record is absent.
    """
    if name_off is None:
        return None

    base = name_off
    count         = _u16(buf, base + 2)
    string_offset = _u16(buf, base + 4)

    best: Optional[tuple[int, int, int]] = None  # (platformId, abs_start, length)

    for i in range(count):
        rec         = base + 6 + i * 12
        platform_id = _u16(buf, rec)
        encoding_id = _u16(buf, rec + 2)
        nid         = _u16(buf, rec + 6)
        length      = _u16(buf, rec + 8)
        str_off     = _u16(buf, rec + 10)

        if nid != name_id:
            continue

        abs_start = base + string_offset + str_off

        if platform_id == 3 and encoding_id == 1:
            best = (3, abs_start, length)
            break  # best possible
        if platform_id == 0 and best is None:
            best = (0, abs_start, length)

    if best is None:
        return None

    _platform, start, length = best
    raw = buf[start : start + length]

    # Decode UTF-16 BE. Python's codecs handle this natively.
    # Unpaired surrogates are replaced with U+FFFD via 'replace' error mode.
    return raw.decode("utf-16-be", errors="replace")
