/*
 * font_parser.c — Metrics-only OpenType/TrueType font parser, pure ISO C17.
 * =====================================================================
 *
 * See font_parser.h. A faithful port of the Rust `font-parser` crate: parse the
 * table directory, then answer metric queries with bounds-checked big-endian
 * reads over the owned font buffer.
 */
#include "font_parser.h"

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy, strcpy, strlen */

struct FontFile {
    uint8_t *data;
    size_t len;
    /* Pre-parsed table offsets (absolute byte offsets into `data`). */
    uint32_t head, hhea, maxp, cmap, hmtx;
    int has_kern;
    uint32_t kern;
    int has_name;
    uint32_t name;
    int has_os2;
    uint32_t os2;
};

const char *font_error_str(FontError e) {
    switch (e) {
    case FONT_OK:
        return "ok";
    case FONT_ERR_INVALID_MAGIC:
        return "invalid sfntVersion magic";
    case FONT_ERR_INVALID_HEAD_MAGIC:
        return "invalid head.magicNumber";
    case FONT_ERR_TABLE_NOT_FOUND:
        return "required table not found";
    case FONT_ERR_BUFFER_TOO_SHORT:
        return "buffer too short";
    case FONT_ERR_UNSUPPORTED_CMAP_FORMAT:
        return "no Format 4 cmap subtable for platform 3 encoding 1";
    }
    return "unknown error";
}

/* ── Big-endian read helpers (overflow-safe bounds checks) ──────────────────*/
int font_read_u16(const uint8_t *buf, size_t len, size_t offset,
                  uint16_t *out) {
    if (offset > len || len - offset < 2) {
        return 0;
    }
    *out = (uint16_t)(((uint16_t)buf[offset] << 8) | buf[offset + 1]);
    return 1;
}
int font_read_i16(const uint8_t *buf, size_t len, size_t offset, int16_t *out) {
    uint16_t v;
    if (!font_read_u16(buf, len, offset, &v)) {
        return 0;
    }
    *out = (int16_t)v; /* reinterpret bits */
    return 1;
}
int font_read_u32(const uint8_t *buf, size_t len, size_t offset,
                  uint32_t *out) {
    if (offset > len || len - offset < 4) {
        return 0;
    }
    *out = ((uint32_t)buf[offset] << 24) | ((uint32_t)buf[offset + 1] << 16) |
           ((uint32_t)buf[offset + 2] << 8) | (uint32_t)buf[offset + 3];
    return 1;
}

/* ── Table directory ────────────────────────────────────────────────────────*/

/* Find a named table's offset. Returns 1 and writes *out if present; returns 0
 * if absent OR if a record runs off the end (mirrors the Rust `?` on a short
 * buffer, which yields "not found"). */
static int find_table(const uint8_t *buf, size_t len, uint16_t num_tables,
                      const char *tag, uint32_t *out) {
    uint16_t i;
    for (i = 0; i < num_tables; i++) {
        size_t rec = 12 + (size_t)i * 16;
        if (rec > len || len - rec < 4) {
            return 0; /* record tag out of bounds → not found */
        }
        if (memcmp(buf + rec, tag, 4) == 0) {
            uint32_t off;
            if (!font_read_u32(buf, len, rec + 8, &off)) {
                return 0;
            }
            *out = off;
            return 1;
        }
    }
    return 0;
}

