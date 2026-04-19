-- Tests for the pure-Lua BLAKE2b implementation.
--
-- All expected values are pre-computed from Python's hashlib.blake2b and
-- mirrored across every sibling language in the monorepo.  Matching these
-- KATs proves we implement RFC 7693 correctly.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local blake2b = require("coding_adventures.blake2b")

-- Helper: produce a byte string containing bytes `start`, start+1, ..., stop-1
-- (each taken mod 256).  Mirrors the bytes_from_range helper in Ruby/Python.
local function bytes_from_range(start, stop)
    local t = {}
    for i = start, stop - 1 do
        t[#t + 1] = string.char(i & 0xff)
    end
    return table.concat(t)
end

local function hex(bytes)
    local parts = {}
    for i = 1, #bytes do
        parts[i] = string.format("%02x", string.byte(bytes, i))
    end
    return table.concat(parts)
end

describe("blake2b", function()

    -- ----------------------------------------------------------------
    -- Meta / version surface
    -- ----------------------------------------------------------------

    it("has VERSION", function()
        assert.is_not_nil(blake2b.VERSION)
        assert.equals("0.1.0", blake2b.VERSION)
    end)

    it("exposes hex and digest", function()
        assert.is_function(blake2b.hex)
        assert.is_function(blake2b.digest)
        assert.is_function(blake2b.blake2b)
        assert.is_function(blake2b.blake2b_hex)
    end)

    it("exposes Hasher", function()
        assert.is_not_nil(blake2b.Hasher)
        assert.is_function(blake2b.Hasher.new)
    end)

    -- ----------------------------------------------------------------
    -- Canonical vectors
    -- ----------------------------------------------------------------

    it("empty message default digest", function()
        assert.equals(
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419"
            .. "d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce",
            blake2b.hex("")
        )
    end)

    it("'abc' matches RFC 7693 Appendix A", function()
        assert.equals(
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
            .. "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
            blake2b.hex("abc")
        )
    end)

    it("'The quick brown fox ...'", function()
        assert.equals(
            "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673"
            .. "f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918",
            blake2b.hex("The quick brown fox jumps over the lazy dog")
        )
    end)

    it("truncated digest_size=32 of empty string", function()
        assert.equals(
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8",
            blake2b.hex("", { digest_size = 32 })
        )
    end)

    it("keyed long vector: data=0..255, key=1..64", function()
        local key = bytes_from_range(1, 65)
        local data = bytes_from_range(0, 256)
        assert.equals(
            "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927"
            .. "ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3",
            blake2b.hex(data, { key = key })
        )
    end)

    -- ----------------------------------------------------------------
    -- Block-boundary sizes (classic BLAKE2 off-by-one territory)
    -- ----------------------------------------------------------------

    local BLOCK_KATS = {
        {0,    "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"},
        {1,    "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d"},
        {63,   "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09"},
        {64,   "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7"},
        {65,   "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e"},
        {127,  "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39"},
        {128,  "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb"},
        {129,  "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b"},
        {255,  "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789"},
        {256,  "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae"},
        {257,  "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348"},
        {1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3"},
        {4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7"},
        {9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069"},
    }

    it("matches KATs at every block-boundary size", function()
        for _, kat in ipairs(BLOCK_KATS) do
            local size, want = kat[1], kat[2]
            local t = {}
            for i = 0, size - 1 do
                t[#t + 1] = string.char((i * 7 + 3) & 0xff)
            end
            local data = table.concat(t)
            assert.equals(want, blake2b.hex(data),
                          "mismatch at size " .. size)
        end
    end)

    -- ----------------------------------------------------------------
    -- Variable digest sizes
    -- ----------------------------------------------------------------

    local DIGEST_SIZE_KATS = {
        {1,  "b5"},
        {16, "249df9a49f517ddcd37f5c897620ec73"},
        {20, "3c523ed102ab45a37d54f5610d5a983162fde84f"},
        {32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9"},
        {48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d"},
        {64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"},
    }

    it("matches KATs across variable digest sizes", function()
        local data = "The quick brown fox jumps over the lazy dog"
        for _, kat in ipairs(DIGEST_SIZE_KATS) do
            local ds, want = kat[1], kat[2]
            local out = blake2b.digest(data, { digest_size = ds })
            assert.equals(ds, #out, "bytes for digest_size " .. ds)
            assert.equals(want, hex(out), "hex for digest_size " .. ds)
        end
    end)

    -- ----------------------------------------------------------------
    -- Keyed variants
    -- ----------------------------------------------------------------

    local KEYED_KATS = {
        {1,  "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422"},
        {16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618"},
        {32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67"},
        {64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb"},
    }

    it("matches KATs for keyed variants (1/16/32/64-byte keys)", function()
        local data = "secret message body"
        for _, kat in ipairs(KEYED_KATS) do
            local klen, want = kat[1], kat[2]
            local key = bytes_from_range(1, klen + 1)
            assert.equals(want,
                blake2b.hex(data, { key = key, digest_size = 32 }),
                "keyLen " .. klen)
        end
    end)

    -- ----------------------------------------------------------------
    -- Salt + personal
    -- ----------------------------------------------------------------

    it("matches KAT for salt+personal parameters", function()
        local salt = bytes_from_range(0, 16)
        local personal = bytes_from_range(16, 32)
        assert.equals(
            "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d",
            blake2b.hex("parameterized hash",
                         { salt = salt, personal = personal })
        )
    end)

    -- ----------------------------------------------------------------
    -- Streaming behavior
    -- ----------------------------------------------------------------

    it("streaming single chunk matches one-shot", function()
        local h = blake2b.Hasher.new()
        h:update("hello world")
        assert.equals(blake2b.hex("hello world"), h:hex_digest())
    end)

    it("streaming byte-by-byte matches one-shot", function()
        local data = bytes_from_range(0, 200)
        local h = blake2b.Hasher.new({ digest_size = 32 })
        for i = 1, #data do
            h:update(string.sub(data, i, i))
        end
        assert.equals(blake2b.hex(data, { digest_size = 32 }),
                      h:hex_digest())
    end)

    it("streaming across a block boundary", function()
        local data = bytes_from_range(0, 129)
        local h = blake2b.Hasher.new()
        h:update(string.sub(data, 1, 127))
        h:update(string.sub(data, 128))
        assert.equals(blake2b.hex(data), h:hex_digest())
    end)

    it("streaming exact-block-then-more (the classic off-by-one)", function()
        -- 128 bytes exactly, then 4 more.  The 128-byte boundary must NOT
        -- be flagged final prematurely.
        local t = {}
        for i = 0, 131 do t[#t + 1] = string.char(i & 0xff) end
        local data = table.concat(t)
        local h = blake2b.Hasher.new()
        h:update(string.sub(data, 1, 128))
        h:update(string.sub(data, 129, 132))
        assert.equals(blake2b.hex(data), h:hex_digest())
    end)

    it("digest is idempotent", function()
        local h = blake2b.Hasher.new()
        h:update("hello")
        assert.equals(h:hex_digest(), h:hex_digest())
    end)

    it("update after digest continues the stream", function()
        local h = blake2b.Hasher.new({ digest_size = 32 })
        h:update("hello ")
        local _ = h:digest()
        h:update("world")
        assert.equals(
            blake2b.hex("hello world", { digest_size = 32 }),
            h:hex_digest()
        )
    end)

    it("copy produces an independent hasher", function()
        local h = blake2b.Hasher.new()
        h:update("prefix ")
        local c = h:copy()
        h:update("path A")
        c:update("path B")
        assert.equals(blake2b.hex("prefix path A"), h:hex_digest())
        assert.equals(blake2b.hex("prefix path B"), c:hex_digest())
    end)

    it("hex_digest equals hex of digest()", function()
        local h = blake2b.Hasher.new({ digest_size = 32 })
        h:update("hex check")
        assert.equals(hex(h:digest()), h:hex_digest())
    end)

    -- ----------------------------------------------------------------
    -- Validation
    -- ----------------------------------------------------------------

    it("rejects digest_size 0", function()
        assert.has_error(function()
            blake2b.hex("", { digest_size = 0 })
        end)
    end)

    it("rejects digest_size 65", function()
        assert.has_error(function()
            blake2b.hex("", { digest_size = 65 })
        end)
    end)

    it("rejects non-integer digest_size", function()
        assert.has_error(function()
            blake2b.hex("", { digest_size = 1.5 })
        end)
    end)

    it("rejects key length > 64", function()
        assert.has_error(function()
            blake2b.hex("", { key = string.rep("a", 65) })
        end)
    end)

    it("rejects salt of wrong length", function()
        assert.has_error(function()
            blake2b.hex("", { salt = string.rep("a", 8) })
        end)
    end)

    it("rejects personal of wrong length", function()
        assert.has_error(function()
            blake2b.hex("", { personal = string.rep("a", 20) })
        end)
    end)

    it("accepts a 64-byte key (the maximum)", function()
        -- Should not raise.
        blake2b.hex("x", { key = string.rep("\x41", 64) })
    end)
end)
