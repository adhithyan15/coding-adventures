// Tests for font-parser. Like the C port, we build a synthetic in-memory
// OpenType font (the Rust tests load an external .ttf fixture) and exercise the
// full parser against it. Uses the header-only iso_test.h harness (pure ISO).
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "font_parser.hpp"

namespace fp = ca::font_parser;
using Bytes = std::vector<std::uint8_t>;

// ── Synthetic-font layout (absolute offsets, big-endian) ──────────────────
static constexpr std::size_t HEAD_OFF = 140, HHEA_OFF = 194, MAXP_OFF = 230,
                             CMAP_OFF = 236, HMTX_OFF = 288, KERN_OFF = 308,
                             NAME_OFF = 338, OS2_OFF = 390, FONT_LEN = 486;

static void pu16(Bytes& b, std::size_t o, std::uint16_t v) {
    b[o] = static_cast<std::uint8_t>(v >> 8);
    b[o + 1] = static_cast<std::uint8_t>(v);
}
static void pi16(Bytes& b, std::size_t o, std::int16_t v) {
    pu16(b, o, static_cast<std::uint16_t>(v));
}
static void pu32(Bytes& b, std::size_t o, std::uint32_t v) {
    b[o] = static_cast<std::uint8_t>(v >> 24);
    b[o + 1] = static_cast<std::uint8_t>(v >> 16);
    b[o + 2] = static_cast<std::uint8_t>(v >> 8);
    b[o + 3] = static_cast<std::uint8_t>(v);
}
static void prec(Bytes& b, std::size_t o, const char* tag, std::uint32_t off,
                 std::uint32_t len) {
    for (int i = 0; i < 4; ++i) b[o + i] = static_cast<std::uint8_t>(tag[i]);
    pu32(b, o + 4, 0);
    pu32(b, o + 8, off);
    pu32(b, o + 12, len);
}

static Bytes build_font() {
    Bytes b(FONT_LEN, 0);
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

    pu32(b, HEAD_OFF, 0x00010000u);
    pu32(b, HEAD_OFF + 12, 0x5F0F3CF5u);
    pu16(b, HEAD_OFF + 18, 1000);

    pi16(b, HHEA_OFF + 4, 800);
    pi16(b, HHEA_OFF + 6, -200);
    pi16(b, HHEA_OFF + 8, 0);
    pu16(b, HHEA_OFF + 34, 5);

    pu32(b, MAXP_OFF, 0x00005000u);
    pu16(b, MAXP_OFF + 4, 5);

    pu16(b, CMAP_OFF, 0);
    pu16(b, CMAP_OFF + 2, 1);
    pu16(b, CMAP_OFF + 4, 3);
    pu16(b, CMAP_OFF + 6, 1);
    pu32(b, CMAP_OFF + 8, 12);
    {
        std::size_t s = CMAP_OFF + 12;
        pu16(b, s, 4);
        pu16(b, s + 2, 40);
        pu16(b, s + 4, 0);
        pu16(b, s + 6, 6);
        pu16(b, s + 14, 0x20);
        pu16(b, s + 16, 0x42);
        pu16(b, s + 18, 0xFFFF);
        pu16(b, s + 22, 0x20);
        pu16(b, s + 24, 0x41);
        pu16(b, s + 26, 0xFFFF);
        pi16(b, s + 28, -29);
        pi16(b, s + 30, -64);
        pi16(b, s + 32, 1);
    }

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

    pu16(b, KERN_OFF, 0);
    pu16(b, KERN_OFF + 2, 1);
    pu16(b, KERN_OFF + 4, 0);
    pu16(b, KERN_OFF + 6, 26);
    pu16(b, KERN_OFF + 8, 0x0001);
    pu16(b, KERN_OFF + 10, 2);
    pu16(b, KERN_OFF + 18, 1);
    pu16(b, KERN_OFF + 20, 2);
    pi16(b, KERN_OFF + 22, -140);
    pu16(b, KERN_OFF + 24, 3);
    pu16(b, KERN_OFF + 26, 4);
    pi16(b, KERN_OFF + 28, 80);

    pu16(b, NAME_OFF, 0);
    pu16(b, NAME_OFF + 2, 2);
    pu16(b, NAME_OFF + 4, 30);
    pu16(b, NAME_OFF + 6, 3);
    pu16(b, NAME_OFF + 8, 1);
    pu16(b, NAME_OFF + 10, 0);
    pu16(b, NAME_OFF + 12, 1);
    pu16(b, NAME_OFF + 14, 8);
    pu16(b, NAME_OFF + 16, 0);
    pu16(b, NAME_OFF + 18, 3);
    pu16(b, NAME_OFF + 20, 1);
    pu16(b, NAME_OFF + 22, 0);
    pu16(b, NAME_OFF + 24, 2);
    pu16(b, NAME_OFF + 26, 14);
    pu16(b, NAME_OFF + 28, 8);
    {
        std::size_t s = NAME_OFF + 30;
        const char* fam = "Test";
        const char* sub = "Regular";
        for (std::size_t i = 0; fam[i]; ++i)
            pu16(b, s + i * 2, static_cast<std::uint16_t>(fam[i]));
        s += 8;
        for (std::size_t i = 0; sub[i]; ++i)
            pu16(b, s + i * 2, static_cast<std::uint16_t>(sub[i]));
    }

    pu16(b, OS2_OFF, 2);
    pi16(b, OS2_OFF + 68, 750);
    pi16(b, OS2_OFF + 70, -250);
    pi16(b, OS2_OFF + 72, 0);
    pi16(b, OS2_OFF + 86, 500);
    pi16(b, OS2_OFF + 88, 700);
    return b;
}

