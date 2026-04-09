-- Tests for coding_adventures.hmac
-- Uses busted test framework.

package.path = "../src/?.lua;" ..
               "../src/?/init.lua;" ..
               "../../sha256/src/?.lua;" ..
               "../../sha256/src/?/init.lua;" ..
               "../../sha512/src/?.lua;" ..
               "../../sha512/src/?/init.lua;" ..
               "../../md5/src/?.lua;" ..
               "../../md5/src/?/init.lua;" ..
               "../../sha1/src/?.lua;" ..
               "../../sha1/src/?/init.lua;" ..
               package.path

local hmac = require("coding_adventures.hmac")

-- ─── RFC 4231 — HMAC-SHA256 ───────────────────────────────────────────────────

describe("HMAC-SHA256 (RFC 4231)", function()
    it("TC1: 20-byte key, 'Hi There'", function()
        local key = string.rep("\x0b", 20)
        assert.are.equal(
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            hmac.hmac_sha256_hex(key, "Hi There")
        )
    end)

    it("TC2: 'Jefe', 'what do ya want for nothing?'", function()
        assert.are.equal(
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            hmac.hmac_sha256_hex("Jefe", "what do ya want for nothing?")
        )
    end)

    it("TC3: 20-byte key (0xaa), 50-byte data (0xdd)", function()
        local key  = string.rep("\xaa", 20)
        local data = string.rep("\xdd", 50)
        assert.are.equal(
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
            hmac.hmac_sha256_hex(key, data)
        )
    end)

    it("TC6: key longer than block size (131 bytes)", function()
        local key = string.rep("\xaa", 131)
        assert.are.equal(
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
            hmac.hmac_sha256_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
        )
    end)

    it("TC7: key and data both longer than block size", function()
        local key  = string.rep("\xaa", 131)
        local data = "This is a test using a larger than block-size key and a larger than block-size data. " ..
                     "The key needs to be hashed before being used by the HMAC algorithm."
        assert.are.equal(
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
            hmac.hmac_sha256_hex(key, data)
        )
    end)
end)

-- ─── RFC 4231 — HMAC-SHA512 ───────────────────────────────────────────────────

describe("HMAC-SHA512 (RFC 4231)", function()
    it("TC1: 20-byte key, 'Hi There'", function()
        local key = string.rep("\x0b", 20)
        assert.are.equal(
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
            hmac.hmac_sha512_hex(key, "Hi There")
        )
    end)

    it("TC2: 'Jefe', 'what do ya want for nothing?'", function()
        assert.are.equal(
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
            hmac.hmac_sha512_hex("Jefe", "what do ya want for nothing?")
        )
    end)

    it("TC6: key longer than block size (131 bytes)", function()
        local key = string.rep("\xaa", 131)
        assert.are.equal(
            "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598",
            hmac.hmac_sha512_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
        )
    end)
end)

-- ─── RFC 2202 — HMAC-MD5 ─────────────────────────────────────────────────────

describe("HMAC-MD5 (RFC 2202)", function()
    it("TC1: 16-byte key, 'Hi There'", function()
        local key = string.rep("\x0b", 16)
        assert.are.equal("9294727a3638bb1c13f48ef8158bfc9d", hmac.hmac_md5_hex(key, "Hi There"))
    end)

    it("TC2: 'Jefe', 'what do ya want for nothing?'", function()
        assert.are.equal(
            "750c783e6ab0b503eaa86e310a5db738",
            hmac.hmac_md5_hex("Jefe", "what do ya want for nothing?")
        )
    end)

    it("TC6: key longer than block size (80 bytes)", function()
        local key = string.rep("\xaa", 80)
        assert.are.equal(
            "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd",
            hmac.hmac_md5_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
        )
    end)
end)

-- ─── RFC 2202 — HMAC-SHA1 ────────────────────────────────────────────────────

describe("HMAC-SHA1 (RFC 2202)", function()
    it("TC1: 20-byte key, 'Hi There'", function()
        local key = string.rep("\x0b", 20)
        assert.are.equal(
            "b617318655057264e28bc0b6fb378c8ef146be00",
            hmac.hmac_sha1_hex(key, "Hi There")
        )
    end)

    it("TC2: 'Jefe', 'what do ya want for nothing?'", function()
        assert.are.equal(
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79",
            hmac.hmac_sha1_hex("Jefe", "what do ya want for nothing?")
        )
    end)

    it("TC6: key longer than block size (80 bytes)", function()
        local key = string.rep("\xaa", 80)
        assert.are.equal(
            "aa4ae5e15272d00e95705637ce8a3b55ed402112",
            hmac.hmac_sha1_hex(key, "Test Using Larger Than Block-Size Key - Hash Key First")
        )
    end)
end)

-- ─── Return lengths ───────────────────────────────────────────────────────────

describe("Return lengths", function()
    it("HMAC-MD5 returns 16 bytes", function()
        assert.are.equal(16, #hmac.hmac_md5("k", "m"))
    end)
    it("HMAC-SHA1 returns 20 bytes", function()
        assert.are.equal(20, #hmac.hmac_sha1("k", "m"))
    end)
    it("HMAC-SHA256 returns 32 bytes", function()
        assert.are.equal(32, #hmac.hmac_sha256("k", "m"))
    end)
    it("HMAC-SHA512 returns 64 bytes", function()
        assert.are.equal(64, #hmac.hmac_sha512("k", "m"))
    end)
end)

-- ─── Key handling ─────────────────────────────────────────────────────────────

describe("Key handling", function()
    it("empty key raises error for SHA-256", function()
        assert.has_error(function() hmac.hmac_sha256("", "") end, "HMAC key must not be empty")
    end)

    it("empty key raises error for SHA-512", function()
        assert.has_error(function() hmac.hmac_sha512("", "") end, "HMAC key must not be empty")
    end)

    it("empty message with non-empty key is allowed", function()
        assert.are.equal(32, #hmac.hmac_sha256("key", ""))
    end)

    it("keys of different long lengths produce different tags", function()
        local k65 = string.rep("\x01", 65)
        local k66 = string.rep("\x01", 66)
        assert.are_not.equal(hmac.hmac_sha256_hex(k65, "msg"), hmac.hmac_sha256_hex(k66, "msg"))
    end)
end)

-- ─── Authentication properties ────────────────────────────────────────────────

describe("Authentication properties", function()
    it("deterministic", function()
        assert.are.equal(hmac.hmac_sha256_hex("k", "m"), hmac.hmac_sha256_hex("k", "m"))
    end)

    it("key sensitivity", function()
        assert.are_not.equal(hmac.hmac_sha256_hex("k1", "m"), hmac.hmac_sha256_hex("k2", "m"))
    end)

    it("message sensitivity", function()
        assert.are_not.equal(hmac.hmac_sha256_hex("k", "m1"), hmac.hmac_sha256_hex("k", "m2"))
    end)
end)