static FontError parse_table_directory(const uint8_t *buf, size_t len,
                                       FontFile *f) {
    uint32_t sfnt_version;
    uint16_t num_tables;

    if (len < 12) {
        return FONT_ERR_BUFFER_TOO_SHORT;
    }
    if (!font_read_u32(buf, len, 0, &sfnt_version)) {
        return FONT_ERR_BUFFER_TOO_SHORT;
    }
    if (sfnt_version != 0x00010000u && sfnt_version != 0x4F54544Fu) {
        return FONT_ERR_INVALID_MAGIC;
    }
    if (!font_read_u16(buf, len, 4, &num_tables)) {
        return FONT_ERR_BUFFER_TOO_SHORT;
    }

    if (!find_table(buf, len, num_tables, "head", &f->head)) {
        return FONT_ERR_TABLE_NOT_FOUND;
    }
    if (!find_table(buf, len, num_tables, "hhea", &f->hhea)) {
        return FONT_ERR_TABLE_NOT_FOUND;
    }
    if (!find_table(buf, len, num_tables, "maxp", &f->maxp)) {
        return FONT_ERR_TABLE_NOT_FOUND;
    }
    if (!find_table(buf, len, num_tables, "cmap", &f->cmap)) {
        return FONT_ERR_TABLE_NOT_FOUND;
    }
    if (!find_table(buf, len, num_tables, "hmtx", &f->hmtx)) {
        return FONT_ERR_TABLE_NOT_FOUND;
    }
    f->has_kern = find_table(buf, len, num_tables, "kern", &f->kern);
    f->has_name = find_table(buf, len, num_tables, "name", &f->name);
    f->has_os2 = find_table(buf, len, num_tables, "OS/2", &f->os2);
    return FONT_OK;
}

FontError font_load(const uint8_t *bytes, size_t len, FontFile **out) {
    FontFile *f;
    FontError err;
    uint32_t magic;

    *out = NULL;
    f = (FontFile *)calloc(1, sizeof(FontFile));
    if (!f) {
        return FONT_ERR_BUFFER_TOO_SHORT; /* treat OOM as parse failure */
    }
    err = parse_table_directory(bytes, len, f);
    if (err != FONT_OK) {
        free(f);
        return err;
    }
    /* Validate the head.magicNumber sentinel (12 bytes into the head table). */
    if (!font_read_u32(bytes, len, (size_t)f->head + 12, &magic)) {
        free(f);
        return FONT_ERR_BUFFER_TOO_SHORT;
    }
    if (magic != 0x5F0F3CF5u) {
        free(f);
        return FONT_ERR_INVALID_HEAD_MAGIC;
    }
    f->data = (uint8_t *)malloc(len ? len : 1);
    if (!f->data) {
        free(f);
        return FONT_ERR_BUFFER_TOO_SHORT;
    }
    if (len) {
        memcpy(f->data, bytes, len);
    }
    f->len = len;
    *out = f;
    return FONT_OK;
}

void font_free(FontFile *f) {
    if (!f) {
        return;
    }
    free(f->data);
    free(f);
}

/* ── UTF-16 BE → UTF-8 decode into a bounded buffer ─────────────────────────*/
static void utf8_put(char *out, size_t cap, size_t *pos, uint32_t cp) {
    /* Encode one codepoint; silently drop if it would overflow the buffer
     * (leaving room for the terminating NUL the caller writes). */
    if (cp < 0x80) {
        if (*pos + 1 < cap) {
            out[(*pos)++] = (char)cp;
        }
    } else if (cp < 0x800) {
        if (*pos + 2 < cap) {
            out[(*pos)++] = (char)(0xC0 | (cp >> 6));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    } else if (cp < 0x10000) {
        if (*pos + 3 < cap) {
            out[(*pos)++] = (char)(0xE0 | (cp >> 12));
            out[(*pos)++] = (char)(0x80 | ((cp >> 6) & 0x3F));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    } else {
        if (*pos + 4 < cap) {
            out[(*pos)++] = (char)(0xF0 | (cp >> 18));
            out[(*pos)++] = (char)(0x80 | ((cp >> 12) & 0x3F));
            out[(*pos)++] = (char)(0x80 | ((cp >> 6) & 0x3F));
            out[(*pos)++] = (char)(0x80 | (cp & 0x3F));
        }
    }
}

static void utf16be_to_utf8(const uint8_t *raw, size_t rawlen, char *out,
                            size_t cap) {
    size_t i = 0, pos = 0;
    while (i + 2 <= rawlen) {
        uint32_t u = ((uint32_t)raw[i] << 8) | raw[i + 1];
        i += 2;
        if (u >= 0xD800 && u <= 0xDBFF) {
            /* High surrogate: pair with the following low surrogate. */
            if (i + 2 <= rawlen) {
                uint32_t lo = ((uint32_t)raw[i] << 8) | raw[i + 1];
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    i += 2;
                    u = 0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00);
                    utf8_put(out, cap, &pos, u);
                    continue;
                }
            }
            utf8_put(out, cap, &pos, 0xFFFD); /* unpaired high surrogate */
        } else if (u >= 0xDC00 && u <= 0xDFFF) {
            utf8_put(out, cap, &pos, 0xFFFD); /* unpaired low surrogate */
        } else {
            utf8_put(out, cap, &pos, u);
        }
    }
    out[pos] = '\0';
}

