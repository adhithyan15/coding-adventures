// font_parser.hpp — Metrics-only OpenType/TrueType font parser, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `font-parser` crate, in namespace
// `ca::font_parser`. An OpenType font is a binary table database; this parser
// reads the subset needed to *measure text* (head/hhea/maxp/cmap/hmtx/kern/
// name/OS-2) without touching the OS font stack. It does not parse outlines,
// shape text, or rasterize.
//
// `FontFile::load` copies the bytes and pre-parses the table directory; every
// query is bounds-checked integer arithmetic over that buffer. Where the Rust
// `load` returns `Result`, this port throws `FontError`. Pure ISO C++17.

#ifndef FONT_PARSER_HPP
#define FONT_PARSER_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace font_parser {

// Parsing errors (thrown by FontFile::load).
enum class FontError {
    InvalidMagic,
    InvalidHeadMagic,
    TableNotFound,
    BufferTooShort,
    UnsupportedCmapFormat,
};

inline const char* to_string(FontError e) {
    switch (e) {
    case FontError::InvalidMagic:
        return "invalid sfntVersion magic";
    case FontError::InvalidHeadMagic:
        return "invalid head.magicNumber";
    case FontError::TableNotFound:
        return "required table not found";
    case FontError::BufferTooShort:
        return "buffer too short";
    case FontError::UnsupportedCmapFormat:
        return "no Format 4 cmap subtable for platform 3 encoding 1";
    }
    return "unknown error";
}

// Global typographic metrics (design units unless noted).
struct FontMetrics {
    std::uint16_t units_per_em = 0;
    std::int16_t ascender = 0;
    std::int16_t descender = 0;
    std::int16_t line_gap = 0;
    std::optional<std::int16_t> x_height;
    std::optional<std::int16_t> cap_height;
    std::uint16_t num_glyphs = 0;
    std::string family_name;
    std::string subfamily_name;
};

// Per-glyph horizontal metrics (design units).
struct GlyphMetrics {
    std::uint16_t advance_width = 0;
    std::int16_t left_side_bearing = 0;
};

