-- ============================================================================
-- Tests for the LZ77 compression implementation.
-- ============================================================================
--
-- Test vectors come from the CMP00 specification. Covers: literals,
-- backreferences, overlapping matches, edge cases, and round-trip invariants.
-- Uses Busted test framework (https://olivinelabs.com/busted/).

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local lz77 = require("coding_adventures.lz77")

-- Helper: check if two token tables are equal.
local function token_eq(t1, t2)
    return t1.offset == t2.offset
        and t1.length == t2.length
        and t1.next_char == t2.next_char
end

-- Helper: check all tokens in a list satisfy a predicate.
local function all_tokens(tokens, pred)
    for _, t in ipairs(tokens) do
        if not pred(t) then return false end
    end
    return true
end

-- Helper: convert string to byte array for encode().
local function str_to_bytes(s)
    return {s:byte(1, #s)}
end

-- ---- Version ----

describe("lz77", function()
    it("has VERSION", function()
        assert.equals("0.1.0", lz77.VERSION)
    end)
end)

-- ---- Specification Test Vectors ----

describe("spec vectors", function()
    it("empty input produces no tokens", function()
        local tokens = lz77.encode({})
        assert.equals(0, #tokens)
        local result = lz77.decode({})
        assert.equals(0, #result)
    end)

    it("no repetition → all literal tokens", function()
        local tokens = lz77.encode(str_to_bytes("ABCDE"))
        assert.equals(5, #tokens)
        assert.is_true(all_tokens(tokens, function(t)
            return t.offset == 0 and t.length == 0
        end))
    end)

    it("all identical bytes exploit overlap", function()
        local tokens = lz77.encode(str_to_bytes("AAAAAAA"))
        assert.equals(2, #tokens)
        assert.is_true(token_eq(tokens[1], lz77.token(0, 0, 65)))
        assert.equals(1, tokens[2].offset)
        assert.equals(5, tokens[2].length)
        assert.equals(65, tokens[2].next_char)
        assert.equals("AAAAAAA", lz77.decode_to_string(tokens))
    end)

    it("repeated pair uses backreference", function()
        local tokens = lz77.encode(str_to_bytes("ABABABAB"))
        assert.equals(3, #tokens)
        assert.is_true(token_eq(tokens[1], lz77.token(0, 0, 65)))
        assert.is_true(token_eq(tokens[2], lz77.token(0, 0, 66)))
        assert.equals(2, tokens[3].offset)
        assert.equals(5, tokens[3].length)
        assert.equals(66, tokens[3].next_char)
        assert.equals("ABABABAB", lz77.decode_to_string(tokens))
    end)

    it("AABCBBABC with min_match=3 → all literals", function()
        local tokens = lz77.encode(str_to_bytes("AABCBBABC"))
        assert.equals(9, #tokens)
        assert.is_true(all_tokens(tokens, function(t)
            return t.offset == 0 and t.length == 0
        end))
        assert.equals("AABCBBABC", lz77.decode_to_string(tokens))
    end)

    it("AABCBBABC with min_match=2 round-trips", function()
        local tokens = lz77.encode(str_to_bytes("AABCBBABC"), 4096, 255, 2)
        assert.equals("AABCBBABC", lz77.decode_to_string(tokens))
    end)
end)

-- ---- Round-Trip Tests ----

-- Module-level round-trip helper: compress then decompress.
local function rt(str)
    return lz77.decompress(lz77.compress(str))
end

describe("round-trip invariants", function()
    it("empty", function()
        assert.equals("", rt(""))
    end)

    it("single byte", function()
        assert.equals("A", rt("A"))
    end)

    it("hello world", function()
        assert.equals("hello world", rt("hello world"))
    end)

    it("the quick brown fox", function()
        assert.equals("the quick brown fox", rt("the quick brown fox"))
    end)

    it("ababababab", function()
        assert.equals("ababababab", rt("ababababab"))
    end)

    it("aaaaaaaaaa", function()
        assert.equals("aaaaaaaaaa", rt("aaaaaaaaaa"))
    end)

    it("null bytes", function()
        local s = "\0\0\0"
        assert.equals(s, rt(s))
    end)

    it("all spec vectors", function()
        for _, s in ipairs({"", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"}) do
            assert.equals(s, rt(s), "round-trip failed for: " .. s)
        end
    end)
end)

-- ---- Parameter Tests ----

describe("parameters", function()
    it("offsets never exceed window_size", function()
        local data = str_to_bytes("X" .. string.rep("Y", 5000) .. "X")
        local tokens = lz77.encode(data, 100)
        assert.is_true(all_tokens(tokens, function(t) return t.offset <= 100 end))
    end)

    it("lengths never exceed max_match", function()
        local data = str_to_bytes(string.rep("A", 1000))
        local tokens = lz77.encode(data, 4096, 50)
        assert.is_true(all_tokens(tokens, function(t) return t.length <= 50 end))
    end)

    it("min_match threshold respected", function()
        local tokens = lz77.encode(str_to_bytes("AABAA"), 4096, 255, 2)
        assert.is_true(all_tokens(tokens, function(t)
            return t.length == 0 or t.length >= 2
        end))
    end)
end)

-- ---- Edge Cases ----

describe("edge cases", function()
    it("single byte encodes as literal", function()
        local tokens = lz77.encode(str_to_bytes("X"))
        assert.equals(1, #tokens)
        assert.is_true(token_eq(tokens[1], lz77.token(0, 0, 88)))
    end)

    it("exact window boundary match", function()
        local data = str_to_bytes(string.rep("X", 11))
        local tokens = lz77.encode(data, 10)
        local found_match = false
        for _, t in ipairs(tokens) do
            if t.offset > 0 then found_match = true end
        end
        assert.is_true(found_match, "expected at least one match")
        assert.equals(string.rep("X", 11), lz77.decode_to_string(tokens))
    end)

    it("overlapping match decoded byte-by-byte", function()
        -- [A, B] + (offset=2, length=5, next_char='Z') -> ABABABAZ
        local tokens = {
            lz77.token(0, 0, 65),
            lz77.token(0, 0, 66),
            lz77.token(2, 5, 90)
        }
        assert.equals("ABABABAZ", lz77.decode_to_string(tokens))
    end)

    it("binary with nulls", function()
        local s = "\0\0\0\255\255"
        assert.equals(s, rt(s))
    end)

    it("long run of identical bytes compresses well", function()
        local data = string.rep("A", 10000)
        local tokens = lz77.encode(str_to_bytes(data))
        assert.is_true(#tokens < 50, "expected < 50 tokens, got " .. #tokens)
        assert.equals(data, lz77.decode_to_string(tokens))
    end)

    it("very long input", function()
        local data = string.rep("Hello, World! ", 100) .. string.rep("X", 500)
        assert.equals(data, rt(data))
    end)
end)

-- ---- Serialisation Tests ----

describe("serialisation", function()
    it("format is 4 + N*4 bytes", function()
        local tokens = {lz77.token(0, 0, 65), lz77.token(2, 5, 66)}
        local serialised = lz77.serialise_tokens(tokens)
        assert.equals(4 + 2 * 4, #serialised)
    end)

    it("serialise/deserialise is a no-op", function()
        local tokens = {
            lz77.token(0, 0, 65),
            lz77.token(1, 3, 66),
            lz77.token(2, 5, 67)
        }
        local serialised = lz77.serialise_tokens(tokens)
        local got = lz77.deserialise_tokens(serialised)
        assert.equals(#tokens, #got)
        for i = 1, #tokens do
            assert.is_true(token_eq(tokens[i], got[i]))
        end
    end)

    it("empty compress/decompress", function()
        assert.equals("", lz77.decompress(lz77.compress("")))
    end)
end)

-- ---- Behaviour Tests ----

describe("behaviour", function()
    it("incompressible data does not expand beyond 4N+10 bytes", function()
        local chars = {}
        for i = 0, 255 do chars[#chars + 1] = string.char(i) end
        local data = table.concat(chars)
        local compressed = lz77.compress(data)
        assert.is_true(#compressed <= 4 * #data + 10)
    end)

    it("repetitive data compresses significantly", function()
        local data = string.rep("ABC", 100)
        local compressed = lz77.compress(data)
        assert.is_true(#compressed < #data)
    end)

    it("compression is deterministic", function()
        local data = "hello world test"
        assert.equals(lz77.compress(data), lz77.compress(data))
    end)
end)
