/*
 * Tests for font-parser. The Rust crate's tests load an external Inter-Regular
 * .ttf fixture; to stay self-contained we exercise the full parser against a
 * synthetic in-memory OpenType font (mirroring the crate's own synthetic-font
 * builder, extended with cmap segments, name and OS/2 tables). Uses the
 * header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "font_parser.h"

#include <string.h>

/* ── Synthetic-font layout (all offsets absolute, big-endian) ────────────── */
#define HEAD_OFF 140
#define HHEA_OFF 194
#define MAXP_OFF 230
#define CMAP_OFF 236
#define HMTX_OFF 288
#define KERN_OFF 308
#define NAME_OFF 338
#define OS2_OFF 390
#define FONT_LEN 486

static void pu16(uint8_t *b, size_t o, uint16_t v) {
    b[o] = (uint8_t)(v >> 8);
    b[o + 1] = (uint8_t)v;
}
static void pi16(uint8_t *b, size_t o, int16_t v) { pu16(b, o, (uint16_t)v); }
static void pu32(uint8_t *b, size_t o, uint32_t v) {
    b[o] = (uint8_t)(v >> 24);
    b[o + 1] = (uint8_t)(v >> 16);
    b[o + 2] = (uint8_t)(v >> 8);
    b[o + 3] = (uint8_t)v;
}
static void prec(uint8_t *b, size_t o, const char *tag, uint32_t off,
                 uint32_t len) {
    memcpy(b + o, tag, 4);
    pu32(b, o + 4, 0); /* checksum */
    pu32(b, o + 8, off);
    pu32(b, o + 12, len);
}

/* Build the synthetic font into `b` (>= FONT_LEN bytes). Returns FONT_LEN.
 *   head: unitsPerEm=1000        hhea: asc=800 desc=-200 gap=0 numHMetrics=5
 *   maxp: numGlyphs=5            cmap: 'A'->1 'B'->2 ' '->3 (+sentinel)
 *   hmtx: advances 600/650/680/300/600, lsb 50
 *   kern: (1,2)->-140 (3,4)->80  name: family "Test" subfamily "Regular"
 *   OS/2 v2: typoAsc=750 typoDesc=-250 gap=0 xHeight=500 capHeight=700 */
