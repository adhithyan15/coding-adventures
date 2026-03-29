-- Tests for md5
--
-- Validates the pure-Lua MD5 implementation against well-known test vectors
-- from RFC 1321 and other trusted sources.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.md5")

describe("md5", function()

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("exposes hex function", function()
        assert.is_function(m.hex)
    end)

    it("exposes digest function", function()
        assert.is_function(m.digest)
    end)

    -- -----------------------------------------------------------------------
    -- RFC 1321 test vectors (Section A.5)
    -- These are the canonical test cases from the original MD5 RFC.
    -- -----------------------------------------------------------------------

    it('md5("") == d41d8cd98f00b204e9800998ecf8427e', function()
        assert.equals("d41d8cd98f00b204e9800998ecf8427e", m.hex(""))
    end)

    it('md5("a") == 0cc175b9c0f1b6a831c399e269772661', function()
        assert.equals("0cc175b9c0f1b6a831c399e269772661", m.hex("a"))
    end)

    it('md5("abc") == 900150983cd24fb0d6963f7d28e17f72', function()
        assert.equals("900150983cd24fb0d6963f7d28e17f72", m.hex("abc"))
    end)

    it('md5("message digest") == f96b697d7cb7938d525a2f31aaf161d0', function()
        assert.equals("f96b697d7cb7938d525a2f31aaf161d0", m.hex("message digest"))
    end)

    it('md5("hello") == 5d41402abc4b2a76b9719d911017c592', function()
        assert.equals("5d41402abc4b2a76b9719d911017c592", m.hex("hello"))
    end)

    it('md5("abcdefghijklmnopqrstuvwxyz") == c3fcd3d76192e4007dfb496cca67e13b', function()
        assert.equals("c3fcd3d76192e4007dfb496cca67e13b",
            m.hex("abcdefghijklmnopqrstuvwxyz"))
    end)

    it('md5(alphabet+digits) == d174ab98d277d9f5a5611c2c9f419d9f', function()
        assert.equals("d174ab98d277d9f5a5611c2c9f419d9f",
            m.hex("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"))
    end)

    it('md5("The quick brown fox...") == 9e107d9d372bb6826bd81d3542a419d6', function()
        assert.equals("9e107d9d372bb6826bd81d3542a419d6",
            m.hex("The quick brown fox jumps over the lazy dog"))
    end)

    it('md5("The quick brown fox...") with trailing dot', function()
        assert.equals("e4d909c290d0fb1ca068ffaddf22cbd0",
            m.hex("The quick brown fox jumps over the lazy dog."))
    end)

    -- -----------------------------------------------------------------------
    -- digest() function — raw byte array output
    -- -----------------------------------------------------------------------

    it("digest returns 16 integers", function()
        local raw = m.digest("hello")
        assert.equals(16, #raw)
    end)

    it("digest bytes are in 0..255 range", function()
        local raw = m.digest("abc")
        for _, b in ipairs(raw) do
            assert.is_true(b >= 0 and b <= 255)
        end
    end)

    it("digest of empty string first 4 bytes are correct", function()
        -- d41d8cd9...
        local raw = m.digest("")
        assert.equals(0xd4, raw[1])
        assert.equals(0x1d, raw[2])
        assert.equals(0x8c, raw[3])
        assert.equals(0xd9, raw[4])
    end)

    -- -----------------------------------------------------------------------
    -- Long message (crosses multiple 512-bit blocks)
    -- 80 'a' characters = 80 bytes > 64 bytes (forces 2-block processing)
    -- -----------------------------------------------------------------------

    it("handles message longer than one 64-byte block", function()
        -- 80 'a' characters forces two-block (2×64-byte) processing
        local s = string.rep("a", 80)
        assert.equals("b15af9cdabbaea0516866a33d8fd0f98", m.hex(s))
    end)

    -- -----------------------------------------------------------------------
    -- Error handling
    -- -----------------------------------------------------------------------

    it("errors when passed a non-string to hex()", function()
        assert.has_error(function() m.hex(42) end)
    end)

    it("errors when passed a non-string to digest()", function()
        assert.has_error(function() m.digest(nil) end)
    end)

    -- -----------------------------------------------------------------------
    -- Consistency: hex and digest agree
    -- -----------------------------------------------------------------------

    it("hex and digest are consistent", function()
        local msg = "consistency check"
        local raw = m.digest(msg)
        local hex = m.hex(msg)
        local rebuilt = ""
        for _, b in ipairs(raw) do
            rebuilt = rebuilt .. string.format("%02x", b)
        end
        assert.equals(hex, rebuilt)
    end)

end)