/* Read a `name`-table string by nameID into `out` (UTF-8). Returns 1 if found,
 * else 0. Prefers platform 3 / encoding 1 (UTF-16 BE), falls back to platform
 * 0. */
static int read_name(const uint8_t *buf, size_t len, int has_name,
                     uint32_t name_off, uint16_t name_id, char *out,
                     size_t cap) {
    uint16_t count, string_offset, i;
    size_t base;
    int found = 0, best_is_win = 0;
    size_t best_start = 0, best_len = 0;

    if (!has_name) {
        return 0;
    }
    base = (size_t)name_off;
    if (!font_read_u16(buf, len, base + 2, &count) ||
        !font_read_u16(buf, len, base + 4, &string_offset)) {
        return 0;
    }

    for (i = 0; i < count; i++) {
        size_t rec = base + 6 + (size_t)i * 12;
        uint16_t platform_id, encoding_id, nid, length, str_off;
        if (!font_read_u16(buf, len, rec, &platform_id) ||
            !font_read_u16(buf, len, rec + 2, &encoding_id) ||
            !font_read_u16(buf, len, rec + 6, &nid) ||
            !font_read_u16(buf, len, rec + 8, &length) ||
            !font_read_u16(buf, len, rec + 10, &str_off)) {
            return 0;
        }
        if (nid != name_id) {
            continue;
        }
        if (platform_id == 3 && encoding_id == 1) {
            best_start = base + (size_t)string_offset + str_off;
            best_len = length;
            best_is_win = 1;
            found = 1;
            break; /* best possible match */
        }
        if (platform_id == 0 && !found) {
            best_start = base + (size_t)string_offset + str_off;
            best_len = length;
            found = 1;
        }
    }
    (void)best_is_win;
    if (!found) {
        return 0;
    }
    /* Bounds-check the string span (Rust's buf.get(start..start+len)?). */
    if (best_start > len || len - best_start < best_len) {
        return 0;
    }
    utf16be_to_utf8(buf + best_start, best_len, out, cap);
    return 1;
}

/* ── font_metrics ───────────────────────────────────────────────────────────*/
void font_metrics(const FontFile *f, FontMetrics *out) {
    const uint8_t *buf = f->data;
    size_t len = f->len;
    uint16_t upm;
    int16_t hhea_asc = 0, hhea_desc = 0, hhea_gap = 0;
    uint16_t num_glyphs = 0;

    if (!font_read_u16(buf, len, (size_t)f->head + 18, &upm)) {
        upm = 1000;
    }
    out->units_per_em = upm;

    (void)font_read_i16(buf, len, (size_t)f->hhea + 4, &hhea_asc);
    (void)font_read_i16(buf, len, (size_t)f->hhea + 6, &hhea_desc);
    (void)font_read_i16(buf, len, (size_t)f->hhea + 8, &hhea_gap);
    (void)font_read_u16(buf, len, (size_t)f->maxp + 4, &num_glyphs);
    out->num_glyphs = num_glyphs;

    out->ascender = hhea_asc;
    out->descender = hhea_desc;
    out->line_gap = hhea_gap;
    out->has_x_height = 0;
    out->x_height = 0;
    out->has_cap_height = 0;
    out->cap_height = 0;

    if (f->has_os2) {
        size_t base = (size_t)f->os2;
        uint16_t version = 0;
        int16_t typo_asc = hhea_asc, typo_desc = hhea_desc, typo_gap = hhea_gap;
        (void)font_read_u16(buf, len, base, &version);
        (void)font_read_i16(buf, len, base + 68, &typo_asc);
        (void)font_read_i16(buf, len, base + 70, &typo_desc);
        (void)font_read_i16(buf, len, base + 72, &typo_gap);
        out->ascender = typo_asc;
        out->descender = typo_desc;
        out->line_gap = typo_gap;
        if (version >= 2) {
            int16_t xh, ch;
            if (font_read_i16(buf, len, base + 86, &xh)) {
                out->has_x_height = 1;
                out->x_height = xh;
            }
            if (font_read_i16(buf, len, base + 88, &ch)) {
                out->has_cap_height = 1;
                out->cap_height = ch;
            }
        }
    }

    if (!read_name(buf, len, f->has_name, f->name, 1, out->family_name,
                   FONT_NAME_CAP)) {
        strcpy(out->family_name, "(unknown)");
    }
    if (!read_name(buf, len, f->has_name, f->name, 2, out->subfamily_name,
                   FONT_NAME_CAP)) {
        strcpy(out->subfamily_name, "(unknown)");
    }
}