static size_t build_font(uint8_t *b) {
    memset(b, 0, FONT_LEN);

    /* Offset table + directory (8 tables). */
    pu32(b, 0, 0x00010000u);
    pu16(b, 4, 8);
    prec(b, 12, "head", HEAD_OFF, 54);
    prec(b, 28, "hhea", HHEA_OFF, 36);
    prec(b, 44, "maxp", MAXP_OFF, 6);
    prec(b, 60, "cmap", CMAP_OFF, 52);
    prec(b, 76, "hmtx", HMTX_OFF, 20);
    prec(b, 92, "kern", KERN_OFF, 30);
    prec(b, 108, "name", NAME_OFF, 52);
    prec(b, 124, "OS/2", OS2_OFF, 96);

    /* head */
    pu32(b, HEAD_OFF, 0x00010000u);
    pu32(b, HEAD_OFF + 12, 0x5F0F3CF5u);
    pu16(b, HEAD_OFF + 18, 1000);

    /* hhea */
    pi16(b, HHEA_OFF + 4, 800);
    pi16(b, HHEA_OFF + 6, -200);
    pi16(b, HHEA_OFF + 8, 0);
    pu16(b, HHEA_OFF + 34, 5);

    /* maxp */
    pu32(b, MAXP_OFF, 0x00005000u);
    pu16(b, MAXP_OFF + 4, 5);

    /* cmap: index + one encoding record + Format 4 subtable (segCount=3). */
    pu16(b, CMAP_OFF, 0);
    pu16(b, CMAP_OFF + 2, 1);
    pu16(b, CMAP_OFF + 4, 3);       /* platform 3 */
    pu16(b, CMAP_OFF + 6, 1);       /* encoding 1 */
    pu32(b, CMAP_OFF + 8, 12);      /* subtable offset from cmap start */
    {
        size_t s = CMAP_OFF + 12;   /* subtable base = 248 */
        pu16(b, s, 4);              /* format */
        pu16(b, s + 2, 40);         /* length */
        pu16(b, s + 4, 0);          /* language */
        pu16(b, s + 6, 6);          /* segCountX2 (segCount = 3) */
        /* searchRange/entrySelector/rangeShift left 0 */
        /* endCode[3] @ s+14 */
        pu16(b, s + 14, 0x20);
        pu16(b, s + 16, 0x42);
        pu16(b, s + 18, 0xFFFF);
        /* reservedPad @ s+20 = 0 */
        /* startCode[3] @ s+22 */
        pu16(b, s + 22, 0x20);
        pu16(b, s + 24, 0x41);
        pu16(b, s + 26, 0xFFFF);
        /* idDelta[3] @ s+28 */
        pi16(b, s + 28, -29); /* ' '(0x20)+(-29)=3 */
        pi16(b, s + 30, -64); /* 'A'(0x41)-64=1, 'B'-64=2 */
        pi16(b, s + 32, 1);   /* sentinel -> (0xFFFF+1)&0xFFFF=0 -> None */
        /* idRangeOffset[3] @ s+34 all 0 */
    }

    /* hmtx: 5 full records */
    pu16(b, HMTX_OFF + 0, 600);
    pi16(b, HMTX_OFF + 2, 50);
    pu16(b, HMTX_OFF + 4, 650);
    pi16(b, HMTX_OFF + 6, 50);
    pu16(b, HMTX_OFF + 8, 680);
    pi16(b, HMTX_OFF + 10, 50);
    pu16(b, HMTX_OFF + 12, 300);
    pi16(b, HMTX_OFF + 14, 50);
    pu16(b, HMTX_OFF + 16, 600);
    pi16(b, HMTX_OFF + 18, 50);

    /* kern: header + one Format 0 subtable with two pairs */
    pu16(b, KERN_OFF, 0);      /* version */
    pu16(b, KERN_OFF + 2, 1);  /* nTables */
    pu16(b, KERN_OFF + 4, 0);  /* subtable version */
    pu16(b, KERN_OFF + 6, 26); /* subtable length */
    pu16(b, KERN_OFF + 8, 0x0001); /* coverage: format 0, horizontal */
    pu16(b, KERN_OFF + 10, 2); /* nPairs */
    /* searchRange/entrySelector/rangeShift @ +12/+14/+16 = 0 */
    pu16(b, KERN_OFF + 18, 1); /* pair (1,2) */
    pu16(b, KERN_OFF + 20, 2);
    pi16(b, KERN_OFF + 22, -140);
    pu16(b, KERN_OFF + 24, 3); /* pair (3,4) */
    pu16(b, KERN_OFF + 26, 4);
    pi16(b, KERN_OFF + 28, 80);

    /* name: header + 2 records + UTF-16BE storage */
    pu16(b, NAME_OFF, 0);      /* format */
    pu16(b, NAME_OFF + 2, 2);  /* count */
    pu16(b, NAME_OFF + 4, 30); /* stringOffset (6 + 24) */
    /* record 0: family (nameID 1), plat 3 enc 1, len 8, off 0 */
    pu16(b, NAME_OFF + 6, 3);
    pu16(b, NAME_OFF + 8, 1);
    pu16(b, NAME_OFF + 10, 0);
    pu16(b, NAME_OFF + 12, 1);
    pu16(b, NAME_OFF + 14, 8);
    pu16(b, NAME_OFF + 16, 0);
    /* record 1: subfamily (nameID 2), len 14, off 8 */
    pu16(b, NAME_OFF + 18, 3);
    pu16(b, NAME_OFF + 20, 1);
    pu16(b, NAME_OFF + 22, 0);
    pu16(b, NAME_OFF + 24, 2);
    pu16(b, NAME_OFF + 26, 14);
    pu16(b, NAME_OFF + 28, 8);
    /* storage @ NAME_OFF+30 = 368: "Test" then "Regular" (UTF-16 BE) */
    {
        size_t s = NAME_OFF + 30;
        const char *fam = "Test";
        const char *sub = "Regular";
        size_t i;
        for (i = 0; fam[i]; i++) {
            pu16(b, s + i * 2, (uint16_t)(unsigned char)fam[i]);
        }
        s += 8;
        for (i = 0; sub[i]; i++) {
            pu16(b, s + i * 2, (uint16_t)(unsigned char)sub[i]);
        }
    }

    /* OS/2 version 2 */
    pu16(b, OS2_OFF, 2);
    pi16(b, OS2_OFF + 68, 750);
    pi16(b, OS2_OFF + 70, -250);
    pi16(b, OS2_OFF + 72, 0);
    pi16(b, OS2_OFF + 86, 500);
    pi16(b, OS2_OFF + 88, 700);

    return FONT_LEN;
}

