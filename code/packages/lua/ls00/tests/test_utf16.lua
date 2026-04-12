-- test_utf16.lua — UTF-16 offset conversion tests
-- =================================================
--
-- These tests verify the critical UTF-16 -> byte offset conversion.
-- This is the most important correctness test in the entire package.
-- If this function is wrong, every feature that depends on cursor position
-- will be wrong: hover, go-to-definition, references, completion, rename,
-- and signature help.
--
-- # Why UTF-16?
--
-- LSP character offsets are measured in UTF-16 code units because VS Code
-- uses TypeScript internally (which has UTF-16 strings). Lua strings are
-- byte strings (UTF-8). This function bridges the gap.
--
-- # Test Coverage
--
-- We test:
--   1. ASCII strings (1 byte per character, 1 UTF-16 unit)
--   2. 2-byte UTF-8 (BMP codepoints like e-acute, still 1 UTF-16 unit)
--   3. 4-byte UTF-8 (above U+FFFF, like emoji, 2 UTF-16 units)
--   4. Multi-line strings (line boundaries)
--   5. Edge cases (start of file, end of line, beyond line end)

local ls00 = require("coding_adventures.ls00")

describe("UTF-16 offset conversion", function()
    -- Helper: Lua is 1-based, so we compare the 1-based byte position
    -- returned by convert_utf16_offset_to_byte_offset against expected values.
    -- The function returns a 1-based byte position for use with string.sub().

    it("handles ASCII simple", function()
        -- "hello world" — all ASCII, 1 byte = 1 UTF-16 unit.
        -- "world" starts at byte 7 (1-based).
        local pos = ls00.convert_utf16_offset_to_byte_offset("hello world", 0, 6)
        assert.are.equal(7, pos)
    end)

    it("handles start of file", function()
        local pos = ls00.convert_utf16_offset_to_byte_offset("abc", 0, 0)
        assert.are.equal(1, pos)
    end)

    it("handles end of short string", function()
        -- "abc" — character 3 is one past the end, byte 4 (1-based).
        local pos = ls00.convert_utf16_offset_to_byte_offset("abc", 0, 3)
        assert.are.equal(4, pos)
    end)

    it("handles second line", function()
        -- "hello\nworld" — line 1 starts at byte 7 (1-based).
        local pos = ls00.convert_utf16_offset_to_byte_offset("hello\nworld", 1, 0)
        assert.are.equal(7, pos)
    end)

    it("handles emoji surrogate pairs", function()
        -- "A🎸B"
        -- UTF-8 bytes: A(1) + 🎸(4) + B(1) = 6 bytes total.
        -- UTF-16 units: A(1) + 🎸(2) + B(1) = 4 units total.
        -- "B" is at UTF-16 character 3, byte offset 6 (1-based).
        --
        -- 🎸 is U+1F3B8, encoded as the 4-byte sequence: F0 9F 8E B8
        local text = "A\xF0\x9F\x8E\xB8B"
        local pos = ls00.convert_utf16_offset_to_byte_offset(text, 0, 3)
        assert.are.equal(6, pos)
    end)

    it("handles emoji at start", function()
        -- "🎸hello"
        -- 🎸 = 2 UTF-16 units = 4 UTF-8 bytes.
        -- "h" is at UTF-16 char 2, byte offset 5 (1-based).
        local text = "\xF0\x9F\x8E\xB8hello"
        local pos = ls00.convert_utf16_offset_to_byte_offset(text, 0, 2)
        assert.are.equal(5, pos)
    end)

    it("handles 2-byte UTF-8 BMP codepoint", function()
        -- "cafe!" where e is actually e-acute (U+00E9).
        -- e-acute in UTF-8: 2 bytes (C3 A9).
        -- In UTF-16: 1 code unit (BMP codepoint).
        -- UTF-16 char 4 = the "!" = byte offset 6 (c=1, a=1, f=1, e-acute=2, !=1).
        -- 1-based: byte 6.
        local text = "caf\xC3\xA9!"
        local pos = ls00.convert_utf16_offset_to_byte_offset(text, 0, 4)
        assert.are.equal(6, pos)
    end)

    it("handles multiline with emoji", function()
        -- line 0: "A🎸B\n"  (A=1, 🎸=4, B=1, \n=1 = 7 bytes)
        -- line 1: "hello"
        -- "hello" starts at byte 8 (1-based), char 0 on line 1.
        local text = "A\xF0\x9F\x8E\xB8B\nhello"
        local pos = ls00.convert_utf16_offset_to_byte_offset(text, 1, 0)
        assert.are.equal(8, pos)
    end)

    it("clamps beyond line end to newline position", function()
        -- If character is past the end of the line, we stop at the newline.
        -- "ab\ncd" — line 0 has 2 chars (a, b), then newline at byte 3.
        -- Character 100 on line 0 should clamp to byte 3 (1-based, the newline).
        local pos = ls00.convert_utf16_offset_to_byte_offset("ab\ncd", 0, 100)
        assert.are.equal(3, pos)
    end)

    it("handles empty string", function()
        local pos = ls00.convert_utf16_offset_to_byte_offset("", 0, 0)
        assert.are.equal(1, pos)
    end)

    it("handles 3-byte UTF-8 BMP codepoint", function()
        -- Chinese character 中 (U+4E2D): 3 UTF-8 bytes, 1 UTF-16 unit.
        -- "A中B" — byte layout: A(1) + 中(3) + B(1) = 5 bytes.
        -- "B" is at UTF-16 char 2, byte 5 (1-based).
        local text = "A\xE4\xB8\xADB"
        local pos = ls00.convert_utf16_offset_to_byte_offset(text, 0, 2)
        assert.are.equal(5, pos)
    end)
end)
