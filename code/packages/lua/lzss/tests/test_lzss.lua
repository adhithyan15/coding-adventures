-- ============================================================================
-- Tests for the LZSS compression implementation.
-- ============================================================================
--
-- Test vectors come from the CMP02 specification. Covers: spec vectors,
-- encode properties, decode correctness, round-trip invariants, wire format,
-- and compression effectiveness. Uses Busted test framework.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local lzss = require("coding_adventures.lzss")

-- Helper: check if a token is a literal matching the given byte.
local function is_literal(tok, byte)
    return tok.kind == "literal" and tok.byte == byte
end

-- Helper: check if a token is a match with given offset and length.
local function is_match(tok, offset, length)
    return tok.kind == "match" and tok.offset == offset and tok.length == length
end

-- Helper: check all tokens satisfy a predicate.
local function all(tokens, pred)
    for _, t in ipairs(tokens) do
        if not pred(t) then return false end
    end
    return true
end

-- Module-level round-trip helper.
local function rt(str)
    return lzss.decompress(lzss.compress(str))
end

-- ── Version ──────────────────────────────────────────────────────────────────

describe("lzss", function()
    it("has VERSION", function()
        assert.equals("0.1.0", lzss.VERSION)
    end)
end)

-- ── Spec vectors ─────────────────────────────────────────────────────────────

describe("spec vectors — encode", function()
    it("encode empty → []", function()
        local tokens = lzss.encode({})
        assert.equals(0, #tokens)
    end)

    it("encode single byte → [Literal(65)]", function()
        local tokens = lzss.encode({65})
        assert.equals(1, #tokens)
        assert.is_true(is_literal(tokens[1], 65))
    end)

    it("encode no repetition → all literals", function()
        local tokens = lzss.encode_string("ABCDE")
        assert.equals(5, #tokens)
        assert.is_true(all(tokens, function(t) return t.kind == "literal" end))
    end)

    it("encode AABCBBABC → 7 tokens, last is Match(5,3)", function()
        local tokens = lzss.encode_string("AABCBBABC")
        assert.equals(7, #tokens)
        assert.is_true(is_match(tokens[7], 5, 3))
    end)

    it("encode ABABAB → [Lit(A), Lit(B), Match(2,4)]", function()
        local tokens = lzss.encode_string("ABABAB")
        assert.equals(3, #tokens)
        assert.is_true(is_literal(tokens[1], 65))
        assert.is_true(is_literal(tokens[2], 66))
        assert.is_true(is_match(tokens[3], 2, 4))
    end)

    it("encode AAAAAAA → [Lit(A), Match(1,6)]", function()
        local tokens = lzss.encode_string("AAAAAAA")
        assert.equals(2, #tokens)
        assert.is_true(is_literal(tokens[1], 65))
        assert.is_true(is_match(tokens[2], 1, 6))
    end)
end)

-- ── Encode properties ─────────────────────────────────────────────────────────

describe("encode properties", function()
    it("match offset >= 1", function()
        local tokens = lzss.encode_string("ABABABAB")
        assert.is_true(all(tokens, function(t)
            return t.kind ~= "match" or t.offset >= 1
        end))
    end)

    it("match length >= min_match (default 3)", function()
        local tokens = lzss.encode_string("ABABABABABAB")
        assert.is_true(all(tokens, function(t)
            return t.kind ~= "match" or t.length >= 3
        end))
    end)

    it("large min_match forces all literals", function()
        local tokens = lzss.encode_string("ABABAB", 4096, 255, 100)
        assert.is_true(all(tokens, function(t) return t.kind == "literal" end))
    end)
end)

-- ── Decode ────────────────────────────────────────────────────────────────────

describe("decode", function()
    it("decode empty → empty string", function()
        assert.equals("", lzss.decode_to_string({}, 0))
    end)

    it("decode single literal", function()
        assert.equals("A", lzss.decode_to_string({lzss.literal(65)}, 1))
    end)

    it("decode overlapping match AAAAAAA", function()
        local tokens = {lzss.literal(65), lzss.match(1, 6)}
        assert.equals("AAAAAAA", lzss.decode_to_string(tokens, 7))
    end)

    it("decode ABABAB", function()
        local tokens = {lzss.literal(65), lzss.literal(66), lzss.match(2, 4)}
        assert.equals("ABABAB", lzss.decode_to_string(tokens, 6))
    end)
end)

-- ── Round-trip ────────────────────────────────────────────────────────────────

describe("round-trip", function()
    it("empty", function()
        assert.equals("", rt(""))
    end)

    it("single byte", function()
        assert.equals("A", rt("A"))
    end)

    it("no repetition ABCDE", function()
        assert.equals("ABCDE", rt("ABCDE"))
    end)

    it("all identical AAAAAAA", function()
        assert.equals("AAAAAAA", rt("AAAAAAA"))
    end)

    it("ABABAB", function()
        assert.equals("ABABAB", rt("ABABAB"))
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

    it("full byte range", function()
        local chars = {}
        for i = 0, 255 do chars[i + 1] = string.char(i) end
        local data = table.concat(chars)
        assert.equals(data, rt(data))
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
end)

-- ── Wire format ───────────────────────────────────────────────────────────────

describe("wire format", function()
    it("compress stores original length in first 4 bytes", function()
        local compressed = lzss.compress("hello")
        local b1, b2, b3, b4 = compressed:byte(1, 4)
        local orig_len = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
        assert.equals(5, orig_len)
    end)

    it("compress empty → 8-byte header, block_count=0", function()
        local c = lzss.compress("")
        assert.equals(8, #c)
        local b1, b2, b3, b4 = c:byte(1, 4)
        local orig_len = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
        assert.equals(0, orig_len)
        local n1, n2, n3, n4 = c:byte(5, 8)
        local block_count = n1 * 16777216 + n2 * 65536 + n3 * 256 + n4
        assert.equals(0, block_count)
    end)

    it("compress is deterministic", function()
        local data = "hello world test"
        assert.equals(lzss.compress(data), lzss.compress(data))
    end)

    it("crafted large block_count is safe", function()
        -- Craft a header claiming 0x40000000 blocks but only 5 payload bytes.
        local header =
            string.char(0, 0, 0, 4) ..   -- original_length = 4
            string.char(0x40, 0, 0, 0) .. -- block_count = 0x40000000 (huge)
            string.char(0, 65, 66, 67, 68)
        local result = lzss.decompress(header)
        assert.is_string(result)
    end)
end)

-- ── Compression effectiveness ─────────────────────────────────────────────────

describe("compression effectiveness", function()
    it("repetitive data compresses", function()
        local data = string.rep("ABC", 1000)
        assert.is_true(#lzss.compress(data) < #data)
    end)

    it("all same byte compresses significantly", function()
        local data = string.rep("\66", 10000)
        local compressed = lzss.compress(data)
        assert.is_true(#compressed < #data)
        assert.equals(data, lzss.decompress(compressed))
    end)
end)