int main(void) {
    /* ── read helpers ─────────────────────────────────────────────────────── */
    {
        uint8_t buf2[2] = {0x08, 0x00};
        uint16_t u;
        int16_t s;
        ISO_CHECK(font_read_u16(buf2, 2, 0, &u));
        ISO_CHECK_EQ_UINT(u, 0x0800); /* 2048 */
        buf2[0] = 0xFE;
        buf2[1] = 0x00;
        ISO_CHECK(font_read_i16(buf2, 2, 0, &s));
        ISO_CHECK_EQ_INT(s, -512);
    }
    {
        uint8_t one[1] = {0x08};
        uint16_t u;
        ISO_CHECK(!font_read_u16(one, 1, 0, &u)); /* out of bounds */
    }
    {
        uint8_t buf4[4] = {0x00, 0x01, 0x00, 0x00};
        uint32_t v;
        ISO_CHECK(font_read_u32(buf4, 4, 0, &v));
        ISO_CHECK_EQ_UINT(v, 0x00010000u);
    }

    /* ── load error cases ─────────────────────────────────────────────────── */
    {
        FontFile *f = NULL;
        uint8_t empty[1] = {0};
        ISO_CHECK_EQ_INT(font_load(empty, 0, &f), FONT_ERR_BUFFER_TOO_SHORT);
        ISO_CHECK(f == NULL);
    }
    {
        FontFile *f = NULL;
        uint8_t buf[28];
        memset(buf, 0, sizeof buf);
        buf[0] = 0xDE;
        buf[1] = 0xAD;
        buf[2] = 0xBE;
        buf[3] = 0xEF;
        ISO_CHECK_EQ_INT(font_load(buf, sizeof buf, &f), FONT_ERR_INVALID_MAGIC);
    }
    {
        /* Valid directory but wrong head.magicNumber. */
        uint8_t b[FONT_LEN];
        FontFile *f = NULL;
        build_font(b);
        pu32(b, HEAD_OFF + 12, 0x12345678u); /* corrupt head magic */
        ISO_CHECK_EQ_INT(font_load(b, FONT_LEN, &f),
                         FONT_ERR_INVALID_HEAD_MAGIC);
    }

    /* ── full parse of the synthetic font ─────────────────────────────────── */
    {
        uint8_t b[FONT_LEN];
        FontFile *f = NULL;
        FontMetrics m;
        GlyphMetrics gm;
        uint16_t gid;
        build_font(b);

        ISO_CHECK_EQ_INT(font_load(b, FONT_LEN, &f), FONT_OK);
        ISO_CHECK(f != NULL);

        font_metrics(f, &m);
        ISO_CHECK_EQ_UINT(m.units_per_em, 1000);
        ISO_CHECK_EQ_INT(m.ascender, 750);   /* OS/2 typo preferred over hhea */
        ISO_CHECK_EQ_INT(m.descender, -250);
        ISO_CHECK_EQ_INT(m.line_gap, 0);
        ISO_CHECK(m.has_x_height);
        ISO_CHECK_EQ_INT(m.x_height, 500);
        ISO_CHECK(m.has_cap_height);
        ISO_CHECK_EQ_INT(m.cap_height, 700);
        ISO_CHECK_EQ_UINT(m.num_glyphs, 5);
        ISO_CHECK_STR_EQ(m.family_name, "Test");
        ISO_CHECK_STR_EQ(m.subfamily_name, "Regular");

        /* glyph_id via cmap Format 4 */
        ISO_CHECK(font_glyph_id(f, 0x41, &gid) && gid == 1); /* 'A' */
        ISO_CHECK(font_glyph_id(f, 0x42, &gid) && gid == 2); /* 'B' */
        ISO_CHECK(font_glyph_id(f, 0x20, &gid) && gid == 3); /* ' ' */
        ISO_CHECK(!font_glyph_id(f, 0x5A, &gid));    /* 'Z' unmapped */
        ISO_CHECK(!font_glyph_id(f, 0x10000, &gid)); /* above BMP */
        ISO_CHECK(!font_glyph_id(f, 0xFFFF, &gid));  /* sentinel -> None */

        /* glyph_metrics */
        ISO_CHECK(font_glyph_metrics(f, 1, &gm));
        ISO_CHECK_EQ_UINT(gm.advance_width, 650);
        ISO_CHECK_EQ_INT(gm.left_side_bearing, 50);
        ISO_CHECK(font_glyph_metrics(f, 3, &gm) && gm.advance_width == 300);
        ISO_CHECK(!font_glyph_metrics(f, 5, &gm)); /* == numGlyphs: out of range */

        /* kerning */
        ISO_CHECK_EQ_INT(font_kerning(f, 1, 2), -140);
        ISO_CHECK_EQ_INT(font_kerning(f, 3, 4), 80);
        ISO_CHECK_EQ_INT(font_kerning(f, 1, 4), 0); /* absent pair */

        font_free(f);
    }

    /* ── shared-advance hmtx path (numberOfHMetrics < numGlyphs) ───────────── */
    {
        uint8_t b[FONT_LEN];
        FontFile *f = NULL;
        GlyphMetrics gm;
        build_font(b);
        /* Shrink numberOfHMetrics to 2 but keep numGlyphs 5, and extend hmtx so
         * glyphs 2..4 are lsb-only records sharing glyph 1's advance (650). The
         * lsb-only array starts after 2 full records (8 bytes) at HMTX_OFF+8. */
        pu16(b, HHEA_OFF + 34, 2);
        pi16(b, HMTX_OFF + 8, 11);  /* glyph 2 lsb */
        pi16(b, HMTX_OFF + 10, 12); /* glyph 3 lsb */
        pi16(b, HMTX_OFF + 12, 13); /* glyph 4 lsb */
        ISO_CHECK_EQ_INT(font_load(b, FONT_LEN, &f), FONT_OK);
        ISO_CHECK(font_glyph_metrics(f, 3, &gm));
        ISO_CHECK_EQ_UINT(gm.advance_width, 650); /* shared last advance */
        ISO_CHECK_EQ_INT(gm.left_side_bearing, 12);
        font_free(f);
    }

    /* ── OTTO magic is accepted ───────────────────────────────────────────── */
    {
        uint8_t b[FONT_LEN];
        FontFile *f = NULL;
        build_font(b);
        pu32(b, 0, 0x4F54544Fu); /* "OTTO" */
        ISO_CHECK_EQ_INT(font_load(b, FONT_LEN, &f), FONT_OK);
        font_free(f);
    }

    /* ── missing required table -> error ──────────────────────────────────── */
    {
        uint8_t b[FONT_LEN];
        FontFile *f = NULL;
        build_font(b);
        memcpy(b + 28, "zzzz", 4); /* clobber the hhea record's tag */
        ISO_CHECK_EQ_INT(font_load(b, FONT_LEN, &f), FONT_ERR_TABLE_NOT_FOUND);
        ISO_CHECK(f == NULL);
    }

    /* ── truncation fuzz: every prefix parses without OOB ─────────────────── */
    {
        uint8_t b[FONT_LEN];
        size_t n;
        build_font(b);
        for (n = 0; n <= FONT_LEN; n++) {
            FontFile *f = NULL;
            if (font_load(b, n, &f) == FONT_OK) {
                FontMetrics m;
                uint16_t gid;
                GlyphMetrics gm;
                font_metrics(f, &m);            /* must not read OOB */
                (void)font_glyph_id(f, 0x41, &gid);
                (void)font_glyph_metrics(f, 1, &gm);
                (void)font_kerning(f, 1, 2);
                font_free(f);
            }
        }
        ISO_CHECK(1); /* survived all prefixes */
    }

    return ISO_TEST_RESULT();
}