/* ── glyph_id — cmap Format 4 lookup ────────────────────────────────────────*/
int font_glyph_id(const FontFile *f, uint32_t codepoint, uint16_t *out) {
    const uint8_t *buf = f->data;
    size_t len = f->len;
    size_t cmap_off = (size_t)f->cmap;
    uint16_t cp, num_subtables, i, format, seg_count_x2;
    size_t seg_count, sub = 0;
    int have_sub = 0;
    size_t end_base, start_base, delta_base, range_base;
    size_t lo, hi;
    uint16_t end_code, start_code, id_range_offset;
    int16_t id_delta;
    uint16_t glyph;

    if (codepoint > 0xFFFF) {
        return 0;
    }
    cp = (uint16_t)codepoint;

    if (!font_read_u16(buf, len, cmap_off + 2, &num_subtables)) {
        return 0;
    }
    for (i = 0; i < num_subtables; i++) {
        size_t rec = cmap_off + 4 + (size_t)i * 8;
        uint16_t platform_id, encoding_id;
        uint32_t sub_offset;
        if (!font_read_u16(buf, len, rec, &platform_id) ||
            !font_read_u16(buf, len, rec + 2, &encoding_id) ||
            !font_read_u32(buf, len, rec + 4, &sub_offset)) {
            return 0;
        }
        if (platform_id == 3 && encoding_id == 1) {
            sub = cmap_off + sub_offset;
            have_sub = 1;
            break;
        }
        if (platform_id == 0 && !have_sub) {
            sub = cmap_off + sub_offset;
            have_sub = 1;
        }
    }
    if (!have_sub) {
        return 0;
    }

    if (!font_read_u16(buf, len, sub, &format) || format != 4) {
        return 0;
    }
    if (!font_read_u16(buf, len, sub + 6, &seg_count_x2)) {
        return 0;
    }
    seg_count = (size_t)seg_count_x2 / 2;

    end_base = sub + 14;
    start_base = sub + 16 + seg_count * 2;
    delta_base = sub + 16 + seg_count * 4;
    range_base = sub + 16 + seg_count * 6;

    /* Binary search for the first segment whose endCode >= codepoint. */
    lo = 0;
    hi = seg_count;
    while (lo < hi) {
        size_t mid = (lo + hi) / 2;
        uint16_t ec;
        if (!font_read_u16(buf, len, end_base + mid * 2, &ec)) {
            return 0;
        }
        if ((uint32_t)ec < codepoint) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if (lo >= seg_count) {
        return 0;
    }

    if (!font_read_u16(buf, len, end_base + lo * 2, &end_code) ||
        !font_read_u16(buf, len, start_base + lo * 2, &start_code)) {
        return 0;
    }
    if (cp > end_code || cp < start_code) {
        return 0;
    }
    if (!font_read_i16(buf, len, delta_base + lo * 2, &id_delta) ||
        !font_read_u16(buf, len, range_base + lo * 2, &id_range_offset)) {
        return 0;
    }

    if (id_range_offset == 0) {
        glyph = (uint16_t)(((int32_t)cp + (int32_t)id_delta) & 0xFFFF);
    } else {
        size_t abs_off = (range_base + lo * 2) + (size_t)id_range_offset +
                         (size_t)(cp - start_code) * 2;
        if (!font_read_u16(buf, len, abs_off, &glyph)) {
            return 0;
        }
    }
    if (glyph == 0) {
        return 0;
    }
    *out = glyph;
    return 1;
}

/* ── glyph_metrics — hmtx lookup ────────────────────────────────────────────*/
int font_glyph_metrics(const FontFile *f, uint16_t glyph_id,
                       GlyphMetrics *out) {
    const uint8_t *buf = f->data;
    size_t len = f->len;
    uint16_t num_glyphs, num_h_metrics;
    size_t hmtx_off = (size_t)f->hmtx, gid = glyph_id;

    if (!font_read_u16(buf, len, (size_t)f->maxp + 4, &num_glyphs) ||
        !font_read_u16(buf, len, (size_t)f->hhea + 34, &num_h_metrics)) {
        return 0;
    }
    if (gid >= num_glyphs) {
        return 0;
    }

    if (gid < num_h_metrics) {
        size_t base = hmtx_off + gid * 4;
        if (!font_read_u16(buf, len, base, &out->advance_width) ||
            !font_read_i16(buf, len, base + 2, &out->left_side_bearing)) {
            return 0;
        }
    } else {
        uint16_t last_advance;
        size_t lsb_off;
        if (num_h_metrics == 0) {
            return 0;
        }
        if (!font_read_u16(buf, len, hmtx_off + (size_t)(num_h_metrics - 1) * 4,
                           &last_advance)) {
            return 0;
        }
        lsb_off = hmtx_off + (size_t)num_h_metrics * 4 +
                  (gid - num_h_metrics) * 2;
        if (!font_read_i16(buf, len, lsb_off, &out->left_side_bearing)) {
            return 0;
        }
        out->advance_width = last_advance;
    }
    return 1;
}

/* ── kerning — kern Format 0 lookup ─────────────────────────────────────────*/
int16_t font_kerning(const FontFile *f, uint16_t left, uint16_t right) {
    const uint8_t *buf = f->data;
    size_t len = f->len;
    size_t kern_off, pos;
    uint16_t n_tables, t;

    if (!f->has_kern) {
        return 0;
    }
    kern_off = (size_t)f->kern;
    if (!font_read_u16(buf, len, kern_off + 2, &n_tables)) {
        return 0;
    }

    pos = kern_off + 4;
    for (t = 0; t < n_tables; t++) {
        uint16_t length, coverage, sub_format;
        if (pos > len || len - pos < 6) {
            break;
        }
        if (!font_read_u16(buf, len, pos + 2, &length) ||
            !font_read_u16(buf, len, pos + 4, &coverage)) {
            break;
        }
        sub_format = (uint16_t)(coverage >> 8);
        if (sub_format == 0) {
            uint16_t n_pairs;
            size_t pairs_base, lo, hi;
            uint32_t target;
            if (!font_read_u16(buf, len, pos + 6, &n_pairs)) {
                break;
            }
            pairs_base = pos + 14;
            target = ((uint32_t)left << 16) | right;
            lo = 0;
            hi = n_pairs;
            while (lo < hi) {
                size_t mid = (lo + hi) / 2;
                size_t pair_off = pairs_base + mid * 6;
                uint16_t pl, pr;
                uint32_t key;
                if (!font_read_u16(buf, len, pair_off, &pl) ||
                    !font_read_u16(buf, len, pair_off + 2, &pr)) {
                    break;
                }
                key = ((uint32_t)pl << 16) | pr;
                if (key == target) {
                    int16_t v = 0;
                    (void)font_read_i16(buf, len, pair_off + 4, &v);
                    return v;
                } else if (key < target) {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
        }
        pos += length;
    }
    return 0;
}
