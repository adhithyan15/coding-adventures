//! # Display -- VGA text-mode framebuffer simulation
//!
//! This crate simulates a VGA text-mode framebuffer display, modeled after
//! the classic 80x25 text mode that dominated personal computing from the
//! 1980s through the early 2000s.
//!
//! ## What is a framebuffer?
//!
//! A framebuffer is a region of memory that directly maps to what appears on
//! screen. In VGA text mode, the framebuffer is an array of cells, where each
//! cell is 2 bytes: one byte for the ASCII character and one byte for the
//! color attribute.
//!
//! Think of it like a wall of Post-it notes: 80 columns wide and 25 rows tall.
//! Each note holds one character and has a color.
//!
//! ## Quick start
//!
//! ```
//! use display::{DisplayDriver, DisplayConfig, BYTES_PER_CELL};
//!
//! let config = DisplayConfig::default();
//! let mut memory = vec![0u8; config.columns * config.rows * BYTES_PER_CELL];
//! let mut driver = DisplayDriver::new(config, &mut memory);
//! driver.puts("Hello World");
//! let snap = driver.snapshot();
//! assert_eq!(snap.lines[0], "Hello World");
//! assert!(snap.contains("Hello"));
//! ```

// ============================================================
// Constants -- the fundamental parameters of VGA text mode
// ============================================================

/// Each cell is 2 bytes: byte 0 = character, byte 1 = attribute.
pub const BYTES_PER_CELL: usize = 2;

/// Standard VGA text mode width.
pub const DEFAULT_COLUMNS: usize = 80;

/// Standard VGA text mode height.
pub const DEFAULT_ROWS: usize = 25;

/// Memory-mapped base address. We use 0xFFFB0000 to avoid conflicts
/// with program memory. On real x86 hardware, VGA lives at 0xB8000.
pub const DEFAULT_FRAMEBUFFER_BASE: u32 = 0xFFFB_0000;

/// Light gray on black (0x07). The classic terminal appearance.
pub const DEFAULT_ATTRIBUTE: u8 = 0x07;

// ============================================================
// Color constants -- the VGA color palette
// ============================================================

pub const COLOR_BLACK: u8 = 0;
pub const COLOR_BLUE: u8 = 1;
pub const COLOR_GREEN: u8 = 2;
pub const COLOR_CYAN: u8 = 3;
pub const COLOR_RED: u8 = 4;
pub const COLOR_MAGENTA: u8 = 5;
pub const COLOR_BROWN: u8 = 6;
pub const COLOR_LIGHT_GRAY: u8 = 7;
pub const COLOR_DARK_GRAY: u8 = 8;
pub const COLOR_LIGHT_BLUE: u8 = 9;
pub const COLOR_LIGHT_GREEN: u8 = 10;
pub const COLOR_LIGHT_CYAN: u8 = 11;
pub const COLOR_LIGHT_RED: u8 = 12;
pub const COLOR_LIGHT_MAGENTA: u8 = 13;
pub const COLOR_YELLOW: u8 = 14;
pub const COLOR_WHITE: u8 = 15;

/// Combine foreground and background colors into an attribute byte.
///
/// The foreground occupies the low 4 bits, the background occupies bits 4-6.
///
/// # Examples
///
/// ```
/// use display::{make_attribute, COLOR_WHITE, COLOR_BLUE};
/// assert_eq!(make_attribute(COLOR_WHITE, COLOR_BLUE), 0x1F);
/// ```
pub fn make_attribute(fg: u8, bg: u8) -> u8 {
    ((bg & 0x07) << 4) | (fg & 0x0F)
}

// ============================================================
// Data structures
// ============================================================

/// A single character position in the framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// ASCII character code (0-255).
    pub character: u8,
    /// Color attribute byte.
    pub attribute: u8,
}

/// Tracks the row and column of the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: usize,
    pub col: usize,
}

/// Parameters for the display dimensions and memory mapping.
#[derive(Debug, Clone, Copy)]
pub struct DisplayConfig {
    pub columns: usize,
    pub rows: usize,
    pub framebuffer_base: u32,
    pub default_attribute: u8,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            columns: DEFAULT_COLUMNS,
            rows: DEFAULT_ROWS,
            framebuffer_base: DEFAULT_FRAMEBUFFER_BASE,
            default_attribute: DEFAULT_ATTRIBUTE,
        }
    }
}

