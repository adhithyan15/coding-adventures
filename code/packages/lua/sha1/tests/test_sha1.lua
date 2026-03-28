-- Tests for sha1
--
-- Validates the pure-Lua SHA-1 implementation against NIST FIPS 180-4
-- test vectors and other well-known reference values.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.sha1")

describe("sha1", function()

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
    -- NIST / well-known test vectors
    -- -----------------------------------------------------------------------

    it('sha1("") == da39a3ee5e6b4b0d3255bfef95601890afd80709', function()
        assert.equals("da39a3ee5e6b4b0d3255bfef95601890afd80709", m.hex(""))
    end)

    it('sha1("abc") == a9993e364706816aba3e25717850c26c9cd0d89d', function()
        assert.equals("a9993e364706816aba3e25717850c26c9cd0d89d", m.hex("abc"))
    end)

    it('sha1("hello") == aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d', function()
        assert.equals("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d", m.hex("hello"))
    end)

    it('sha1("The quick brown fox...") == 2fd4e1c67a2d28fced849ee1bb76e7391b93eb12', function()
        assert.equals("2fd4e1c67a2d28fced849ee1bb76e7391b93eb12",
            m.hex("The quick brown fox jumps over the lazy dog"))
    end)

    it('sha1("The quick brown fox... cog") == de9f2c7fd25e1b3afad3e85a0bd17d9b100db4b3', function()
        -- "cog" instead of "dog" — one-bit difference ripples through the whole hash
        assert.equals("de9f2c7fd25e1b3afad3e85a0bd17d9b100db4b3",
            m.hex("The quick brown fox jumps over the lazy cog"))
    end)

    it('sha1("SHA1 test") has expected length', function()
        local h = m.hex("SHA1 test")
        assert.equals(40, #h)
    end)

    it('sha1("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")', function()
        -- NIST FIPS 180-4 example B.1
        assert.equals("84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            m.hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"))
    end)

    it("sha1 of single 'a'", function()
        assert.equals("86f7e437faa5a7fce15d1ddcb9eaeaea377667b8", m.hex("a"))
    end)

    -- -----------------------------------------------------------------------
    -- digest() function — raw byte array
    -- -----------------------------------------------------------------------

    it("digest returns 20 integers", function()
        local raw = m.digest("hello")
        assert.equals(20, #raw)
    end)

    it("digest bytes are in 0..255 range", function()
        local raw = m.digest("abc")
        for _, b in ipairs(raw) do
            assert.is_true(b >= 0 and b <= 255)
        end
    end)

    it("digest of empty string first 5 bytes match", function()
        -- da39a3ee...
        local raw = m.digest("")
        assert.equals(0xda, raw[1])
        assert.equals(0x39, raw[2])
        assert.equals(0xa3, raw[3])
        assert.equals(0xee, raw[4])
        assert.equals(0x5e, raw[5])
    end)

    -- -----------------------------------------------------------------------
    -- Multi-block message (> 64 bytes)
    -- -----------------------------------------------------------------------

    it("handles message longer than one 64-byte block", function()
        -- 72 'a' characters crosses the 64-byte block boundary
        local s = string.rep("a", 72)
        assert.equals("227c150957bf386497eb4f8eeabbaf9fe5ff5b96", m.hex(s))
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
        local msg = "sha1 consistency test"
        local raw = m.digest(msg)
        local hex = m.hex(msg)
        local rebuilt = ""
        for _, b in ipairs(raw) do
            rebuilt = rebuilt .. string.format("%02x", b)
        end
        assert.equals(hex, rebuilt)
    end)

end)
