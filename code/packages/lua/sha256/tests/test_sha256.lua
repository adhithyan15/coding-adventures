-- Tests for sha256
--
-- Validates the pure-Lua SHA-256 implementation against NIST FIPS 180-4
-- test vectors, boundary conditions, and the streaming hasher API.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local m = require("coding_adventures.sha256")

describe("sha256", function()

    -- -----------------------------------------------------------------------
    -- Meta / version
    -- -----------------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)

    it("exposes sha256 function", function()
        assert.is_function(m.sha256)
    end)

    it("exposes sha256_hex function", function()
        assert.is_function(m.sha256_hex)
    end)

    it("exposes new function for streaming", function()
        assert.is_function(m.new)
    end)

    -- -----------------------------------------------------------------------
    -- NIST FIPS 180-4 test vectors
    -- -----------------------------------------------------------------------

    it('sha256("") — empty string', function()
        assert.equals(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            m.sha256_hex("")
        )
    end)

    it('sha256("abc") — FIPS example B.1', function()
        assert.equals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            m.sha256_hex("abc")
        )
    end)

    it('sha256(448-bit test vector)', function()
        assert.equals(
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            m.sha256_hex("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")
        )
    end)

    -- -----------------------------------------------------------------------
    -- Additional known vectors
    -- -----------------------------------------------------------------------

    it('sha256("hello")', function()
        assert.equals(
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            m.sha256_hex("hello")
        )
    end)

    it('sha256("a")', function()
        assert.equals(
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
            m.sha256_hex("a")
        )
    end)

    it('sha256(pangram)', function()
        assert.equals(
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
            m.sha256_hex("The quick brown fox jumps over the lazy dog")
        )
    end)

    -- -----------------------------------------------------------------------
    -- Edge cases: block boundaries
    -- -----------------------------------------------------------------------
    -- SHA-256 processes 64-byte blocks. Padding adds 1 + zeros + 8 bytes.
    -- The boundary cases at 55, 56, 64 bytes are critical to test because
    -- they determine whether padding fits in the current block or overflows
    -- into an additional block.

    it("55 bytes (padding fits in one block)", function()
        local s = string.rep("a", 55)
        local h = m.sha256_hex(s)
        assert.equals(64, #h)
        assert.equals("9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318", h)
    end)

    it("56 bytes (padding overflows to second block)", function()
        local s = string.rep("a", 56)
        local h = m.sha256_hex(s)
        assert.equals(64, #h)
        assert.equals("b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a", h)
    end)

    it("64 bytes (exact one block before padding)", function()
        local s = string.rep("a", 64)
        local h = m.sha256_hex(s)
        assert.equals(64, #h)
        assert.equals("ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb", h)
    end)

    it("handles message longer than one block", function()
        local s = string.rep("a", 100)
        local h = m.sha256_hex(s)
        assert.equals(64, #h)
    end)

    -- -----------------------------------------------------------------------
    -- Raw digest (byte array)
    -- -----------------------------------------------------------------------

    it("sha256 returns 32 integers", function()
        local raw = m.sha256("abc")
        assert.equals(32, #raw)
    end)

    it("all bytes in range 0..255", function()
        local raw = m.sha256("test")
        for _, b in ipairs(raw) do
            assert.is_true(b >= 0 and b <= 255)
        end
    end)

    it("first bytes of sha256('abc') match known value", function()
        local raw = m.sha256("abc")
        -- ba7816bf...
        assert.equals(0xba, raw[1])
        assert.equals(0x78, raw[2])
        assert.equals(0x16, raw[3])
        assert.equals(0xbf, raw[4])
    end)

    -- -----------------------------------------------------------------------
    -- Consistency: sha256_hex and sha256 agree
    -- -----------------------------------------------------------------------

    it("sha256_hex and sha256 produce consistent results", function()
        local msg = "consistency test"
        local raw = m.sha256(msg)
        local hex = m.sha256_hex(msg)
        local rebuilt = ""
        for _, b in ipairs(raw) do
            rebuilt = rebuilt .. string.format("%02x", b)
        end
        assert.equals(hex, rebuilt)
    end)

    -- -----------------------------------------------------------------------
    -- Determinism
    -- -----------------------------------------------------------------------

    it("is deterministic", function()
        local h1 = m.sha256_hex("determinism")
        local h2 = m.sha256_hex("determinism")
        assert.equals(h1, h2)
    end)

    -- -----------------------------------------------------------------------
    -- Output format
    -- -----------------------------------------------------------------------

    it("hex output is always 64 lowercase hex characters", function()
        for _, s in ipairs({"", "a", "abc", string.rep("x", 55),
                            string.rep("x", 56), string.rep("x", 64),
                            string.rep("x", 100)}) do
            local h = m.sha256_hex(s)
            assert.equals(64, #h)
            assert.truthy(h:match("^[0-9a-f]+$"))
        end
    end)

    -- -----------------------------------------------------------------------
    -- Avalanche
    -- -----------------------------------------------------------------------

    it("one character difference changes the hash", function()
        local h1 = m.sha256_hex("a")
        local h2 = m.sha256_hex("b")
        assert.are_not.equals(h1, h2)
    end)

    -- -----------------------------------------------------------------------
    -- Error handling
    -- -----------------------------------------------------------------------

    it("errors when passed non-string to sha256()", function()
        assert.has_error(function() m.sha256(42) end)
    end)

    it("errors when passed non-string to sha256_hex()", function()
        assert.has_error(function() m.sha256_hex(nil) end)
    end)

    -- -----------------------------------------------------------------------
    -- Streaming hasher
    -- -----------------------------------------------------------------------

    describe("streaming hasher", function()

        it("single update equals one-shot", function()
            local h = m.new()
            h:update("abc")
            assert.equals(m.sha256_hex("abc"), h:hex_digest())
        end)

        it("split at byte boundary", function()
            local h = m.new()
            h:update("ab")
            h:update("c")
            assert.equals(
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                h:hex_digest()
            )
        end)

        it("split at block boundary", function()
            local full = string.rep("a", 128)
            local h = m.new()
            h:update(string.rep("a", 64))
            h:update(string.rep("a", 64))
            assert.equals(m.sha256_hex(full), h:hex_digest())
        end)

        it("many tiny updates", function()
            local msg = "abcdefghijklmnopqrstuvwxyz"
            local h = m.new()
            for i = 1, #msg do
                h:update(msg:sub(i, i))
            end
            assert.equals(m.sha256_hex(msg), h:hex_digest())
        end)

        it("digest is non-destructive", function()
            local h = m.new()
            h:update("abc")
            local d1 = h:hex_digest()
            local d2 = h:hex_digest()
            assert.equals(d1, d2)
        end)

        it("can continue after digest", function()
            local h = m.new()
            h:update("abc")
            local _ = h:hex_digest()
            h:update("def")
            assert.equals(m.sha256_hex("abcdef"), h:hex_digest())
        end)

        it("empty streaming matches empty one-shot", function()
            local h = m.new()
            assert.equals(m.sha256_hex(""), h:hex_digest())
        end)

        it("copy is independent", function()
            local original = m.new()
            original:update("abc")
            local copied = original:copy()
            copied:update("def")

            assert.equals(m.sha256_hex("abc"), original:hex_digest())
            assert.equals(m.sha256_hex("abcdef"), copied:hex_digest())
        end)

        it("digest returns 32 bytes", function()
            local h = m.new()
            h:update("test")
            local raw = h:digest()
            assert.equals(32, #raw)
        end)

        it("update is chainable", function()
            local h = m.new()
            local ret = h:update("abc")
            assert.equals(h, ret)
        end)

        it("large streaming input matches one-shot", function()
            local data = string.rep("X", 1000)
            local h = m.new()
            for i = 1, 10 do
                h:update(string.rep("X", 100))
            end
            assert.equals(m.sha256_hex(data), h:hex_digest())
        end)

    end)

end)
