-- Tests for pure-Lua Argon2d (RFC 9106).

package.path = "../src/?.lua;../src/?/init.lua;"
            .. "../../blake2b/src/?.lua;../../blake2b/src/?/init.lua;"
            .. package.path

local argon2d = require("coding_adventures.argon2d")

local function rep_byte(n, byte)
    return string.rep(string.char(byte), n)
end

local RFC_PW = rep_byte(32, 0x01)
local RFC_SALT = rep_byte(16, 0x02)
local RFC_KEY = rep_byte(8, 0x03)
local RFC_AD = rep_byte(12, 0x04)
local RFC_EXPECTED = "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb"

describe("argon2d", function()
    it("RFC 9106 §5.1 gold-standard vector", function()
        local hex = argon2d.argon2d_hex(RFC_PW, RFC_SALT, 3, 32, 4, 32,
            {key = RFC_KEY, associated_data = RFC_AD})
        assert.equals(RFC_EXPECTED, hex)
    end)

    it("hex matches binary", function()
        local tag = argon2d.argon2d(RFC_PW, RFC_SALT, 3, 32, 4, 32,
            {key = RFC_KEY, associated_data = RFC_AD})
        local parts = {}
        for i = 1, #tag do parts[i] = string.format("%02x", string.byte(tag, i)) end
        assert.equals(RFC_EXPECTED, table.concat(parts))
    end)

    it("rejects short salt", function()
        assert.has_error(function()
            argon2d.argon2d("pw", "short", 1, 8, 1, 32)
        end)
    end)

    it("rejects zero time_cost", function()
        assert.has_error(function()
            argon2d.argon2d("pw", string.rep("a", 8), 0, 8, 1, 32)
        end)
    end)

    it("rejects tag_length under 4", function()
        assert.has_error(function()
            argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 3)
        end)
    end)

    it("rejects memory below floor", function()
        assert.has_error(function()
            argon2d.argon2d("pw", string.rep("a", 8), 1, 7, 1, 32)
        end)
    end)

    it("rejects zero parallelism", function()
        assert.has_error(function()
            argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 0, 32)
        end)
    end)

    it("rejects unsupported version", function()
        assert.has_error(function()
            argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 32, {version = 0x10})
        end)
    end)

    it("deterministic", function()
        local a = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        assert.equals(a, b)
    end)

    it("differs on password", function()
        local a = argon2d.argon2d_hex("pw1", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw2", string.rep("a", 8), 1, 8, 1, 32)
        assert.not_equals(a, b)
    end)

    it("differs on salt", function()
        local a = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw", string.rep("b", 8), 1, 8, 1, 32)
        assert.not_equals(a, b)
    end)

    it("key binds", function()
        local a = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32, {key = "k1"})
        local c = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32, {key = "k2"})
        assert.not_equals(a, b)
        assert.not_equals(b, c)
    end)

    it("associated_data binds", function()
        local a = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32, {associated_data = "x"})
        local c = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32, {associated_data = "y"})
        assert.not_equals(a, b)
        assert.not_equals(b, c)
    end)

    it("tag_length 4", function()
        assert.equals(4, #argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 4))
    end)

    it("tag_length 16", function()
        assert.equals(16, #argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 16))
    end)

    it("tag_length 65 crosses H' boundary", function()
        assert.equals(65, #argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 65))
    end)

    it("tag_length 128", function()
        assert.equals(128, #argon2d.argon2d("pw", string.rep("a", 8), 1, 8, 1, 128))
    end)

    it("multi-lane", function()
        assert.equals(32, #argon2d.argon2d("pw", string.rep("a", 8), 1, 16, 2, 32))
    end)

    it("multi-pass", function()
        local a = argon2d.argon2d_hex("pw", string.rep("a", 8), 1, 8, 1, 32)
        local b = argon2d.argon2d_hex("pw", string.rep("a", 8), 2, 8, 1, 32)
        assert.not_equals(a, b)
    end)
end)
