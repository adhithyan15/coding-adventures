-- coding_adventures.display
-- ============================================================================
--
-- TEXT-MODE DISPLAY DRIVER
--
-- This module simulates a classic text-mode display like those used in early
-- personal computers (think: IBM PC text mode, VGA 80x25). It manages writing
-- characters into a flat "framebuffer" memory array and tracks cursor position.
--
-- MEMORY LAYOUT
-- -------------
-- The framebuffer is a flat array of bytes. Each character cell occupies
-- exactly 2 bytes:
--
--   Byte 0: character code (ASCII / code page 437 byte value)
--   Byte 1: attribute byte
--
-- The attribute byte encodes foreground and background colors:
--   Bits 0-2: foreground color (3 bits = 8 colors)
--   Bit  3:   foreground intensity (bright/dim)
--   Bits 4-6: background color (3 bits = 8 colors)
--   Bit  7:   blinking (or high-intensity background)
--
-- Default attribute 0x07 = white text on black background:
--   0000 0111 binary:  background=000 (black), foreground=111 (white)
--
-- For an 80×25 display, the memory array has 80 × 25 × 2 = 4000 entries.
-- Cell (row, col) starts at index: (row * columns + col) * 2 + 1  (1-based Lua)
--
-- CURSOR MANAGEMENT
-- -----------------
-- The cursor tracks where the next character will be written.
-- cursor_row and cursor_col are both 0-based.
--
-- Special character codes handled:
--   0x08  Backspace: move cursor left one column (if not at column 0)
--   0x09  Tab:       advance to the next tab stop (multiple of 8)
--   0x0A  Newline:   move to column 0 of the next row; scroll if at last row
--   0x0D  Carriage return: move cursor to column 0 of current row
--   Other bytes: write the character at cursor position, advance cursor
--
-- SCROLLING
-- ---------
-- When the cursor moves past the last row (row >= rows), the display scrolls:
--   - Each row's content is copied to the row above it
--   - The last row is filled with spaces (attribute 0x07)
--   - cursor_row stays at the last row
--
-- SNAPSHOT
-- --------
-- snapshot() returns a table with a `lines` array. Each entry is the text
-- content of one display row, with trailing spaces stripped.
--
-- Usage:
--   local Display = require("coding_adventures.display")
--   local config = Display.DisplayConfig.new({columns=80, rows=25})
--   local mem = {}
--   for i=1, config.columns*config.rows*2 do mem[i]=0 end
--   local drv = Display.DisplayDriver.new(config, mem)
--   drv:puts_str("Hello!")
--   local snap = drv:snapshot()
--   print(snap.lines[1])   -- "Hello!"
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- Default text attribute: white foreground (7), black background (0).
-- In VGA text mode, 0x07 is the canonical "normal" attribute.
local DEFAULT_ATTR = 0x07

-- Special character codes
local CHAR_BACKSPACE        = 0x08
local CHAR_TAB              = 0x09
local CHAR_NEWLINE          = 0x0A
local CHAR_CARRIAGE_RETURN  = 0x0D

-- Tab stop width: advance to the next column that is a multiple of 8.
local TAB_WIDTH = 8

-- ---------------------------------------------------------------------------
-- DisplayConfig
-- ---------------------------------------------------------------------------

M.DisplayConfig = {}
M.DisplayConfig.__index = M.DisplayConfig

-- DisplayConfig.new(opts)
--   Create a display configuration.
--
--   opts.columns (integer, default 80) — number of text columns per row
--   opts.rows    (integer, default 25) — number of text rows
--
--   Returns: a DisplayConfig object
function M.DisplayConfig.new(opts)
    opts = opts or {}
    local self = setmetatable({}, M.DisplayConfig)
    self.columns = opts.columns or 80
    self.rows    = opts.rows    or 25
    assert(self.columns >= 1, "DisplayConfig: columns must be >= 1")
    assert(self.rows    >= 1, "DisplayConfig: rows must be >= 1")
    return self
end

-- ---------------------------------------------------------------------------
-- DisplayDriver
-- ---------------------------------------------------------------------------

M.DisplayDriver = {}
M.DisplayDriver.__index = M.DisplayDriver

-- DisplayDriver.new(config, memory)
--   Create a new display driver.
--
--   Parameters:
--     config (DisplayConfig) — display dimensions and settings
--     memory (table)         — flat array of bytes (length = columns*rows*2)
--                              Caller must initialize to all zeros before use.
--
--   The driver starts with the cursor at row=0, col=0.
function M.DisplayDriver.new(config, memory)
    local self = setmetatable({}, M.DisplayDriver)
    self.config     = config
    self.memory     = memory
    self.cursor_row = 0
    self.cursor_col = 0
    return self
end

-- ---------------------------------------------------------------------------
-- Internal helpers
-- ---------------------------------------------------------------------------

-- cell_offset(driver, row, col)
--   Return the 1-based Lua index into memory where cell (row, col) begins.
--   Each cell is 2 bytes: [char_byte, attr_byte].
local function cell_offset(driver, row, col)
    return (row * driver.config.columns + col) * 2 + 1
