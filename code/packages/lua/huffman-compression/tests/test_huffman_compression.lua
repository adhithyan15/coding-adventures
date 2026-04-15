-- ============================================================================
-- Tests for the Huffman Compression implementation (CMP04).
-- ============================================================================
--
-- Covers: round-trip invariants, wire format verification, edge cases,
-- compression effectiveness, and decoding correctness.
--
-- Uses Busted test framework: https://olivinelabs.com/busted/

package.path = "../src/?.lua;../src/?/init.lua;../../huffman-tree/src/?.lua;../../huffman-tree/src/?/init.lua;" .. package.path

local hc = require("coding_adventures.huffman_compression")

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- rt: compress then decompress — result must equal the original.
local function rt(str)
    return hc.decompress(hc.compress(str))
end

-- read_u32_be: read a big-endian uint32 from string `s` at byte position `pos`
-- (1-indexed).  Returns the integer value.
local function read_u32_be(s, pos)
    local b1, b2, b3, b4 = s:byte(pos, pos + 3)
    return b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
end

-- ── Version ───────────────────────────────────────────────────────────────────

describe("huffman_compression module", function()
    it("has VERSION '0.1.0'", function()
        assert.equals("0.1.0", hc.VERSION)
    end)

    it("exposes compress function", function()
        assert.is_function(hc.compress)
    end)

    it("exposes decompress function", function()
        assert.is_function(hc.decompress)
    end)
end)

-- ── Round-trip spec vectors ───────────────────────────────────────────────────

describe("round-trip spec vectors", function()
    it("empty string", function()
        assert.equals("", rt(""))
    end)

    it("single byte 'A'", function()
        assert.equals("A", rt("A"))
    end)

    it("two distinct bytes 'AB'", function()
        assert.equals("AB", rt("AB"))
    end)

    it("AAABBC — the canonical spec example", function()
        -- From the spec: A=3, B=2, C=1 → lengths A=1, B=2, C=2
        -- Canonical codes: A→"0", B→"10", C→"11"
        assert.equals("AAABBC", rt("AAABBC"))
    end)

    it("ABABAB — alternating pair", function()
        assert.equals("ABABAB", rt("ABABAB"))
    end)

    it("all same byte AAAAAA", function()
        assert.equals("AAAAAA", rt("AAAAAA"))
    end)

    it("all 256 distinct bytes", function()
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)

    it("hello world", function()
        assert.equals("hello world", rt("hello world"))
    end)

    it("null bytes", function()
        local s = "\0\0\0\255\255"
        assert.equals(s, rt(s))
    end)

    it("long repetitive string ABC × 1000", function()
        local data = string.rep("ABC", 1000)
        assert.equals(data, rt(data))
    end)

    it("longer repetitive string ABCDEF × 500", function()
        local data = string.rep("ABCDEF", 500)
        assert.equals(data, rt(data))
    end)

    it("single byte repeated 10000 times", function()
        local data = string.rep("Z", 10000)
        assert.equals(data, rt(data))
    end)

    it("binary data with all byte values twice", function()
        local chars = {}
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)

    it("long uniform run of zero bytes", function()
        local data = string.rep("\0", 5000)
        assert.equals(data, rt(data))
    end)
end)

-- ── Wire format verification ──────────────────────────────────────────────────

