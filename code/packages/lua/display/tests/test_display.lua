-- Tests for coding_adventures.display
--
-- Covers: DisplayConfig, DisplayDriver construction, put_char, puts_str,
--         snapshot, clear, newline, carriage return, tab, backspace,
--         cursor wrapping, and scrolling.
--
-- Lua 5.4 busted test suite.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local Display = require("coding_adventures.display")

-- Helper: create a fresh driver with a zeroed memory buffer.
local function make_driver(cols, rows)
    cols = cols or 80
    rows = rows or 25
    local config = Display.DisplayConfig.new({columns=cols, rows=rows})
    local memory = {}
    for i = 1, cols * rows * 2 do memory[i] = 0 end
    local driver = Display.DisplayDriver.new(config, memory)
    return driver
end

describe("display", function()

    -- -----------------------------------------------------------------------
    -- Version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(Display.VERSION)
        assert.equals("0.1.0", Display.VERSION)
    end)

    -- -----------------------------------------------------------------------
    -- DisplayConfig
    -- -----------------------------------------------------------------------

    it("DisplayConfig defaults to 80 columns and 25 rows", function()
        local cfg = Display.DisplayConfig.new()
        assert.equals(80, cfg.columns)
        assert.equals(25, cfg.rows)
    end)

    it("DisplayConfig respects custom dimensions", function()
        local cfg = Display.DisplayConfig.new({columns=40, rows=10})
        assert.equals(40, cfg.columns)
        assert.equals(10, cfg.rows)
    end)

    -- -----------------------------------------------------------------------
    -- DisplayDriver construction
    -- -----------------------------------------------------------------------

    it("DisplayDriver starts with cursor at (0, 0)", function()
        local drv = make_driver()
        assert.equals(0, drv.cursor_row)
        assert.equals(0, drv.cursor_col)
    end)

    -- -----------------------------------------------------------------------
    -- put_char — basic printable character
    -- -----------------------------------------------------------------------

    it("put_char writes a character to the framebuffer", function()
        local drv = make_driver(80, 25)
        drv:put_char(string.byte("H"))
        local snap = drv:snapshot()
        assert.equals("H", snap.lines[1])
    end)

    it("put_char advances the cursor by one column", function()
        local drv = make_driver(80, 25)
        drv:put_char(string.byte("A"))
        assert.equals(0, drv.cursor_row)
        assert.equals(1, drv.cursor_col)
    end)

    it("put_char writes multiple characters in sequence", function()
        local drv = make_driver(80, 25)
        drv:put_char(string.byte("H"))
        drv:put_char(string.byte("i"))
        local snap = drv:snapshot()
        assert.equals("Hi", snap.lines[1])
    end)

    -- -----------------------------------------------------------------------
    -- puts_str
    -- -----------------------------------------------------------------------

    it("puts_str writes a full string", function()
        local drv = make_driver(80, 25)
        drv:puts_str("Hello World")
        local snap = drv:snapshot()
        assert.equals("Hello World", snap.lines[1])
    end)

    -- -----------------------------------------------------------------------
    -- snapshot line count
    -- -----------------------------------------------------------------------

    it("snapshot returns one entry per row", function()
        local drv = make_driver(80, 25)
        local snap = drv:snapshot()
        assert.equals(25, #snap.lines)
    end)

    it("snapshot lines are empty strings for blank rows", function()
        local drv = make_driver(80, 25)
        local snap = drv:snapshot()
        for _, line in ipairs(snap.lines) do
            assert.equals("", line)
        end
    end)

    -- -----------------------------------------------------------------------
    -- Newline (0x0A)
    -- -----------------------------------------------------------------------

    it("newline moves cursor to start of the next row", function()
        local drv = make_driver(80, 25)
        drv:puts_str("Hi")
        drv:put_char(0x0A)
        assert.equals(1, drv.cursor_row)
        assert.equals(0, drv.cursor_col)
    end)

    it("newline followed by text writes to the second row", function()
        local drv = make_driver(80, 25)
        drv:puts_str("Row1")
        drv:put_char(0x0A)
        drv:puts_str("Row2")
        local snap = drv:snapshot()
        assert.equals("Row1", snap.lines[1])
        assert.equals("Row2", snap.lines[2])
    end)

    -- -----------------------------------------------------------------------
    -- Carriage return (0x0D)
    -- -----------------------------------------------------------------------

    it("carriage return moves cursor to column 0 on the same row", function()
        local drv = make_driver(80, 25)
        drv:puts_str("ABC")
        drv:put_char(0x0D)
        assert.equals(0, drv.cursor_row)
        assert.equals(0, drv.cursor_col)
    end)

    it("carriage return then writing overwrites from column 0", function()
        local drv = make_driver(80, 25)
        drv:puts_str("XXXX")
        drv:put_char(0x0D)
        drv:puts_str("Hi")
        local snap = drv:snapshot()
        -- "Hi" overwrites the first two chars of "XXXX"
        assert.equals("HiXX", snap.lines[1])
    end)

    -- -----------------------------------------------------------------------
    -- Tab (0x09)
    -- -----------------------------------------------------------------------

    it("tab advances cursor to the next multiple of 8", function()
        local drv = make_driver(80, 25)
        drv:puts_str("AB")       -- cursor at col 2
        drv:put_char(0x09)       -- next tab stop is 8
        assert.equals(8, drv.cursor_col)
    end)

    it("tab from column 0 advances to column 8", function()
        local drv = make_driver(80, 25)
        drv:put_char(0x09)
        assert.equals(8, drv.cursor_col)
    end)

    -- -----------------------------------------------------------------------
    -- Backspace (0x08)
    -- -----------------------------------------------------------------------

    it("backspace moves cursor left one column", function()
        local drv = make_driver(80, 25)
        drv:puts_str("AB")
        drv:put_char(0x08)
        assert.equals(1, drv.cursor_col)
    end)

    it("backspace at column 0 does not move cursor further left", function()
        local drv = make_driver(80, 25)
        drv:put_char(0x08)
        assert.equals(0, drv.cursor_col)
    end)

    -- -----------------------------------------------------------------------
    -- Line wrap
    -- -----------------------------------------------------------------------

    it("writing past the last column wraps to the next row", function()
        local drv = make_driver(5, 10)   -- only 5 columns wide
        drv:puts_str("ABCDE")            -- fills row 0 columns 0-4
        -- cursor should now be at row 1, col 0
        assert.equals(1, drv.cursor_row)
        assert.equals(0, drv.cursor_col)
    end)

    -- -----------------------------------------------------------------------
    -- Scrolling
    -- -----------------------------------------------------------------------

    it("scrolling up when the last row is full shifts content up", function()
        local drv = make_driver(10, 3)   -- 3 rows, 10 cols
        drv:puts_str("Row0")
        drv:put_char(0x0A)
        drv:puts_str("Row1")
        drv:put_char(0x0A)
        drv:puts_str("Row2")
        drv:put_char(0x0A)  -- trigger scroll
        drv:puts_str("Row3")

        local snap = drv:snapshot()
        assert.equals("Row1", snap.lines[1])
        assert.equals("Row2", snap.lines[2])
        assert.equals("Row3", snap.lines[3])
    end)

    -- -----------------------------------------------------------------------
    -- clear
    -- -----------------------------------------------------------------------

    it("clear resets cursor to (0, 0)", function()
        local drv = make_driver(80, 25)
        drv:puts_str("stuff")
        drv:clear()
        assert.equals(0, drv.cursor_row)
        assert.equals(0, drv.cursor_col)
    end)

    it("clear makes all lines blank", function()
        local drv = make_driver(80, 5)
        drv:puts_str("Line 0")
        drv:put_char(0x0A)
        drv:puts_str("Line 1")
        drv:clear()
        local snap = drv:snapshot()
        for _, line in ipairs(snap.lines) do
            assert.equals("", line)
        end
    end)

end)