namespace detail {
inline std::optional<std::uint16_t> read_u16(const std::vector<std::uint8_t>& b,
                                             std::size_t off) {
    if (off > b.size() || b.size() - off < 2) {
        return std::nullopt;
    }
    return static_cast<std::uint16_t>((static_cast<std::uint16_t>(b[off]) << 8) |
                                      b[off + 1]);
}
inline std::optional<std::int16_t> read_i16(const std::vector<std::uint8_t>& b,
                                            std::size_t off) {
    auto v = read_u16(b, off);
    if (!v) {
        return std::nullopt;
    }
    return static_cast<std::int16_t>(*v);
}
inline std::optional<std::uint32_t> read_u32(const std::vector<std::uint8_t>& b,
                                             std::size_t off) {
    if (off > b.size() || b.size() - off < 4) {
        return std::nullopt;
    }
    return (static_cast<std::uint32_t>(b[off]) << 24) |
           (static_cast<std::uint32_t>(b[off + 1]) << 16) |
           (static_cast<std::uint32_t>(b[off + 2]) << 8) |
           static_cast<std::uint32_t>(b[off + 3]);
}

inline void utf8_put(std::string& out, std::uint32_t cp) {
    if (cp < 0x80) {
        out.push_back(static_cast<char>(cp));
    } else if (cp < 0x800) {
        out.push_back(static_cast<char>(0xC0 | (cp >> 6)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else if (cp < 0x10000) {
        out.push_back(static_cast<char>(0xE0 | (cp >> 12)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else {
        out.push_back(static_cast<char>(0xF0 | (cp >> 18)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    }
}

// Decode a UTF-16 BE byte span to a UTF-8 std::string (unpaired surrogates
// become U+FFFD, mirroring Rust's char::decode_utf16).
inline std::string utf16be_to_utf8(const std::vector<std::uint8_t>& b,
                                   std::size_t start, std::size_t len) {
    std::string out;
    std::size_t i = start, end = start + len;
    while (i + 2 <= end) {
        std::uint32_t u = (static_cast<std::uint32_t>(b[i]) << 8) | b[i + 1];
        i += 2;
        if (u >= 0xD800 && u <= 0xDBFF) {
            if (i + 2 <= end) {
                std::uint32_t lo =
                    (static_cast<std::uint32_t>(b[i]) << 8) | b[i + 1];
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    i += 2;
                    utf8_put(out, 0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00));
                    continue;
                }
            }
            utf8_put(out, 0xFFFD);
        } else if (u >= 0xDC00 && u <= 0xDFFF) {
            utf8_put(out, 0xFFFD);
        } else {
            utf8_put(out, u);
        }
    }
    return out;
}
}  // namespace detail

class FontFile {
  public:
    // Parse raw font bytes. Throws FontError on failure.
    static FontFile load(const std::vector<std::uint8_t>& bytes) {
        Tables t = parse_table_directory(bytes);
        auto magic = detail::read_u32(bytes, static_cast<std::size_t>(t.head) + 12);
        if (!magic) {
            throw FontError::BufferTooShort;
        }
        if (*magic != 0x5F0F3CF5u) {
            throw FontError::InvalidHeadMagic;
        }
        FontFile f;
        f.data_ = bytes;
        f.tables_ = t;
        return f;
    }

    FontMetrics metrics() const {
        const auto& buf = data_;
        FontMetrics m;
        m.units_per_em =
            detail::read_u16(buf, static_cast<std::size_t>(tables_.head) + 18)
                .value_or(1000);

        std::int16_t hhea_asc =
            detail::read_i16(buf, static_cast<std::size_t>(tables_.hhea) + 4)
                .value_or(0);
        std::int16_t hhea_desc =
            detail::read_i16(buf, static_cast<std::size_t>(tables_.hhea) + 6)
                .value_or(0);
        std::int16_t hhea_gap =
            detail::read_i16(buf, static_cast<std::size_t>(tables_.hhea) + 8)
                .value_or(0);
        m.num_glyphs =
            detail::read_u16(buf, static_cast<std::size_t>(tables_.maxp) + 4)
                .value_or(0);

        m.ascender = hhea_asc;
        m.descender = hhea_desc;
        m.line_gap = hhea_gap;

        if (tables_.os2) {
            std::size_t base = *tables_.os2;
            std::uint16_t version = detail::read_u16(buf, base).value_or(0);
            m.ascender = detail::read_i16(buf, base + 68).value_or(hhea_asc);
            m.descender = detail::read_i16(buf, base + 70).value_or(hhea_desc);
            m.line_gap = detail::read_i16(buf, base + 72).value_or(hhea_gap);
            if (version >= 2) {
                m.x_height = detail::read_i16(buf, base + 86);
                m.cap_height = detail::read_i16(buf, base + 88);
            }
        }

        m.family_name = read_name(1).value_or("(unknown)");
        m.subfamily_name = read_name(2).value_or("(unknown)");
        return m;
    }

    std::optional<std::uint16_t> glyph_id(std::uint32_t codepoint) const {
        if (codepoint > 0xFFFF) {
            return std::nullopt;
        }
        auto cp = static_cast<std::uint16_t>(codepoint);
        const auto& buf = data_;
        std::size_t cmap_off = tables_.cmap;

        auto num_subtables = detail::read_u16(buf, cmap_off + 2);
        if (!num_subtables) {
            return std::nullopt;
        }
        std::optional<std::size_t> sub;
        for (std::uint16_t i = 0; i < *num_subtables; ++i) {
            std::size_t rec = cmap_off + 4 + static_cast<std::size_t>(i) * 8;
            auto platform_id = detail::read_u16(buf, rec);
            auto encoding_id = detail::read_u16(buf, rec + 2);
            auto sub_offset = detail::read_u32(buf, rec + 4);
            if (!platform_id || !encoding_id || !sub_offset) {
                return std::nullopt;
            }
            if (*platform_id == 3 && *encoding_id == 1) {
                sub = cmap_off + *sub_offset;
                break;
            }
            if (*platform_id == 0 && !sub) {
                sub = cmap_off + *sub_offset;
            }
        }
        if (!sub) {
            return std::nullopt;
        }

        auto format = detail::read_u16(buf, *sub);
        if (!format || *format != 4) {
            return std::nullopt;
        }
        auto seg_count_x2 = detail::read_u16(buf, *sub + 6);
        if (!seg_count_x2) {
            return std::nullopt;
        }
        std::size_t seg_count = *seg_count_x2 / 2;
        std::size_t end_base = *sub + 14;
        std::size_t start_base = *sub + 16 + seg_count * 2;
        std::size_t delta_base = *sub + 16 + seg_count * 4;
        std::size_t range_base = *sub + 16 + seg_count * 6;

        std::size_t lo = 0, hi = seg_count;
        while (lo < hi) {
            std::size_t mid = (lo + hi) / 2;
            auto ec = detail::read_u16(buf, end_base + mid * 2);
            if (!ec) {
                return std::nullopt;
            }
            if (static_cast<std::uint32_t>(*ec) < codepoint) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if (lo >= seg_count) {
            return std::nullopt;
        }

        auto end_code = detail::read_u16(buf, end_base + lo * 2);
        auto start_code = detail::read_u16(buf, start_base + lo * 2);
        if (!end_code || !start_code) {
            return std::nullopt;
        }
        if (cp > *end_code || cp < *start_code) {
            return std::nullopt;
        }
        auto id_delta = detail::read_i16(buf, delta_base + lo * 2);
        auto id_range_offset = detail::read_u16(buf, range_base + lo * 2);
        if (!id_delta || !id_range_offset) {
            return std::nullopt;
        }

        std::uint16_t glyph;
        if (*id_range_offset == 0) {
            glyph = static_cast<std::uint16_t>(
                (static_cast<std::int32_t>(cp) + *id_delta) & 0xFFFF);
        } else {
            std::size_t abs_off = (range_base + lo * 2) +
                                  static_cast<std::size_t>(*id_range_offset) +
                                  static_cast<std::size_t>(cp - *start_code) * 2;
            auto g = detail::read_u16(buf, abs_off);
            if (!g) {
                return std::nullopt;
            }
            glyph = *g;
        }
        if (glyph == 0) {
            return std::nullopt;
        }
        return glyph;
    }

    std::optional<GlyphMetrics> glyph_metrics(std::uint16_t glyph_id) const {
        const auto& buf = data_;
        auto num_glyphs =
            detail::read_u16(buf, static_cast<std::size_t>(tables_.maxp) + 4);
        auto num_h_metrics =
            detail::read_u16(buf, static_cast<std::size_t>(tables_.hhea) + 34);
        if (!num_glyphs || !num_h_metrics) {
            return std::nullopt;
        }
        std::size_t hmtx_off = tables_.hmtx;
        std::size_t gid = glyph_id;
        if (gid >= *num_glyphs) {
            return std::nullopt;
        }
        GlyphMetrics gm;
        if (gid < *num_h_metrics) {
            std::size_t base = hmtx_off + gid * 4;
            auto aw = detail::read_u16(buf, base);
            auto lsb = detail::read_i16(buf, base + 2);
            if (!aw || !lsb) {
                return std::nullopt;
            }
            gm.advance_width = *aw;
            gm.left_side_bearing = *lsb;
        } else {
            if (*num_h_metrics == 0) {
                return std::nullopt;
            }
            auto last = detail::read_u16(
                buf, hmtx_off + static_cast<std::size_t>(*num_h_metrics - 1) * 4);
            if (!last) {
                return std::nullopt;
            }
            std::size_t lsb_off = hmtx_off +
                                  static_cast<std::size_t>(*num_h_metrics) * 4 +
                                  (gid - *num_h_metrics) * 2;
            auto lsb = detail::read_i16(buf, lsb_off);
            if (!lsb) {
                return std::nullopt;
            }
            gm.advance_width = *last;
            gm.left_side_bearing = *lsb;
        }
        return gm;
    }

    std::int16_t kerning(std::uint16_t left, std::uint16_t right) const {
        const auto& buf = data_;
        if (!tables_.kern) {
            return 0;
        }
        std::size_t kern_off = *tables_.kern;
        auto n_tables = detail::read_u16(buf, kern_off + 2);
        if (!n_tables) {
            return 0;
        }
        std::size_t pos = kern_off + 4;
        for (std::uint16_t t = 0; t < *n_tables; ++t) {
            if (pos > buf.size() || buf.size() - pos < 6) {
                break;
            }
            auto length = detail::read_u16(buf, pos + 2);
            auto coverage = detail::read_u16(buf, pos + 4);
            if (!length || !coverage) {
                break;
            }
            std::uint16_t sub_format = static_cast<std::uint16_t>(*coverage >> 8);
            if (sub_format == 0) {
                auto n_pairs = detail::read_u16(buf, pos + 6);
                if (!n_pairs) {
                    break;
                }
                std::size_t pairs_base = pos + 14;
                std::uint32_t target =
                    (static_cast<std::uint32_t>(left) << 16) | right;
                std::size_t lo = 0, hi = *n_pairs;
                while (lo < hi) {
                    std::size_t mid = (lo + hi) / 2;
                    std::size_t pair_off = pairs_base + mid * 6;
                    auto pl = detail::read_u16(buf, pair_off);
                    auto pr = detail::read_u16(buf, pair_off + 2);
                    if (!pl || !pr) {
                        break;
                    }
                    std::uint32_t key =
                        (static_cast<std::uint32_t>(*pl) << 16) | *pr;
                    if (key == target) {
                        return detail::read_i16(buf, pair_off + 4).value_or(0);
                    } else if (key < target) {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
            }
            pos += *length;
        }
        return 0;
    }

    std::size_t byte_size() const { return data_.size(); }

  private:
    struct Tables {
        std::uint32_t head = 0, hhea = 0, maxp = 0, cmap = 0, hmtx = 0;
        std::optional<std::uint32_t> kern, name, os2;
    };

    static std::optional<std::uint32_t> find_table(
        const std::vector<std::uint8_t>& buf, std::uint16_t num_tables,
        const char tag[4]) {
        for (std::uint16_t i = 0; i < num_tables; ++i) {
            std::size_t rec = 12 + static_cast<std::size_t>(i) * 16;
            if (rec > buf.size() || buf.size() - rec < 4) {
                return std::nullopt;
            }
            if (std::memcmp(buf.data() + rec, tag, 4) == 0) {
                return detail::read_u32(buf, rec + 8);
            }
        }
        return std::nullopt;
    }

    static Tables parse_table_directory(const std::vector<std::uint8_t>& buf) {
        if (buf.size() < 12) {
            throw FontError::BufferTooShort;
        }
        auto sfnt = detail::read_u32(buf, 0);
        if (!sfnt) {
            throw FontError::BufferTooShort;
        }
        if (*sfnt != 0x00010000u && *sfnt != 0x4F54544Fu) {
            throw FontError::InvalidMagic;
        }
        auto num_tables = detail::read_u16(buf, 4);
        if (!num_tables) {
            throw FontError::BufferTooShort;
        }
        auto require = [&](const char tag[4]) {
            auto o = find_table(buf, *num_tables, tag);
            if (!o) {
                throw FontError::TableNotFound;
            }
            return *o;
        };
        Tables t;
        t.head = require("head");
        t.hhea = require("hhea");
        t.maxp = require("maxp");
        t.cmap = require("cmap");
        t.hmtx = require("hmtx");
        t.kern = find_table(buf, *num_tables, "kern");
        t.name = find_table(buf, *num_tables, "name");
        t.os2 = find_table(buf, *num_tables, "OS/2");
        return t;
    }

    std::optional<std::string> read_name(std::uint16_t name_id) const {
        if (!tables_.name) {
            return std::nullopt;
        }
        const auto& buf = data_;
        std::size_t base = *tables_.name;
        auto count = detail::read_u16(buf, base + 2);
        auto string_offset = detail::read_u16(buf, base + 4);
        if (!count || !string_offset) {
            return std::nullopt;
        }
        bool found = false;
        std::size_t best_start = 0, best_len = 0;
        for (std::uint16_t i = 0; i < *count; ++i) {
            std::size_t rec = base + 6 + static_cast<std::size_t>(i) * 12;
            auto platform_id = detail::read_u16(buf, rec);
            auto encoding_id = detail::read_u16(buf, rec + 2);
            auto nid = detail::read_u16(buf, rec + 6);
            auto length = detail::read_u16(buf, rec + 8);
            auto str_off = detail::read_u16(buf, rec + 10);
            if (!platform_id || !encoding_id || !nid || !length || !str_off) {
                return std::nullopt;
            }
            if (*nid != name_id) {
                continue;
            }
            std::size_t abs_start = base + *string_offset + *str_off;
            if (*platform_id == 3 && *encoding_id == 1) {
                best_start = abs_start;
                best_len = *length;
                found = true;
                break;
            }
            if (*platform_id == 0 && !found) {
                best_start = abs_start;
                best_len = *length;
                found = true;
            }
        }
        if (!found) {
            return std::nullopt;
        }
        if (best_start > buf.size() || buf.size() - best_start < best_len) {
            return std::nullopt;
        }
        return detail::utf16be_to_utf8(buf, best_start, best_len);
    }

    std::vector<std::uint8_t> data_;
    Tables tables_;
};

}  // namespace font_parser
}  // namespace ca

#endif  // FONT_PARSER_HPP
