-- Tests for wasm_leb128
--
-- Comprehensive test suite for LEB128 encoding/decoding, covering the
-- WebAssembly spec examples, edge cases, round-trips, and error conditions.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.wasm_leb128")

-- Helper: compare two byte arrays for equality
local function bytes_equal(a, b)
    if #a ~= #b then return false end
    for i = 1, #a do
        if a[i] ~= b[i] then return false end
    end
    return true
end

describe("wasm_leb128", function()

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("exposes encode_unsigned", function()
        assert.is_function(m.encode_unsigned)
    end)

    it("exposes encode_signed", function()
        assert.is_function(m.encode_signed)
    end)

    it("exposes decode_unsigned", function()
        assert.is_function(m.decode_unsigned)
    end)

    it("exposes decode_signed", function()
        assert.is_function(m.decode_signed)
    end)

    -- -----------------------------------------------------------------------
    -- encode_unsigned — basic values
    -- -----------------------------------------------------------------------

    it("encode_unsigned(0) == {0x00}", function()
        assert.is_true(bytes_equal({0x00}, m.encode_unsigned(0)))
    end)

    it("encode_unsigned(1) == {0x01}", function()
        assert.is_true(bytes_equal({0x01}, m.encode_unsigned(1)))
    end)

    it("encode_unsigned(63) == {0x3F}", function()
        -- 63 = 0b0111111, fits in 7 bits, no continuation
        assert.is_true(bytes_equal({0x3F}, m.encode_unsigned(63)))
    end)

    it("encode_unsigned(127) == {0x7F}", function()
        -- 127 = 0b1111111, max single-byte unsigned
        assert.is_true(bytes_equal({0x7F}, m.encode_unsigned(127)))
    end)

    it("encode_unsigned(128) == {0x80, 0x01}", function()
        -- 128 requires 2 bytes
        assert.is_true(bytes_equal({0x80, 0x01}, m.encode_unsigned(128)))
    end)

    it("encode_unsigned(255) == {0xFF, 0x01}", function()
        assert.is_true(bytes_equal({0xFF, 0x01}, m.encode_unsigned(255)))
    end)

    it("encode_unsigned(300) == {0xAC, 0x02}", function()
        -- 300 = 0b100101100
        -- byte 1: 300 & 0x7F = 44 = 0x2C, set continuation → 0xAC
        -- byte 2: 300 >> 7 = 2, no continuation → 0x02
        assert.is_true(bytes_equal({0xAC, 0x02}, m.encode_unsigned(300)))
    end)

    it("encode_unsigned(624485) == {0xE5, 0x8E, 0x26}", function()
        -- WebAssembly spec example
        assert.is_true(bytes_equal({0xE5, 0x8E, 0x26}, m.encode_unsigned(624485)))
    end)

    -- -----------------------------------------------------------------------
    -- encode_signed — basic values
    -- -----------------------------------------------------------------------

    it("encode_signed(0) == {0x00}", function()
        assert.is_true(bytes_equal({0x00}, m.encode_signed(0)))
    end)

    it("encode_signed(1) == {0x01}", function()
        assert.is_true(bytes_equal({0x01}, m.encode_signed(1)))
    end)

    it("encode_signed(-1) == {0x7F}", function()
        -- -1 in signed LEB128 is a single byte with all 7 bits set
        assert.is_true(bytes_equal({0x7F}, m.encode_signed(-1)))
    end)

    it("encode_signed(-2) == {0x7E}", function()
        -- WebAssembly spec example
        assert.is_true(bytes_equal({0x7E}, m.encode_signed(-2)))
    end)

    it("encode_signed(63) == {0x3F}", function()
        -- 63 fits in 6 payload bits with sign bit 0 → single byte
        assert.is_true(bytes_equal({0x3F}, m.encode_signed(63)))
    end)

    it("encode_signed(64) == {0xC0, 0x00}", function()
        -- 64 = 0b1000000: bit 6 would be 0 but bit 7 is set → needs two bytes
        -- Actually: 64 & 0x7F = 64 = 0b1000000, bit 6 = 1 (sign bit!) so continuation
        assert.is_true(bytes_equal({0xC0, 0x00}, m.encode_signed(64)))
    end)

    it("encode_signed(-64) == {0x40}", function()
        -- -64 in 7 bits: 0b1000000 = 64 in two's complement of 7 bits
        assert.is_true(bytes_equal({0x40}, m.encode_signed(-64)))
    end)

    it("encode_signed(-128) == {0x80, 0x7F}", function()
        assert.is_true(bytes_equal({0x80, 0x7F}, m.encode_signed(-128)))
    end)

    it("encode_signed(-129) == {0xFF, 0x7E}", function()
        assert.is_true(bytes_equal({0xFF, 0x7E}, m.encode_signed(-129)))
    end)

    -- -----------------------------------------------------------------------
    -- decode_unsigned — basic values
    -- -----------------------------------------------------------------------

    it("decode_unsigned({0x00}) == 0, 1", function()
        local v, c = m.decode_unsigned({0x00})
        assert.equals(0, v)
        assert.equals(1, c)
    end)

    it("decode_unsigned({0x7F}) == 127, 1", function()
        local v, c = m.decode_unsigned({0x7F})
        assert.equals(127, v)
        assert.equals(1, c)
    end)

    it("decode_unsigned({0x80, 0x01}) == 128, 2", function()
        local v, c = m.decode_unsigned({0x80, 0x01})
        assert.equals(128, v)
        assert.equals(2, c)
    end)

    it("decode_unsigned({0xE5, 0x8E, 0x26}) == 624485, 3", function()
        local v, c = m.decode_unsigned({0xE5, 0x8E, 0x26})
        assert.equals(624485, v)
        assert.equals(3, c)
    end)

    -- -----------------------------------------------------------------------
    -- decode_signed — basic values
    -- -----------------------------------------------------------------------

    it("decode_signed({0x00}) == 0, 1", function()
        local v, c = m.decode_signed({0x00})
        assert.equals(0, v)
        assert.equals(1, c)
    end)

    it("decode_signed({0x7F}) == -1, 1", function()
        local v, c = m.decode_signed({0x7F})
        assert.equals(-1, v)
        assert.equals(1, c)
    end)

    it("decode_signed({0x7E}) == -2, 1", function()
        local v, c = m.decode_signed({0x7E})
        assert.equals(-2, v)
        assert.equals(1, c)
    end)

    it("decode_signed({0x3F}) == 63, 1", function()
        local v, c = m.decode_signed({0x3F})
        assert.equals(63, v)
        assert.equals(1, c)
    end)

    it("decode_signed({0x40}) == -64, 1", function()
        local v, c = m.decode_signed({0x40})
        assert.equals(-64, v)
        assert.equals(1, c)
    end)

    it("decode_signed({0x80, 0x7F}) == -128, 2", function()
        local v, c = m.decode_signed({0x80, 0x7F})
        assert.equals(-128, v)
        assert.equals(2, c)
    end)

    -- -----------------------------------------------------------------------
    -- Offset parameter (1-based)
    -- -----------------------------------------------------------------------

    it("decode_unsigned with offset 2 skips first byte", function()
        -- {0x00, 0xE5, 0x8E, 0x26}: start at index 2
        local v, c = m.decode_unsigned({0x00, 0xE5, 0x8E, 0x26}, 2)
        assert.equals(624485, v)
        assert.equals(3, c)
    end)

    it("decode_signed with offset 3 reads from correct position", function()
        -- {0x01, 0x02, 0x7E}: decode signed from index 3
        local v, c = m.decode_signed({0x01, 0x02, 0x7E}, 3)
        assert.equals(-2, v)
        assert.equals(1, c)
    end)

    -- -----------------------------------------------------------------------
    -- Round-trip tests
    -- -----------------------------------------------------------------------

    it("unsigned encode/decode round-trips for various values", function()
        local values = {0, 1, 63, 64, 127, 128, 255, 300, 624485, 65535, 1000000}
        for _, v in ipairs(values) do
            local encoded = m.encode_unsigned(v)
            local decoded, _ = m.decode_unsigned(encoded)
            assert.equals(v, decoded)
        end
    end)

    it("signed encode/decode round-trips for various values", function()
        local values = {0, 1, 63, 64, 127, -1, -2, -64, -128, -129, -256, -1000}
        for _, v in ipairs(values) do
            local encoded = m.encode_signed(v)
            local decoded, _ = m.decode_signed(encoded)
            assert.equals(v, decoded)
        end
    end)

    -- -----------------------------------------------------------------------
    -- Error cases
    -- -----------------------------------------------------------------------

    it("encode_unsigned errors on negative input", function()
        assert.has_error(function() m.encode_unsigned(-1) end)
    end)

    it("encode_unsigned errors on non-number", function()
        assert.has_error(function() m.encode_unsigned("hello") end)
    end)

    it("decode_unsigned errors on unterminated sequence", function()
        -- All bytes have the continuation bit set — never terminates
        assert.has_error(function() m.decode_unsigned({0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80}) end)
    end)

    it("decode_unsigned errors on empty table", function()
        assert.has_error(function() m.decode_unsigned({}) end)
    end)

    it("decode_signed errors on unterminated sequence", function()
        assert.has_error(function() m.decode_signed({0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80}) end)
    end)

end)