// Try to load; return the caught error (or nullopt on success).
static std::optional<fp::FontError> load_err(const Bytes& b) {
    try {
        fp::FontFile::load(b);
        return std::nullopt;
    } catch (fp::FontError e) {
        return e;
    }
}

int main() {
    // ── read helpers ──────────────────────────────────────────────────────
    {
        Bytes b2 = {0x08, 0x00};
        ISO_CHECK(fp::detail::read_u16(b2, 0).value() == 0x0800);
        Bytes bn = {0xFE, 0x00};
        ISO_CHECK(fp::detail::read_i16(bn, 0).value() == -512);
        Bytes one = {0x08};
        ISO_CHECK(!fp::detail::read_u16(one, 0).has_value());
        Bytes b4 = {0x00, 0x01, 0x00, 0x00};
        ISO_CHECK(fp::detail::read_u32(b4, 0).value() == 0x00010000u);
    }

    // ── load error cases ──────────────────────────────────────────────────
    {
        ISO_CHECK(load_err(Bytes{}) == fp::FontError::BufferTooShort);
        Bytes bad(28, 0);
        bad[0] = 0xDE;
        bad[1] = 0xAD;
        bad[2] = 0xBE;
        bad[3] = 0xEF;
        ISO_CHECK(load_err(bad) == fp::FontError::InvalidMagic);
        Bytes bhm = build_font();
        pu32(bhm, HEAD_OFF + 12, 0x12345678u);
        ISO_CHECK(load_err(bhm) == fp::FontError::InvalidHeadMagic);
        Bytes miss = build_font();
        for (int i = 0; i < 4; ++i) miss[28 + i] = 'z'; // clobber hhea record
        ISO_CHECK(load_err(miss) == fp::FontError::TableNotFound);
    }

    // ── full parse ────────────────────────────────────────────────────────
    {
        auto f = fp::FontFile::load(build_font());
        auto m = f.metrics();
        ISO_CHECK_EQ_UINT(m.units_per_em, 1000u);
        ISO_CHECK_EQ_INT(m.ascender, 750);
        ISO_CHECK_EQ_INT(m.descender, -250);
        ISO_CHECK_EQ_INT(m.line_gap, 0);
        ISO_CHECK(m.x_height.has_value() && *m.x_height == 500);
        ISO_CHECK(m.cap_height.has_value() && *m.cap_height == 700);
        ISO_CHECK_EQ_UINT(m.num_glyphs, 5u);
        ISO_CHECK(m.family_name == "Test");
        ISO_CHECK(m.subfamily_name == "Regular");

        ISO_CHECK(f.glyph_id(0x41) == std::optional<std::uint16_t>(1));
        ISO_CHECK(f.glyph_id(0x42) == std::optional<std::uint16_t>(2));
        ISO_CHECK(f.glyph_id(0x20) == std::optional<std::uint16_t>(3));
        ISO_CHECK(!f.glyph_id(0x5A).has_value());
        ISO_CHECK(!f.glyph_id(0x10000).has_value());
        ISO_CHECK(!f.glyph_id(0xFFFF).has_value());

        auto gm = f.glyph_metrics(1);
        ISO_CHECK(gm.has_value());
        ISO_CHECK_EQ_UINT(gm->advance_width, 650u);
        ISO_CHECK_EQ_INT(gm->left_side_bearing, 50);
        ISO_CHECK(f.glyph_metrics(3)->advance_width == 300);
        ISO_CHECK(!f.glyph_metrics(5).has_value());

        ISO_CHECK_EQ_INT(f.kerning(1, 2), -140);
        ISO_CHECK_EQ_INT(f.kerning(3, 4), 80);
        ISO_CHECK_EQ_INT(f.kerning(1, 4), 0);
    }

    // ── shared-advance hmtx path ──────────────────────────────────────────
    {
        Bytes b = build_font();
        pu16(b, HHEA_OFF + 34, 2);
        pi16(b, HMTX_OFF + 8, 11);
        pi16(b, HMTX_OFF + 10, 12);
        pi16(b, HMTX_OFF + 12, 13);
        auto f = fp::FontFile::load(b);
        auto gm = f.glyph_metrics(3);
        ISO_CHECK(gm.has_value());
        ISO_CHECK_EQ_UINT(gm->advance_width, 650u);
        ISO_CHECK_EQ_INT(gm->left_side_bearing, 12);
    }

    // ── OTTO magic accepted ───────────────────────────────────────────────
    {
        Bytes b = build_font();
        pu32(b, 0, 0x4F54544Fu);
        ISO_CHECK(!load_err(b).has_value());
    }

    // ── truncation fuzz: every prefix parses without OOB ──────────────────
    {
        Bytes b = build_font();
        for (std::size_t n = 0; n <= FONT_LEN; ++n) {
            Bytes prefix(b.begin(), b.begin() + static_cast<std::ptrdiff_t>(n));
            try {
                auto f = fp::FontFile::load(prefix);
                (void)f.metrics();
                (void)f.glyph_id(0x41);
                (void)f.glyph_metrics(1);
                (void)f.kerning(1, 2);
            } catch (fp::FontError&) {
                // expected for short/invalid prefixes
            }
        }
        ISO_CHECK(true);
    }

    return ISO_TEST_RESULT();
}