impl DisplayConfig {
    /// Compact 40x10 config for testing.
    pub fn compact() -> Self {
        Self {
            columns: 40,
            rows: 10,
            ..Default::default()
        }
    }
}

// ============================================================
// DisplayDriver
// ============================================================

/// Manages the framebuffer and cursor state.
///
/// The display driver is the software layer between the OS kernel and the
/// raw framebuffer memory. It tracks the cursor, handles special characters,
/// and triggers scrolling when output exceeds the screen height.
pub struct DisplayDriver<'a> {
    pub config: DisplayConfig,
    pub memory: &'a mut [u8],
    pub cursor: CursorPosition,
}

impl<'a> DisplayDriver<'a> {
    /// Create a display driver backed by the given memory slice.
    ///
    /// The memory must be at least `columns * rows * 2` bytes long.
    /// All cells are initialized to space + default attribute.
    pub fn new(config: DisplayConfig, memory: &'a mut [u8]) -> Self {
        let mut driver = Self {
            config,
            memory,
            cursor: CursorPosition { row: 0, col: 0 },
        };
        driver.clear();
        driver
    }

    // ============================================================
    // Writing characters
    // ============================================================

    /// Write a single character at the current cursor position using
    /// the default attribute, then advance the cursor.
    ///
    /// Special characters:
    /// - `\n` (0x0A): move to column 0 of the next row
    /// - `\r` (0x0D): move to column 0 of the current row
    /// - `\t` (0x09): advance to the next multiple of 8
    /// - `\x08` (backspace): move cursor left by 1 (does not erase)
    pub fn put_char(&mut self, ch: u8) {
        match ch {
            0x0A => {
                // Newline: move to beginning of next row.
                self.cursor.col = 0;
                self.cursor.row += 1;
            }
            0x0D => {
                // Carriage return: move to column 0.
                self.cursor.col = 0;
            }
            0x09 => {
                // Tab: advance to next tab stop (every 8 columns).
                self.cursor.col = (self.cursor.col / 8 + 1) * 8;
                if self.cursor.col >= self.config.columns {
                    self.cursor.col = 0;
                    self.cursor.row += 1;
                }
            }
            0x08 => {
                // Backspace: move left by one (no erase).
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            _ => {
                // Regular character: write to framebuffer, advance cursor.
                let offset =
                    (self.cursor.row * self.config.columns + self.cursor.col) * BYTES_PER_CELL;
                if offset + 1 < self.memory.len() {
                    self.memory[offset] = ch;
                    self.memory[offset + 1] = self.config.default_attribute;
                }
                self.cursor.col += 1;

                // Line wrap.
                if self.cursor.col >= self.config.columns {
                    self.cursor.col = 0;
                    self.cursor.row += 1;
                }
            }
        }

        // Scroll check.
        if self.cursor.row >= self.config.rows {
            self.scroll();
        }
    }

    /// Write a character with a specific attribute at the given position.
    /// Does NOT move the cursor. Does NOT handle special characters.
    pub fn put_char_at(&mut self, row: usize, col: usize, ch: u8, attr: u8) {
        if row >= self.config.rows || col >= self.config.columns {
            return;
        }
        let offset = (row * self.config.columns + col) * BYTES_PER_CELL;
        self.memory[offset] = ch;
        self.memory[offset + 1] = attr;
    }

    /// Write a string to the display, one character at a time.
    pub fn puts(&mut self, s: &str) {
        for ch in s.bytes() {
            self.put_char(ch);
        }
    }

    // ============================================================
    // Screen management
    // ============================================================

    /// Reset the entire display: fill all cells with space + default attribute,
    /// reset cursor to (0, 0).
    pub fn clear(&mut self) {
        let total_bytes = self.config.columns * self.config.rows * BYTES_PER_CELL;
        let mut i = 0;
        while i < total_bytes && i + 1 < self.memory.len() {
            self.memory[i] = b' ';
            self.memory[i + 1] = self.config.default_attribute;
            i += BYTES_PER_CELL;
        }
        self.cursor.row = 0;
        self.cursor.col = 0;
    }

    /// Shift all rows up by one line. The last row is cleared.
    /// Cursor moves to (last_row, 0).
    pub fn scroll(&mut self) {
        let bytes_per_row = self.config.columns * BYTES_PER_CELL;
        let total_bytes = self.config.rows * bytes_per_row;

        // Copy rows 1..N-1 into rows 0..N-2.
        for i in 0..(total_bytes - bytes_per_row) {
            self.memory[i] = self.memory[i + bytes_per_row];
        }

        // Clear the last row.
        let last_row_start = (self.config.rows - 1) * bytes_per_row;
        let mut i = last_row_start;
        while i < total_bytes {
            self.memory[i] = b' ';
            self.memory[i + 1] = self.config.default_attribute;
            i += BYTES_PER_CELL;
        }

        self.cursor.row = self.config.rows - 1;
        self.cursor.col = 0;
    }

    // ============================================================
    // Cursor management
    // ============================================================

    /// Move the cursor to the given position, clamped to valid bounds.
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.config.rows - 1);
        self.cursor.col = col.min(self.config.columns - 1);
    }

    /// Return the current cursor position.
    pub fn get_cursor(&self) -> CursorPosition {
        self.cursor
    }

    // ============================================================
    // Reading cells
    // ============================================================

    /// Return the character and attribute at the given position.
    /// Returns Cell{' ', default_attribute} if out of bounds.
    pub fn get_cell(&self, row: usize, col: usize) -> Cell {
        if row >= self.config.rows || col >= self.config.columns {
            return Cell {
                character: b' ',
                attribute: self.config.default_attribute,
            };
        }
        let offset = (row * self.config.columns + col) * BYTES_PER_CELL;
        Cell {
            character: self.memory[offset],
            attribute: self.memory[offset + 1],
        }
    }

    // ============================================================
    // Snapshot
    // ============================================================

    /// Return a read-friendly view of the current display state.
    pub fn snapshot(&self) -> DisplaySnapshot {
        let mut lines = Vec::with_capacity(self.config.rows);
        for row in 0..self.config.rows {
            let mut line = String::with_capacity(self.config.columns);
            for col in 0..self.config.columns {
                let offset = (row * self.config.columns + col) * BYTES_PER_CELL;
                line.push(self.memory[offset] as char);
            }
            lines.push(line.trim_end().to_string());
        }
        DisplaySnapshot {
            lines,
            cursor: self.cursor,
            rows: self.config.rows,
            columns: self.config.columns,
        }
    }
}

