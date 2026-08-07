// Tests for the C++ display library, using the header-only iso_test.h harness
// (pure ISO). Expectations mirror the Rust crate's own unit tests one-for-one.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "display.hpp"

namespace dp = ca::display;
using dp::DisplayConfig;
using dp::DisplayDriver;

// Allocate a framebuffer sized for `cfg`.
static std::vector<std::uint8_t> make_mem(const DisplayConfig& cfg) {
    return std::vector<std::uint8_t>(cfg.columns * cfg.rows * dp::BYTES_PER_CELL, 0);
}

int main() {
    // ── config & make_attribute ────────────────────────────────────────────
    {
        auto def = DisplayConfig::default_config();
        ISO_CHECK(def.columns == 80 && def.rows == 25);
        ISO_CHECK(def.framebuffer_base == 0xFFFB0000u);
        ISO_CHECK(def.default_attribute == 0x07);
        auto comp = DisplayConfig::compact();
        ISO_CHECK(comp.columns == 40 && comp.rows == 10);
        ISO_CHECK(dp::make_attribute(dp::COLOR_WHITE, dp::COLOR_BLUE) == 0x1F);
        ISO_CHECK(dp::make_attribute(dp::COLOR_LIGHT_GRAY, dp::COLOR_BLACK) == 0x07);
        ISO_CHECK(dp::make_attribute(dp::COLOR_WHITE, dp::COLOR_RED) == 0x4F);
        ISO_CHECK(dp::make_attribute(dp::COLOR_GREEN, dp::COLOR_BLACK) == 0x02);
    }

    // ── constructor clears + cursor origin ──────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        for (std::size_t r = 0; r < 10; r++)
            for (std::size_t c = 0; c < 40; c++)
                ISO_CHECK((d.get_cell(r, c) ==
                           dp::Cell{static_cast<std::uint8_t>(' '), dp::DEFAULT_ATTRIBUTE}));
        ISO_CHECK(d.get_cursor().row == 0 && d.get_cursor().col == 0);
    }

    // ── put_char basics ─────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        d.put_char('A');
        ISO_CHECK(d.get_cell(0, 0).character == 'A');
        ISO_CHECK(d.get_cell(0, 0).attribute == dp::DEFAULT_ATTRIBUTE);
        ISO_CHECK(d.get_cursor().col == 1);
        d.put_char('i');
        ISO_CHECK(d.get_cell(0, 1).character == 'i' && d.get_cursor().col == 2);
    }

    // ── control characters ──────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.put_char('A');
            d.put_char('\n');
            ISO_CHECK(d.get_cursor().row == 1 && d.get_cursor().col == 0);
            for (int i = 0; i < 5; i++) d.put_char('x');
            d.put_char('\r');
            ISO_CHECK(d.get_cursor().col == 0 && d.get_cursor().row == 1);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.put_char('\t');
            ISO_CHECK(d.get_cursor().col == 8);
            d.put_char('x');
            d.put_char('\t');
            ISO_CHECK(d.get_cursor().col == 16);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.put_char('A');
            d.put_char('B');
            d.put_char(0x08);
            ISO_CHECK(d.get_cursor().col == 1);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.put_char(0x08);
            ISO_CHECK(d.get_cursor().col == 0);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.set_cursor(0, 39);
            d.put_char('\t');
            ISO_CHECK(d.get_cursor().row == 1 && d.get_cursor().col == 0);
        }
    }

    // ── put_char_at ─────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        d.put_char_at(5, 10, 'X', 0x0F);
        ISO_CHECK(d.get_cell(5, 10).character == 'X' && d.get_cell(5, 10).attribute == 0x0F);
        d.set_cursor(0, 0);
        d.put_char_at(5, 10, 'Y', 0x07);
        ISO_CHECK(d.get_cursor().row == 0 && d.get_cursor().col == 0);
        d.put_char_at(30, 0, 'X', 0x07);  // out of bounds: no-op
        d.put_char_at(0, 100, 'X', 0x07);
    }

    // ── puts ────────────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        d.puts("Hello");
        std::string h = "Hello";
        for (std::size_t i = 0; i < 5; i++)
            ISO_CHECK(d.get_cell(0, i).character == static_cast<std::uint8_t>(h[i]));
        ISO_CHECK(d.get_cursor().col == 5);
        d.puts("");
        ISO_CHECK(d.get_cursor().col == 5);
    }

    // ── line wrap ───────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (int i = 0; i < 40; i++) d.put_char('A');
            ISO_CHECK(d.get_cursor().row == 1 && d.get_cursor().col == 0);
            d.put_char('B');
            ISO_CHECK(d.get_cell(1, 0).character == 'B');
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (int i = 0; i < 40 * 2 + 1; i++) d.put_char('x');
            ISO_CHECK(d.get_cursor().row == 2 && d.get_cursor().col == 1);
        }
    }

    // ── scroll ──────────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (std::size_t r = 0; r < 10; r++)
                d.put_char_at(r, 0, static_cast<std::uint8_t>('A' + r), dp::DEFAULT_ATTRIBUTE);
            auto row1 = d.get_cell(1, 0).character;
            d.set_cursor(9, 0);
            d.put_char('\n');
            ISO_CHECK(d.get_cell(0, 0).character == row1);
            for (std::size_t c = 0; c < 40; c++)
                ISO_CHECK(d.get_cell(9, c).character == ' ');
            ISO_CHECK(d.get_cursor().row == 9 && d.get_cursor().col == 0);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            auto attr = dp::make_attribute(dp::COLOR_WHITE, dp::COLOR_BLUE);
            d.put_char_at(1, 0, 'Z', attr);
            d.set_cursor(9, 0);
            d.put_char('\n');
            ISO_CHECK(d.get_cell(0, 0).character == 'Z' && d.get_cell(0, 0).attribute == attr);
        }
    }

    // ── clear ───────────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        d.puts("Hello World");
        d.clear();
        for (std::size_t r = 0; r < 10; r++)
            for (std::size_t c = 0; c < 40; c++)
                ISO_CHECK(d.get_cell(r, c).character == ' ');
        ISO_CHECK(d.get_cursor().row == 0 && d.get_cursor().col == 0);
    }

    // ── snapshot ────────────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.puts("Hello World");
            auto s = d.snapshot();
            ISO_CHECK(s.lines[0] == "Hello World");
            ISO_CHECK(s.contains("Hello World") && s.contains("World"));
            ISO_CHECK(!s.contains("Goodbye"));
            ISO_CHECK(s.rows == 10 && s.columns == 40);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.puts("Hi");
            auto s = d.snapshot();
            ISO_CHECK(s.lines[0] == "Hi");
            for (std::size_t i = 1; i < s.rows; i++) ISO_CHECK(s.lines[i] == "");
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.puts("Line 0");
            d.put_char('\n');
            d.puts("Line 1");
            auto s = d.snapshot();
            ISO_CHECK(s.line_at(0) == "Line 0");
            ISO_CHECK(s.line_at(1) == "Line 1");
            ISO_CHECK(s.line_at(100) == "");
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.puts("Hello");
            auto s = d.snapshot();
            auto padded = s.to_string_padded();
            std::size_t line_count = 1;
            bool all_full = true;
            std::size_t this_len = 0;
            for (char ch : padded) {
                if (ch == '\n') {
                    if (this_len != 40) all_full = false;
                    line_count++;
                    this_len = 0;
                } else {
                    this_len++;
                }
            }
            if (this_len != 40) all_full = false;
            ISO_CHECK(line_count == 10 && all_full);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.set_cursor(5, 10);
            auto s = d.snapshot();
            ISO_CHECK(s.cursor.row == 5 && s.cursor.col == 10);
        }
    }

    // ── cursor clamp & edge cases ───────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        d.set_cursor(100, 100);
        ISO_CHECK(d.get_cursor().row == 9 && d.get_cursor().col == 39);
        auto oob = d.get_cell(100, 0);
        ISO_CHECK(oob.character == ' ' && oob.attribute == dp::DEFAULT_ATTRIBUTE);
        d.clear();
        d.put_char(0x00);
        ISO_CHECK(d.get_cell(0, 0).character == 0x00);
    }

    // ── standard 80x25 ──────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::default_config();
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            d.put_char('A');
            ISO_CHECK(d.get_cell(0, 0).character == 'A' && d.get_cell(0, 0).attribute == 0x07);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (int i = 0; i < 81; i++) d.put_char('A');
            ISO_CHECK(d.get_cursor().row == 1 && d.get_cursor().col == 1);
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (std::size_t r = 0; r < 25; r++)
                d.put_char_at(r, 0, static_cast<std::uint8_t>('A' + (r % 26)), dp::DEFAULT_ATTRIBUTE);
            d.set_cursor(24, 0);
            d.put_char('\n');
            ISO_CHECK(d.get_cell(0, 0).character == 'B');
        }
        {
            auto mem = make_mem(cfg);
            DisplayDriver d(cfg, mem);
            for (int i = 0; i < 256; i++)
                d.put_char_at(static_cast<std::size_t>(i) / 80, static_cast<std::size_t>(i) % 80,
                              static_cast<std::uint8_t>(i), dp::DEFAULT_ATTRIBUTE);
            for (int i = 0; i < 256; i++)
                ISO_CHECK(d.get_cell(static_cast<std::size_t>(i) / 80,
                                     static_cast<std::size_t>(i) % 80)
                              .character == static_cast<std::uint8_t>(i));
        }
    }

    // ── rapid scrolling ─────────────────────────────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        DisplayDriver d(cfg, mem);
        for (int i = 0; i < 100; i++) {
            d.puts("Line");
            d.put_char('\n');
        }
        ISO_CHECK(d.snapshot().contains("Line"));
    }

    // ── wrap() does not clear existing content ──────────────────────────────
    {
        auto cfg = DisplayConfig::compact();
        auto mem = make_mem(cfg);
        {
            DisplayDriver d(cfg, mem);  // clears
            d.put_char_at(2, 3, 'Q', 0x1F);
        }
        auto w = DisplayDriver::wrap(cfg, mem);  // preserves
        ISO_CHECK(w.get_cell(2, 3).character == 'Q' && w.get_cell(2, 3).attribute == 0x1F);
    }

    return ISO_TEST_RESULT();
}
