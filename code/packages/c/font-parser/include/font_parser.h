/*
 * font_parser.h — Metrics-only OpenType/TrueType font parser, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `font-parser` crate. An OpenType font file is a
 * binary table database: a directory of named tables and where to find each.
 * This parser reads the subset needed to *measure text* without touching the
 * OS font stack:
 *
 *   head — unitsPerEm                     hmtx — advance width + LSB per glyph
 *   hhea — ascender/descender/lineGap     kern — Format 0 kerning pairs
 *   maxp — numGlyphs                      name — family / subfamily (UTF-16 BE)
 *   cmap — Format 4 Unicode → glyph id    OS/2 — typo metrics, x/cap height
 *
 * It does NOT parse glyph outlines, shape text, or rasterize.
 *
 * `font_load` copies the font bytes into an owned `FontFile` and pre-parses the
 * table directory; every metric query is integer arithmetic over that buffer.
 * All multi-byte fields are big-endian; every read is bounds-checked (a short
 * or corrupt file yields an error or a `None`-style 0 return, never an
 * out-of-bounds access).
 *
 * Divergence from the Rust (documented): `FontError::TableNotFound` carries the
 * missing table's name in Rust; this port returns the plain
 * `FONT_ERR_TABLE_NOT_FOUND` code. Name strings are decoded UTF-16 BE → UTF-8
 * into fixed 128-byte buffers (truncated if longer).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef FONT_PARSER_H
#define FONT_PARSER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t, int16_t */

#ifdef __cplusplus
extern "C" {
#endif

#define FONT_NAME_CAP 128

/* Parsing errors. FONT_OK (0) means success. */
typedef enum {
    FONT_OK = 0,
    FONT_ERR_INVALID_MAGIC,
    FONT_ERR_INVALID_HEAD_MAGIC,
    FONT_ERR_TABLE_NOT_FOUND,
    FONT_ERR_BUFFER_TOO_SHORT,
    FONT_ERR_UNSUPPORTED_CMAP_FORMAT
} FontError;

/* Human-readable message for an error code. */
const char *font_error_str(FontError e);

/* Global typographic metrics (all integer fields in design units). */
typedef struct {
    uint16_t units_per_em;
    int16_t ascender;
    int16_t descender;
    int16_t line_gap;
    int has_x_height; /* 0 if OS/2 absent or version < 2 */
    int16_t x_height;
    int has_cap_height;
    int16_t cap_height;
    uint16_t num_glyphs;
    char family_name[FONT_NAME_CAP];    /* UTF-8; "(unknown)" if absent */
    char subfamily_name[FONT_NAME_CAP]; /* UTF-8; "(unknown)" if absent */
} FontMetrics;

/* Per-glyph horizontal metrics (design units). */
typedef struct {
    uint16_t advance_width;
    int16_t left_side_bearing;
} GlyphMetrics;

/* Opaque parsed-font handle (owns a copy of the font bytes). */
typedef struct FontFile FontFile;

/* Parse `bytes`. On success returns FONT_OK and sets *out to a new FontFile
 * (free with font_free); otherwise returns the error and leaves *out NULL. */
FontError font_load(const uint8_t *bytes, size_t len, FontFile **out);
void font_free(FontFile *f);

/* Fill *out with the font's global metrics. Never fails (missing optional
 * fields become has_* = 0, or fall back to hhea values). */
void font_metrics(const FontFile *f, FontMetrics *out);

/* Map a Unicode codepoint to a glyph id via the cmap Format 4 subtable.
 * Returns 1 and writes *out on success, 0 if unmapped / out of range. */
int font_glyph_id(const FontFile *f, uint32_t codepoint, uint16_t *out);

/* Horizontal metrics for a glyph id. Returns 1 and writes *out, or 0 if the id
 * is >= numGlyphs. */
int font_glyph_metrics(const FontFile *f, uint16_t glyph_id, GlyphMetrics *out);

/* Kerning adjustment (design units) for a glyph pair; 0 if there is no kern
 * table or the pair is absent. Negative = tighter spacing. */
int16_t font_kerning(const FontFile *f, uint16_t left, uint16_t right);

/* Big-endian read helpers (bounds-checked; overflow-safe). Return 1 on success
 * and write *out, else 0. */
int font_read_u16(const uint8_t *buf, size_t len, size_t offset, uint16_t *out);
int font_read_i16(const uint8_t *buf, size_t len, size_t offset, int16_t *out);
int font_read_u32(const uint8_t *buf, size_t len, size_t offset, uint32_t *out);

#ifdef __cplusplus
}
#endif

#endif /* FONT_PARSER_H */
