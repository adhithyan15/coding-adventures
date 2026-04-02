"""
font_parser — metrics-only OpenType/TrueType font parser.

Parses raw font bytes and exposes the numeric metrics needed to lay out
text without touching the OS font stack:

    from font_parser import load, font_metrics, glyph_id, glyph_metrics, kerning

    data = open("Inter-Regular.ttf", "rb").read()
    font = load(data)

    m = font_metrics(font)
    print(m.units_per_em)   # 2048 for Inter
    print(m.family_name)    # "Inter"

Public API
----------
- :func:`load` — parse bytes, validate, return :class:`FontFile`
- :func:`font_metrics` — global typographic metrics → :class:`FontMetrics`
- :func:`glyph_id` — Unicode codepoint → glyph ID (or ``None``)
- :func:`glyph_metrics` — per-glyph advance + lsb → :class:`GlyphMetrics`
- :func:`kerning` — kern pair value (design units, 0 if absent)
- :class:`FontError` — raised on parse failures
"""

from font_parser._font import (
    FontError,
    FontFile,
    FontMetrics,
    GlyphMetrics,
    font_metrics,
    glyph_id,
    glyph_metrics,
    kerning,
    load,
)

__all__ = [
    "FontError",
    "FontFile",
    "FontMetrics",
    "GlyphMetrics",
    "font_metrics",
    "glyph_id",
    "glyph_metrics",
    "kerning",
    "load",
]