describe("wire format for 'AAABBC'", function()
    -- "AAABBC": A=3, B=2, C=1; symbol_count=3
    -- Canonical codes: A(65)→"0" len=1, B(66)→"10" len=2, C(67)→"11" len=2
    -- Bit string: "0" "0" "0" "10" "10" "11" = "000101011"  (9 bits → 2 bytes)
    -- But LSB-first pack:
    --   bits[1]='0' → pos 0: 0
    --   bits[2]='0' → pos 1: 0
    --   bits[3]='0' → pos 2: 0
    --   bits[4]='1' → pos 3: 8  → accumulator = 0b00001000
    --   bits[5]='0' → pos 4: 0  → accumulator = 0b00001000
    --   bits[6]='1' → pos 5: 32 → accumulator = 0b00101000
    --   bits[7]='0' → pos 6: 0  → accumulator = 0b00101000
    --   bits[8]='1' → pos 7: 128→ accumulator = 0b10101000 → byte 0 = 0xA8
    --   bits[9]='1' → pos 0 of byte 1: 1 → partial = 0b00000001 → byte 1 = 0x01
    -- So bit_bytes = "\xA8\x01"
    --
    -- Wire format:
    --   bytes 0–3:  original_length = 6        → \x00\x00\x00\x06
    --   bytes 4–7:  symbol_count    = 3        → \x00\x00\x00\x03
    --   bytes 8–9:  entry (A=65, len=1)        → \x41\x01
    --   bytes 10–11: entry (B=66, len=2)       → \x42\x02
    --   bytes 12–13: entry (C=67, len=2)       → \x43\x02
    --   bytes 14–15: bit stream                → \xA8\x01
    -- Total: 16 bytes

    local c

    before_each(function()
        c = hc.compress("AAABBC")
    end)

    it("has correct original_length in bytes 0–3", function()
        assert.equals(6, read_u32_be(c, 1))
    end)

    it("has correct symbol_count in bytes 4–7", function()
        assert.equals(3, read_u32_be(c, 5))
    end)

    it("has correct total byte length (16)", function()
        -- 4 + 4 + 3×2 + 2 = 16
        assert.equals(16, #c)
    end)

    it("first entry in code-length table is (A=65, len=1)", function()
        -- byte 8 = symbol 65 ('A'), byte 9 = code length 1
        assert.equals(65, c:byte(9))
        assert.equals(1,  c:byte(10))
    end)

    it("second entry is (B=66, len=2)", function()
        assert.equals(66, c:byte(11))
        assert.equals(2,  c:byte(12))
    end)

    it("third entry is (C=67, len=2)", function()
        assert.equals(67, c:byte(13))
        assert.equals(2,  c:byte(14))
    end)

    it("decompresses back to 'AAABBC'", function()
        assert.equals("AAABBC", hc.decompress(c))
    end)
end)

-- ── Wire format — header properties ──────────────────────────────────────────

describe("wire format header properties", function()
    it("original_length stored big-endian in bytes 0–3", function()
        local c = hc.compress("hello")
        assert.equals(5, read_u32_be(c, 1))
    end)

    it("300-byte input → correct original_length bytes", function()
        local data = string.rep("X", 300)
        local c    = hc.compress(data)
        -- 300 = 0x0000012C → bytes 0x00 0x00 0x01 0x2C
        assert.equals(0,    c:byte(1))
        assert.equals(0,    c:byte(2))
        assert.equals(1,    c:byte(3))
        assert.equals(0x2C, c:byte(4))
    end)

    it("empty input → 8-byte header, original_length=0, symbol_count=0", function()
        local c = hc.compress("")
        assert.equals(8, #c)
        assert.equals(0, read_u32_be(c, 1))
        assert.equals(0, read_u32_be(c, 5))
    end)

    it("single distinct symbol → symbol_count=1", function()
        local c = hc.compress("AAAA")
        assert.equals(1, read_u32_be(c, 5))
    end)

    it("all 256 distinct bytes → symbol_count=256", function()
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        local c = hc.compress(table.concat(chars))
        assert.equals(256, read_u32_be(c, 5))
    end)

    it("code-length entries are sorted by (len, sym)", function()
        -- Verify that the table entries in the header are sorted
        local c            = hc.compress("AAABBC")
        local symbol_count = read_u32_be(c, 5)
        local pos          = 9  -- 1-indexed start of table (after 8-byte header)
        local prev_len     = 0
        local prev_sym     = -1
        for _ = 1, symbol_count do
            local sym = c:byte(pos)
            local len = c:byte(pos + 1)
            -- Length must be non-decreasing
            assert.is_true(len >= prev_len,
                string.format("len %d < prev_len %d at sym %d", len, prev_len, sym))
            -- Within same length, symbol must be increasing
            if len == prev_len then
                assert.is_true(sym > prev_sym,
                    string.format("sym %d <= prev_sym %d at len %d", sym, prev_sym, len))
            end
            prev_len = len
            prev_sym = sym
            pos      = pos + 2
        end
    end)

    it("compress is deterministic", function()
        local data = "hello world test"
        assert.equals(hc.compress(data), hc.compress(data))
    end)
end)

-- ── Edge case robustness ──────────────────────────────────────────────────────

describe("edge case robustness", function()
    it("decompress empty string does not crash", function()
        local result = hc.decompress("")
        assert.is_string(result)
        assert.equals("", result)
    end)

    it("decompress 7-byte string (shorter than 8-byte header) does not crash", function()
        local result = hc.decompress("1234567")
        assert.is_string(result)
    end)

    it("decompress of all-zero 8-byte header (length=0, count=0) returns empty", function()
        local zeros = string.char(0, 0, 0, 0, 0, 0, 0, 0)
        assert.equals("", hc.decompress(zeros))
    end)

    it("single byte input round-trips", function()
        assert.equals("X", rt("X"))
    end)

    it("two bytes same value round-trips", function()
        assert.equals("AA", rt("AA"))
    end)

    it("two distinct bytes round-trips", function()
        assert.equals("XY", rt("XY"))
    end)

    it("binary data with null bytes round-trips", function()
        local s = string.char(0, 1, 2, 3, 0, 255, 128, 0)
        assert.equals(s, rt(s))
    end)

    it("all 256 byte values appear exactly once round-trips", function()
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)
end)

-- ── Compression effectiveness ─────────────────────────────────────────────────

describe("compression effectiveness", function()
    it("highly repetitive single-symbol data compresses vs raw", function()
        -- A single symbol needs only 1 bit per symbol. With 10000 'A's:
        -- Raw: 10000 bytes = 80000 bits.
        -- Compressed bits: 10000 × 1 = 10000 bits = 1250 bytes (+ header overhead).
        -- Even with header, compressed < raw for large inputs.
        local data = string.rep("A", 10000)
        local c    = hc.compress(data)
        assert.is_true(#c < #data)
    end)

    it("biased distribution 'ABC' × 1000 compresses vs raw", function()
        -- With unequal frequencies (A dominant), Huffman codes will be shorter
        -- on average than 8 bits/byte.
        local data = string.rep("A", 5000) .. string.rep("B", 3000) .. string.rep("C", 2000)
        local c    = hc.compress(data)
        assert.is_true(#c < #data)
    end)

    it("compress then decompress preserves exact bytes for all 256 values", function()
        local chars = {}
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)
end)

-- ── Decode correctness ────────────────────────────────────────────────────────

describe("decode correctness", function()
    it("decompresses 'AAABBC' to exactly 6 bytes", function()
        local result = hc.decompress(hc.compress("AAABBC"))
        assert.equals(6, #result)
        assert.equals("AAABBC", result)
    end)

    it("decompresses 'hello world' correctly", function()
        assert.equals("hello world", hc.decompress(hc.compress("hello world")))
    end)

    it("round-trip preserves exact length for long string", function()
        local data   = string.rep("abcde", 200)
        local result = rt(data)
        assert.equals(#data, #result)
        assert.equals(data, result)
    end)

    it("round-trip preserves content of random-looking data", function()
        -- Build a pseudo-random looking string using byte arithmetic.
        local chars = {}
        local state = 17
        for _ = 1, 500 do
            state = (state * 6364136223846793005 + 1) % 256
            chars[#chars + 1] = string.char(state)
        end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
    end)
end)
