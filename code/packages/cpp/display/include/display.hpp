// display.hpp — VGA text-mode framebuffer simulation, header-only in pure ISO
// C++17 (namespace ca::display). A faithful port of the Rust `display` crate.
// ===========================================================================
//
// Simulates the classic 80x25 VGA text-mode framebuffer: a grid of cells, each
// 2 bytes (character + colour attribute). The driver tracks a cursor,
// interprets \n \r \t and backspace, wraps at the right edge, and scrolls when
// output runs past the bottom.
//
// MEMORY. The framebuffer is caller-owned (a `std::vector<std::uint8_t>&`,
// mirroring the Rust borrowed `&mut [u8]`); the driver only views it. Supply at
// least `columns * rows * BYTES_PER_CELL` bytes. `snapshot()` returns an owned
// text view.
//
// DIVERGENCE FROM RUST. Snapshot lines are `std::vector<std::string>` (Rust
// `Vec<String>`); out-of-range framebuffer access is a defensive no-op rather
// than a bounds-check panic.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no <cmath>, no compiler extensions.
#ifndef CA_DISPLAY_HPP
#define CA_DISPLAY_HPP

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace ca {
namespace display {

// ── Constants ────────────────────────────────────────────────────────────────
inline constexpr std::size_t BYTES_PER_CELL = 2;
inline constexpr std::size_t DEFAULT_COLUMNS = 80;
inline constexpr std::size_t DEFAULT_ROWS = 25;
inline constexpr std::uint32_t DEFAULT_FRAMEBUFFER_BASE = 0xFFFB0000u;
inline constexpr std::uint8_t DEFAULT_ATTRIBUTE = 0x07;  // light gray on black

// VGA 16-colour palette.
inline constexpr std::uint8_t COLOR_BLACK = 0;
inline constexpr std::uint8_t COLOR_BLUE = 1;
inline constexpr std::uint8_t COLOR_GREEN = 2;
inline constexpr std::uint8_t COLOR_CYAN = 3;
inline constexpr std::uint8_t COLOR_RED = 4;
inline constexpr std::uint8_t COLOR_MAGENTA = 5;
inline constexpr std::uint8_t COLOR_BROWN = 6;
inline constexpr std::uint8_t COLOR_LIGHT_GRAY = 7;
inline constexpr std::uint8_t COLOR_DARK_GRAY = 8;
inline constexpr std::uint8_t COLOR_LIGHT_BLUE = 9;
inline constexpr std::uint8_t COLOR_LIGHT_GREEN = 10;
inline constexpr std::uint8_t COLOR_LIGHT_CYAN = 11;
inline constexpr std::uint8_t COLOR_LIGHT_RED = 12;
inline constexpr std::uint8_t COLOR_LIGHT_MAGENTA = 13;
inline constexpr std::uint8_t COLOR_YELLOW = 14;
inline constexpr std::uint8_t COLOR_WHITE = 15;

// Combine fg (low nibble) and bg (bits 4-6) into an attribute byte.
inline std::uint8_t make_attribute(std::uint8_t fg, std::uint8_t bg) {
    return static_cast<std::uint8_t>(((bg & 0x07) << 4) | (fg & 0x0F));
}

// ── Data structures ──────────────────────────────────────────────────────────
struct Cell {
    std::uint8_t character;
    std::uint8_t attribute;
    bool operator==(const Cell& o) const {
        return character == o.character && attribute == o.attribute;
    }
};

struct CursorPosition {
    std::size_t row;
    std::size_t col;
};

struct DisplayConfig {
    std::size_t columns;
    std::size_t rows;
    std::uint32_t framebuffer_base;
    std::uint8_t default_attribute;

    static DisplayConfig default_config() {
        return {DEFAULT_COLUMNS, DEFAULT_ROWS, DEFAULT_FRAMEBUFFER_BASE,
                DEFAULT_ATTRIBUTE};
    }
    static DisplayConfig compact() {  // 40x10, for tests
        DisplayConfig c = default_config();
        c.columns = 40;
        c.rows = 10;
        return c;
    }
};

// A frozen text view of the display.
struct DisplaySnapshot {
    std::vector<std::string> lines;  // trailing whitespace trimmed
    CursorPosition cursor;
    std::size_t rows;
    std::size_t columns;

    // Every row padded to the full column width, joined by '\n'.
    std::string to_string_padded() const {
        std::string out;
        for (std::size_t r = 0; r < lines.size(); r++) {
            if (r > 0) out.push_back('\n');
            const std::string& line = lines[r];
            out += line;
            for (std::size_t c = line.size(); c < columns; c++) out.push_back(' ');
        }
        return out;
    }

    bool contains(const std::string& text) const {
        for (const auto& line : lines)
            if (line.find(text) != std::string::npos) return true;
        return false;
    }

    const std::string& line_at(std::size_t row) const {
        static const std::string empty;
        if (row >= lines.size()) return empty;
        return lines[row];
    }
};

// Manages the framebuffer and cursor. Borrows the caller's memory buffer.
class DisplayDriver {
public:
    DisplayConfig config;
    CursorPosition cursor;

    // Create a driver over `memory` and clear the screen (like Rust `new`).
    DisplayDriver(DisplayConfig cfg, std::vector<std::uint8_t>& memory)
        : config(cfg), cursor{0, 0}, mem_(memory.data()), mem_len_(memory.size()) {
        clear();
    }

    // Wrap an existing framebuffer WITHOUT clearing it (like Rust `wrap`).
    static DisplayDriver wrap(DisplayConfig cfg, std::vector<std::uint8_t>& memory) {
        return DisplayDriver(cfg, memory.data(), memory.size(), NoClear{});
    }

    void put_char(std::uint8_t ch) {
        switch (ch) {
            case 0x0A:  // newline
                cursor.col = 0;
                cursor.row += 1;
                break;
            case 0x0D:  // carriage return
                cursor.col = 0;
                break;
            case 0x09:  // tab
                cursor.col = (cursor.col / 8 + 1) * 8;
                if (cursor.col >= config.columns) {
                    cursor.col = 0;
                    cursor.row += 1;
                }
                break;
            case 0x08:  // backspace
                if (cursor.col > 0) cursor.col -= 1;
                break;
            default: {
                std::size_t off = offset(cursor.row, cursor.col);
                if (off + 1 < mem_len_) {
                    mem_[off] = ch;
                    mem_[off + 1] = config.default_attribute;
                }
                cursor.col += 1;
                if (cursor.col >= config.columns) {
                    cursor.col = 0;
                    cursor.row += 1;
                }
                break;
            }
        }
        if (cursor.row >= config.rows) scroll();
    }

    void put_char_at(std::size_t row, std::size_t col, std::uint8_t ch,
                     std::uint8_t attr) {
        if (row >= config.rows || col >= config.columns) return;
        std::size_t off = offset(row, col);
        if (off + 1 >= mem_len_) return;  // defensive
        mem_[off] = ch;
        mem_[off + 1] = attr;
    }

    void puts(const std::string& s) {
        for (char ch : s) put_char(static_cast<std::uint8_t>(ch));
    }

    void clear() {
        std::size_t total = config.columns * config.rows * BYTES_PER_CELL;
        std::size_t i = 0;
        while (i < total && i + 1 < mem_len_) {
            mem_[i] = static_cast<std::uint8_t>(' ');
            mem_[i + 1] = config.default_attribute;
            i += BYTES_PER_CELL;
        }
        cursor = {0, 0};
    }

    void scroll() {
        std::size_t bytes_per_row = config.columns * BYTES_PER_CELL;
        std::size_t total = config.rows * bytes_per_row;
        std::size_t shift_end = total - bytes_per_row;  // rows >= 1
        for (std::size_t i = 0; i < shift_end; i++) {
            if (i + bytes_per_row >= mem_len_) break;  // defensive
            mem_[i] = mem_[i + bytes_per_row];
        }
        std::size_t last_row_start = (config.rows - 1) * bytes_per_row;
        for (std::size_t i = last_row_start; i < total && i + 1 < mem_len_;
             i += BYTES_PER_CELL) {
            mem_[i] = static_cast<std::uint8_t>(' ');
            mem_[i + 1] = config.default_attribute;
        }
        cursor = {config.rows - 1, 0};
    }

    void set_cursor(std::size_t row, std::size_t col) {
        std::size_t max_row = config.rows - 1;
        std::size_t max_col = config.columns - 1;
        cursor.row = row < max_row ? row : max_row;
        cursor.col = col < max_col ? col : max_col;
    }

    CursorPosition get_cursor() const { return cursor; }

    Cell get_cell(std::size_t row, std::size_t col) const {
        if (row >= config.rows || col >= config.columns)
            return Cell{static_cast<std::uint8_t>(' '), config.default_attribute};
        std::size_t off = offset(row, col);
        if (off + 1 >= mem_len_)
            return Cell{static_cast<std::uint8_t>(' '), config.default_attribute};
        return Cell{mem_[off], mem_[off + 1]};
    }

    DisplaySnapshot snapshot() const {
        DisplaySnapshot s;
        s.rows = config.rows;
        s.columns = config.columns;
        s.cursor = cursor;
        s.lines.reserve(config.rows);
        for (std::size_t row = 0; row < config.rows; row++) {
            std::string line;
            line.reserve(config.columns);
            for (std::size_t col = 0; col < config.columns; col++)
                line.push_back(static_cast<char>(get_cell(row, col).character));
            while (!line.empty() && is_ascii_ws(static_cast<std::uint8_t>(line.back())))
                line.pop_back();
            s.lines.push_back(std::move(line));
        }
        return s;
    }

private:
    std::uint8_t* mem_;
    std::size_t mem_len_;

    struct NoClear {};
    DisplayDriver(DisplayConfig cfg, std::uint8_t* mem, std::size_t len, NoClear)
        : config(cfg), cursor{0, 0}, mem_(mem), mem_len_(len) {}

    std::size_t offset(std::size_t row, std::size_t col) const {
        return (row * config.columns + col) * BYTES_PER_CELL;
    }

    static bool is_ascii_ws(std::uint8_t b) {
        return b == ' ' || b == '\t' || b == '\n' || b == '\r' || b == '\v' ||
               b == '\f';
    }
};

}  // namespace display
}  // namespace ca

#endif  // CA_DISPLAY_HPP