end

-- write_cell(driver, row, col, char_byte, attr_byte)
--   Write a character and its attribute into the framebuffer at (row, col).
local function write_cell(driver, row, col, char_byte, attr_byte)
    local off = cell_offset(driver, row, col)
    driver.memory[off]     = char_byte
    driver.memory[off + 1] = attr_byte
end

-- scroll_up(driver)
--   Scroll the display up by one row:
--   - Copy rows 1..rows-1 into rows 0..rows-2
--   - Clear the last row (fill with space + default attribute)
local function scroll_up(driver)
    local cols = driver.config.columns
    local rows = driver.config.rows
    -- Copy each row upward
    for row = 1, rows - 1 do
        for col = 0, cols - 1 do
            local src = cell_offset(driver, row, col)
            local dst = cell_offset(driver, row - 1, col)
            driver.memory[dst]     = driver.memory[src]
            driver.memory[dst + 1] = driver.memory[src + 1]
        end
    end
    -- Clear the last row
    for col = 0, cols - 1 do
        write_cell(driver, rows - 1, col, string.byte(" "), DEFAULT_ATTR)
    end
end

-- advance_cursor(driver)
--   Move the cursor to the next column. If at the end of a row, wrap to the
--   next row. If past the last row, scroll the display up.
local function advance_cursor(driver)
    driver.cursor_col = driver.cursor_col + 1
    if driver.cursor_col >= driver.config.columns then
        driver.cursor_col = 0
        driver.cursor_row = driver.cursor_row + 1
        if driver.cursor_row >= driver.config.rows then
            scroll_up(driver)
            driver.cursor_row = driver.config.rows - 1
        end
    end
end

-- ---------------------------------------------------------------------------
-- Public DisplayDriver methods
-- ---------------------------------------------------------------------------

-- driver:put_char(byte)
--   Write a single byte to the display at the current cursor position,
--   handling control characters as described in the module header.
function M.DisplayDriver:put_char(byte)
    if byte == CHAR_NEWLINE then
        -- Move to beginning of next row
        self.cursor_col = 0
        self.cursor_row = self.cursor_row + 1
        if self.cursor_row >= self.config.rows then
            scroll_up(self)
            self.cursor_row = self.config.rows - 1
        end

    elseif byte == CHAR_CARRIAGE_RETURN then
        -- Return to column 0, stay on current row
        self.cursor_col = 0

    elseif byte == CHAR_TAB then
        -- Advance to next tab stop (next multiple of TAB_WIDTH)
        -- e.g. if cursor is at col 5, next tab stop is col 8
        local next_stop = math.floor(self.cursor_col / TAB_WIDTH) * TAB_WIDTH + TAB_WIDTH
        -- Fill with spaces up to the tab stop
        while self.cursor_col < next_stop do
            write_cell(self, self.cursor_row, self.cursor_col, string.byte(" "), DEFAULT_ATTR)
            advance_cursor(self)
        end

    elseif byte == CHAR_BACKSPACE then
        -- Move cursor left one column (but never before column 0)
        if self.cursor_col > 0 then
            self.cursor_col = self.cursor_col - 1
        end

    else
        -- Normal printable character: write it and advance the cursor
        write_cell(self, self.cursor_row, self.cursor_col, byte, DEFAULT_ATTR)
        advance_cursor(self)
    end
end

-- driver:puts_str(str)
--   Write every byte of the string `str` to the display in sequence.
function M.DisplayDriver:puts_str(str)
    for i = 1, #str do
        self:put_char(string.byte(str, i))
    end
end

-- driver:clear()
--   Fill every cell with a space character and the default attribute.
--   Reset the cursor to (0, 0).
function M.DisplayDriver:clear()
    local cols = self.config.columns
    local rows = self.config.rows
    for row = 0, rows - 1 do
        for col = 0, cols - 1 do
            write_cell(self, row, col, string.byte(" "), DEFAULT_ATTR)
        end
    end
    self.cursor_row = 0
    self.cursor_col = 0
end

-- driver:snapshot()
--   Return a read-only snapshot of the current screen contents.
--
--   Returns a table: { lines = {line1, line2, ...} }
--   where each line is the text of one row with trailing spaces stripped.
--
--   This is useful for testing and for rendering the display to a terminal
--   or file.
function M.DisplayDriver:snapshot()
    local lines = {}
    local cols = self.config.columns
    local rows = self.config.rows
    for row = 0, rows - 1 do
        -- Read each character in this row
        local chars = {}
        for col = 0, cols - 1 do
            local off = cell_offset(self, row, col)
            local ch = self.memory[off] or 0
            chars[#chars + 1] = string.char(ch == 0 and string.byte(" ") or ch)
        end
        -- Join into a string and strip trailing spaces
        local line = table.concat(chars)
        -- Remove trailing spaces
        line = line:gsub("%s+$", "")
        lines[#lines + 1] = line
    end
    return { lines = lines }
end

-- ---------------------------------------------------------------------------

return M