// ============================================================
// DisplaySnapshot
// ============================================================

/// A frozen view of the display's text content.
#[derive(Debug, Clone)]
pub struct DisplaySnapshot {
    /// Text content of each row (trailing spaces trimmed).
    pub lines: Vec<String>,
    /// Cursor position at snapshot time.
    pub cursor: CursorPosition,
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub columns: usize,
}

impl DisplaySnapshot {
    /// Return the full display as a multi-line string, each line padded
    /// to the full column width.
    pub fn to_string_padded(&self) -> String {
        self.lines
            .iter()
            .map(|line| format!("{:<width$}", line, width = self.columns))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Return true if the given text appears anywhere in the display.
    pub fn contains(&self, text: &str) -> bool {
        self.lines.iter().any(|line| line.contains(text))
    }

    /// Return the text content of a specific row (trailing spaces trimmed).
    /// Returns "" if the row is out of bounds.
    pub fn line_at(&self, row: usize) -> &str {
        if row >= self.lines.len() {
            ""
        } else {
            &self.lines[row]
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_driver() -> (Vec<u8>, DisplayConfig) {
        let config = DisplayConfig::compact();
        let memory = vec![0u8; config.columns * config.rows * BYTES_PER_CELL];
        (memory, config)
    }

    fn new_standard_driver() -> (Vec<u8>, DisplayConfig) {
        let config = DisplayConfig::default();
        let memory = vec![0u8; config.columns * config.rows * BYTES_PER_CELL];
        (memory, config)
    }

    // ---- Config tests ----

    #[test]
    fn test_default_config() {
        let config = DisplayConfig::default();
        assert_eq!(config.columns, 80);
        assert_eq!(config.rows, 25);
        assert_eq!(config.framebuffer_base, 0xFFFB_0000);
        assert_eq!(config.default_attribute, 0x07);
    }

    #[test]
    fn test_make_attribute_white_on_blue() {
        assert_eq!(make_attribute(COLOR_WHITE, COLOR_BLUE), 0x1F);
    }

    #[test]
    fn test_make_attribute_default() {
        assert_eq!(make_attribute(COLOR_LIGHT_GRAY, COLOR_BLACK), 0x07);
    }

    #[test]
    fn test_make_attribute_white_on_red() {
        assert_eq!(make_attribute(COLOR_WHITE, COLOR_RED), 0x4F);
    }

    #[test]
    fn test_make_attribute_green_on_black() {
        assert_eq!(make_attribute(COLOR_GREEN, COLOR_BLACK), 0x02);
    }

    #[test]
    fn test_compact_config() {
        let config = DisplayConfig::compact();
        assert_eq!(config.columns, 40);
        assert_eq!(config.rows, 10);
    }

    // ---- Constructor tests ----

    #[test]
    fn test_clears_screen() {
        let (mut mem, config) = new_test_driver();
        let d = DisplayDriver::new(config, &mut mem);
        for row in 0..config.rows {
            for col in 0..config.columns {
                let cell = d.get_cell(row, col);
                assert_eq!(cell.character, b' ');
                assert_eq!(cell.attribute, DEFAULT_ATTRIBUTE);
            }
        }
    }

    #[test]
    fn test_cursor_at_origin() {
        let (mut mem, config) = new_test_driver();
        let d = DisplayDriver::new(config, &mut mem);
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 0);
    }

    // ---- PutChar tests ----

    #[test]
    fn test_put_char_basic() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        let cell = d.get_cell(0, 0);
        assert_eq!(cell.character, b'A');
        assert_eq!(cell.attribute, DEFAULT_ATTRIBUTE);
    }

    #[test]
    fn test_put_char_cursor_advance() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 1);
    }

    #[test]
    fn test_put_char_multiple() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'H');
        d.put_char(b'i');
        assert_eq!(d.get_cell(0, 0).character, b'H');
        assert_eq!(d.get_cell(0, 1).character, b'i');
        assert_eq!(d.get_cursor().col, 2);
    }

    #[test]
    fn test_put_char_newline() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        d.put_char(b'\n');
        let pos = d.get_cursor();
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_put_char_carriage_return() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..5 {
            d.put_char(b'x');
        }
        d.put_char(b'\r');
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_put_char_tab() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'\t');
        assert_eq!(d.get_cursor().col, 8);
    }

    #[test]
    fn test_put_char_tab_from_col_1() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'x');
        d.put_char(b'\t');
        assert_eq!(d.get_cursor().col, 8);
    }

    #[test]
    fn test_put_char_backspace() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        d.put_char(b'B');
        d.put_char(0x08);
        assert_eq!(d.get_cursor().col, 1);
    }

    #[test]
    fn test_put_char_backspace_at_col_zero() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(0x08);
        assert_eq!(d.get_cursor().col, 0);
    }

    // ---- PutCharAt tests ----

    #[test]
    fn test_put_char_at_basic() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char_at(5, 10, b'X', 0x0F);
        let cell = d.get_cell(5, 10);
        assert_eq!(cell.character, b'X');
        assert_eq!(cell.attribute, 0x0F);
    }

    #[test]
    fn test_put_char_at_does_not_move_cursor() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.set_cursor(0, 0);
        d.put_char_at(5, 10, b'X', 0x07);
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_put_char_at_out_of_bounds() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        // Should not panic.
        d.put_char_at(30, 0, b'X', 0x07);
        d.put_char_at(0, 100, b'X', 0x07);
    }

    // ---- Puts tests ----

    #[test]
    fn test_puts_simple() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello");
        for (i, ch) in "Hello".bytes().enumerate() {
            assert_eq!(d.get_cell(0, i).character, ch);
        }
        assert_eq!(d.get_cursor().col, 5);
    }

    #[test]
    fn test_puts_with_newline() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hi\nBye");
        let snap = d.snapshot();
        assert_eq!(snap.lines[0], "Hi");
        assert_eq!(snap.lines[1], "Bye");
    }

    #[test]
    fn test_puts_empty() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("");
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 0);
    }

    // ---- Line wrap tests ----

    #[test]
    fn test_line_wrap_at_end_of_row() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..config.columns {
            d.put_char(b'A');
        }
        let pos = d.get_cursor();
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_line_wrap_next_char() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..config.columns {
            d.put_char(b'A');
        }
        d.put_char(b'B');
        assert_eq!(d.get_cell(1, 0).character, b'B');
    }

    #[test]
    fn test_multi_line_wrap() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        let total = config.columns * 2 + 1;
        for _ in 0..total {
            d.put_char(b'x');
        }
        let pos = d.get_cursor();
        assert_eq!(pos.row, 2);
        assert_eq!(pos.col, 1);
    }

    // ---- Scroll tests ----

    #[test]
    fn test_scroll_trigger() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for row in 0..config.rows {
            d.put_char_at(row, 0, b'A' + row as u8, DEFAULT_ATTRIBUTE);
        }
        let row1_char = d.get_cell(1, 0).character;
        d.set_cursor(config.rows - 1, 0);
        d.put_char(b'\n');
        assert_eq!(d.get_cell(0, 0).character, row1_char);
    }

    #[test]
    fn test_scroll_last_row_cleared() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for row in 0..config.rows {
            for col in 0..config.columns {
                d.put_char_at(row, col, b'X', DEFAULT_ATTRIBUTE);
            }
        }
        d.set_cursor(config.rows - 1, 0);
        d.put_char(b'\n');
        for col in 0..config.columns {
            assert_eq!(d.get_cell(config.rows - 1, col).character, b' ');
        }
    }

    #[test]
    fn test_scroll_cursor_position() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.set_cursor(config.rows - 1, 0);
        d.put_char(b'\n');
        let pos = d.get_cursor();
        assert_eq!(pos.row, config.rows - 1);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_multiple_scrolls() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..30 {
            d.puts("Line");
            d.put_char(b'\n');
        }
        let snap = d.snapshot();
        assert!(snap.contains("Line"));
    }

    #[test]
    fn test_scroll_preserves_attributes() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        let custom_attr = make_attribute(COLOR_WHITE, COLOR_BLUE);
        d.put_char_at(1, 0, b'Z', custom_attr);
        d.set_cursor(config.rows - 1, 0);
        d.put_char(b'\n');
        let cell = d.get_cell(0, 0);
        assert_eq!(cell.character, b'Z');
        assert_eq!(cell.attribute, custom_attr);
    }

    // ---- Clear tests ----

    #[test]
    fn test_clear_display() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello World");
        d.clear();
        for row in 0..config.rows {
            for col in 0..config.columns {
                let cell = d.get_cell(row, col);
                assert_eq!(cell.character, b' ');
                assert_eq!(cell.attribute, DEFAULT_ATTRIBUTE);
            }
        }
    }

    #[test]
    fn test_clear_resets_cursor() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello");
        d.clear();
        let pos = d.get_cursor();
        assert_eq!(pos.row, 0);
        assert_eq!(pos.col, 0);
    }

    // ---- Snapshot tests ----

    #[test]
    fn test_snapshot_basic() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello World");
        let snap = d.snapshot();
        assert_eq!(snap.lines[0], "Hello World");
    }

    #[test]
    fn test_snapshot_trailing_spaces_trimmed() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hi");
        let snap = d.snapshot();
        assert_eq!(snap.lines[0], "Hi");
    }

    #[test]
    fn test_snapshot_empty_lines() {
        let (mut mem, config) = new_test_driver();
        let d = DisplayDriver::new(config, &mut mem);
        let snap = d.snapshot();
        for line in &snap.lines {
            assert_eq!(line, "");
        }
    }

    #[test]
    fn test_snapshot_contains_positive() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello World");
        let snap = d.snapshot();
        assert!(snap.contains("Hello World"));
    }

    #[test]
    fn test_snapshot_contains_negative() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello World");
        let snap = d.snapshot();
        assert!(!snap.contains("Goodbye"));
    }

    #[test]
    fn test_snapshot_contains_partial() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello World");
        let snap = d.snapshot();
        assert!(snap.contains("World"));
    }

    #[test]
    fn test_snapshot_string_output() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Hello");
        let snap = d.snapshot();
        let s = snap.to_string_padded();
        let lines: Vec<&str> = s.split('\n').collect();
        assert_eq!(lines.len(), config.rows);
        for line in &lines {
            assert_eq!(line.len(), config.columns);
        }
    }

    #[test]
    fn test_snapshot_cursor() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.set_cursor(5, 10);
        let snap = d.snapshot();
        assert_eq!(snap.cursor.row, 5);
        assert_eq!(snap.cursor.col, 10);
    }

    #[test]
    fn test_snapshot_line_at() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.puts("Line 0");
        d.put_char(b'\n');
        d.puts("Line 1");
        let snap = d.snapshot();
        assert_eq!(snap.line_at(0), "Line 0");
        assert_eq!(snap.line_at(1), "Line 1");
        assert_eq!(snap.line_at(100), "");
    }

    #[test]
    fn test_snapshot_rows_and_columns() {
        let (mut mem, config) = new_test_driver();
        let d = DisplayDriver::new(config, &mut mem);
        let snap = d.snapshot();
        assert_eq!(snap.rows, config.rows);
        assert_eq!(snap.columns, config.columns);
    }

    // ---- Attribute tests ----

    #[test]
    fn test_default_attribute() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        assert_eq!(d.get_cell(0, 0).attribute, 0x07);
    }

    #[test]
    fn test_custom_attribute() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char_at(0, 0, b'A', 0x1F);
        assert_eq!(d.get_cell(0, 0).attribute, 0x1F);
    }

    // ---- Cursor management tests ----

    #[test]
    fn test_set_cursor_clamps_large() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.set_cursor(100, 100);
        let pos = d.get_cursor();
        assert_eq!(pos.row, config.rows - 1);
        assert_eq!(pos.col, config.columns - 1);
    }

    // ---- Edge case tests ----

    #[test]
    fn test_full_framebuffer() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        let total = config.columns * config.rows;
        for _ in 0..total {
            d.put_char(b'X');
        }
        let snap = d.snapshot();
        assert!(snap.contains("X"));
    }

    #[test]
    fn test_rapid_scrolling() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..100 {
            d.puts("Line");
            d.put_char(b'\n');
        }
        let snap = d.snapshot();
        assert!(snap.contains("Line"));
    }

    #[test]
    fn test_null_character() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(0x00);
        assert_eq!(d.get_cell(0, 0).character, 0x00);
    }

    #[test]
    fn test_all_ascii_values() {
        let (mut mem, config) = new_standard_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for i in 0u16..256 {
            let row = i as usize / config.columns;
            let col = i as usize % config.columns;
            d.put_char_at(row, col, i as u8, DEFAULT_ATTRIBUTE);
        }
        for i in 0u16..256 {
            let row = i as usize / config.columns;
            let col = i as usize % config.columns;
            assert_eq!(d.get_cell(row, col).character, i as u8);
        }
    }

    #[test]
    fn test_get_cell_out_of_bounds() {
        let (mut mem, config) = new_test_driver();
        let d = DisplayDriver::new(config, &mut mem);
        let cell = d.get_cell(100, 0);
        assert_eq!(cell.character, b' ');
        assert_eq!(cell.attribute, DEFAULT_ATTRIBUTE);
    }

    #[test]
    fn test_tab_wrap_to_next_row() {
        let (mut mem, config) = new_test_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.set_cursor(0, 39);
        d.put_char(b'\t');
        let pos = d.get_cursor();
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 0);
    }

    // ---- Standard 80x25 tests ----

    #[test]
    fn test_standard_put_char() {
        let (mut mem, config) = new_standard_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        d.put_char(b'A');
        let cell = d.get_cell(0, 0);
        assert_eq!(cell.character, b'A');
        assert_eq!(cell.attribute, 0x07);
    }

    #[test]
    fn test_standard_line_wrap() {
        let (mut mem, config) = new_standard_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for _ in 0..81 {
            d.put_char(b'A');
        }
        let pos = d.get_cursor();
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 1);
    }

    #[test]
    fn test_standard_scroll() {
        let (mut mem, config) = new_standard_driver();
        let mut d = DisplayDriver::new(config, &mut mem);
        for row in 0..25 {
            d.put_char_at(row, 0, b'A' + (row as u8 % 26), DEFAULT_ATTRIBUTE);
        }
        d.set_cursor(24, 0);
        d.put_char(b'\n');
        assert_eq!(d.get_cell(0, 0).character, b'B');
    }
}
