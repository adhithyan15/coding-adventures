-- ============================================================================
-- Tests for the LZW compression implementation.
-- ============================================================================
--
-- Test vectors come from the CMP03 specification. Covers: spec vectors,
-- encode properties, decode correctness, round-trip invariants, wire format,
-- and compression effectiveness. Uses Busted test framework.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local lzw = require("coding_adventures.lzw")

-- Module-level round-trip helper.
-- Compresses and immediately decompresses `str`; result must equal `str`.
local function rt(str)
    return lzw.decompress(lzw.compress(str))
end

-- ── Version ──────────────────────────────────────────────────────────────────

describe("lzw", function()
    it("has VERSION", function()
        assert.equals("0.1.0", lzw.VERSION)
    end)

    it("exposes constants", function()
        assert.equals(256, lzw.CLEAR_CODE)
        assert.equals(257, lzw.STOP_CODE)
        assert.equals(258, lzw.INITIAL_NEXT_CODE)
        assert.equals(9,   lzw.INITIAL_CODE_SIZE)
        assert.equals(16,  lzw.MAX_CODE_SIZE)
    end)
end)

-- ── Round-trip spec vectors ───────────────────────────────────────────────────

describe("round-trip spec vectors", function()
    it("empty string", function()
        assert.equals("", rt(""))
    end)

    it("single byte A", function()
        assert.equals("A", rt("A"))
    end)

    it("AB", function()
        assert.equals("AB", rt("AB"))
    end)

    it("ABABAB", function()
        assert.equals("ABABAB", rt("ABABAB"))
    end)

    it("AAAAAAA — tricky token", function()
        -- "AAAAAAA" triggers the tricky-token case: the encoder emits a code
        -- for a sequence the decoder hasn't added yet.  The decoder recovers
        -- by repeating the first byte of the previous entry.
        assert.equals("AAAAAAA", rt("AAAAAAA"))
    end)

    it("all 256 bytes", function()
        -- Build a string containing bytes 0–255 in order.
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)

    it("repetitive data compresses", function()
        -- A highly repetitive string should produce fewer bytes than the input.
        local data = string.rep("ABC", 1000)
        local compressed = lzw.compress(data)
        assert.is_true(#compressed < #data)
    end)
end)

-- ── Round-trip extra coverage ─────────────────────────────────────────────────

describe("round-trip extra coverage", function()
    it("no repetition ABCDE", function()
        assert.equals("ABCDE", rt("ABCDE"))
    end)

    it("AABCBBABC", function()
        assert.equals("AABCBBABC", rt("AABCBBABC"))
    end)

    it("hello world", function()
        assert.equals("hello world", rt("hello world"))
    end)

    it("ABC repeated 100 times", function()
        local data = string.rep("ABC", 100)
        assert.equals(data, rt(data))
    end)

    it("null bytes", function()
        local s = "\0\0\0\255\255"
        assert.equals(s, rt(s))
    end)

    it("repeated pattern 0,1,2,0,1,2,...", function()
        local chars = {}
        for i = 0, 299 do chars[i + 1] = string.char(i % 3) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)

    it("long ABCDEF repeated 500 times", function()
        local data = string.rep("ABCDEF", 500)
        assert.equals(data, rt(data))
    end)

    it("all same byte compresses significantly", function()
        local data = string.rep("\66", 10000)
        local compressed = lzw.compress(data)
        assert.is_true(#compressed < #data)
        assert.equals(data, lzw.decompress(compressed))
    end)

    it("binary data with all byte values", function()
        -- Round-trip 256 bytes followed by 256 reversed bytes.
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        for i = 255, 0, -1 do chars[#chars + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)

    it("long uniform run of zero bytes", function()
        local data = string.rep("\0", 5000)
        assert.equals(data, rt(data))
    end)
end)

-- ── Wire format ───────────────────────────────────────────────────────────────

describe("wire format", function()
    it("compress stores original_length in first 4 bytes (big-endian)", function()
        local compressed = lzw.compress("hello")
        local b1, b2, b3, b4 = compressed:byte(1, 4)
        local orig_len = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
        assert.equals(5, orig_len)
    end)

    it("compress empty → 4-byte header only (original_length=0, no codes)", function()
        -- Empty input: CLEAR_CODE + STOP_CODE packed in 9 bits each = 18 bits = 3 bytes
        -- plus the 4-byte header = 7 bytes total.
        local c = lzw.compress("")
        assert.is_true(#c >= 4)
        local b1, b2, b3, b4 = c:byte(1, 4)
        local orig_len = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
        assert.equals(0, orig_len)
    end)

    it("compress is deterministic", function()
        local data = "hello world test"
        assert.equals(lzw.compress(data), lzw.compress(data))
    end)

    it("compressed output starts with big-endian length header", function()
        -- 300-byte input → first 4 bytes should be 0x00 0x00 0x01 0x2C
        local data = string.rep("X", 300)
        local c    = lzw.compress(data)
        assert.equals(0,    c:byte(1))
        assert.equals(0,    c:byte(2))
        assert.equals(1,    c:byte(3))
        assert.equals(0x2C, c:byte(4))
    end)

    it("decompress of truncated data does not crash", function()
        -- Only a 3-byte string (shorter than the 4-byte header).
        local result = lzw.decompress("abc")
        assert.is_string(result)
    end)

    it("decompress of empty string does not crash", function()
        local result = lzw.decompress("")
        assert.is_string(result)
    end)

    it("decompress of garbage does not crash", function()
        -- 4-byte header claiming length 999, followed by random bytes.
        local junk = string.char(0, 0, 3, 231) ..  -- original_length = 999
                     string.char(0xFF, 0xFE, 0xAB, 0xCD, 0x12, 0x34)
        local result = lzw.decompress(junk)
        assert.is_string(result)
    end)
end)

-- ── Compression effectiveness ─────────────────────────────────────────────────

describe("compression effectiveness", function()
    it("repetitive ABC*1000 compresses to less than input", function()
        local data = string.rep("ABC", 1000)
        assert.is_true(#lzw.compress(data) < #data)
    end)

    it("repetitive single-char data compresses to less than input", function()
        local data = string.rep("Z", 5000)
        assert.is_true(#lzw.compress(data) < #data)
    end)

    it("compress then decompress preserves all 256 distinct bytes", function()
        -- Each byte value appears exactly twice.
        local chars = {}
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)
end)
