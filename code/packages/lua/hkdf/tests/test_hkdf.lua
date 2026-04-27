-- ============================================================================
-- Tests for coding_adventures.hkdf
-- ============================================================================
--
-- Test vectors from RFC 5869 (Sections A.1, A.2, A.3).
--
-- These three test cases exercise the core HKDF functionality:
--   TC1: Basic SHA-256 with all parameters
--   TC2: SHA-256 with longer inputs (80-byte IKM, salt, info)
--   TC3: SHA-256 with empty salt and empty info
-- ============================================================================

-- Set up package paths so we can find HKDF and all its transitive deps.
-- HKDF depends on HMAC, which depends on SHA-256, SHA-512, MD5, and SHA-1.
-- Each package lives in ../../<name>/src/ relative to this test directory.
package.path = "../src/?.lua;" ..
               "../src/?/init.lua;" ..
               "../../hmac/src/?.lua;" ..
               "../../hmac/src/?/init.lua;" ..
               "../../sha256/src/?.lua;" ..
               "../../sha256/src/?/init.lua;" ..
               "../../sha512/src/?.lua;" ..
               "../../sha512/src/?/init.lua;" ..
               "../../md5/src/?.lua;" ..
               "../../md5/src/?/init.lua;" ..
               "../../sha1/src/?.lua;" ..
               "../../sha1/src/?/init.lua;" ..
               package.path

local hkdf = require("coding_adventures.hkdf")

-- ─── Helpers ──────────────────────────────────────────────────────────────────

--- Decode a hex string into a binary string.
-- "0b0b0b" becomes "\x0b\x0b\x0b" (3 bytes).
local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

--- Encode a binary string as a lowercase hex string.
local function to_hex(s)
    local parts = {}
    for i = 1, #s do
        parts[i] = string.format("%02x", string.byte(s, i))
    end
    return table.concat(parts)
end

-- ============================================================================
-- RFC 5869 Test Vectors — HKDF-SHA256
-- ============================================================================

describe("HKDF-SHA256 — RFC 5869 test vectors", function()

    -- Test Case 1: Basic extraction and expansion with SHA-256.
    -- All three parameters (salt, IKM, info) are non-empty.
    -- Output length (42) is not a multiple of HashLen (32), testing truncation.
    describe("Test Case 1: basic SHA-256", function()
        local ikm  = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        local salt = h("000102030405060708090a0b0c")
        local info = h("f0f1f2f3f4f5f6f7f8f9")
        local expected_prk = "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
        local expected_okm = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

        it("extract produces correct PRK", function()
            local prk = hkdf.extract(salt, ikm, "sha256")
            assert.are.equal(expected_prk, to_hex(prk))
        end)

        it("expand produces correct OKM", function()
            local prk = h(expected_prk)
            local okm = hkdf.expand(prk, info, 42, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)

        it("combined hkdf produces correct OKM", function()
            local okm = hkdf.hkdf(salt, ikm, info, 42, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)

        it("hex convenience functions work", function()
            assert.are.equal(expected_prk, hkdf.extract_hex(salt, ikm, "sha256"))
            assert.are.equal(expected_okm, hkdf.hkdf_hex(salt, ikm, info, 42, "sha256"))
        end)
    end)

    -- Test Case 2: Longer inputs — 80-byte IKM, salt, and info.
    -- Tests that HKDF handles larger payloads correctly, and that
    -- expansion works when L > 2*HashLen (requires 3 HMAC iterations).
    describe("Test Case 2: longer inputs", function()
        local ikm  = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f")
        local salt = h("606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf")
        local info = h("b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
        local expected_prk = "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"
        local expected_okm = "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"

        it("extract produces correct PRK", function()
            local prk = hkdf.extract(salt, ikm, "sha256")
            assert.are.equal(expected_prk, to_hex(prk))
        end)

        it("expand produces correct OKM", function()
            local prk = h(expected_prk)
            local okm = hkdf.expand(prk, info, 82, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)

        it("combined hkdf produces correct OKM", function()
            local okm = hkdf.hkdf(salt, ikm, info, 82, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)
    end)

    -- Test Case 3: Empty salt and empty info.
    -- When salt is empty, HKDF uses HashLen (32) zero bytes as the HMAC key.
    -- When info is empty, the expand loop uses T(i-1) || 0x0i with no info.
    describe("Test Case 3: empty salt and info", function()
        local ikm  = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        local salt = ""
        local info = ""
        local expected_prk = "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
        local expected_okm = "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"

        it("extract produces correct PRK", function()
            local prk = hkdf.extract(salt, ikm, "sha256")
            assert.are.equal(expected_prk, to_hex(prk))
        end)

        it("expand produces correct OKM", function()
            local prk = h(expected_prk)
            local okm = hkdf.expand(prk, info, 42, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)

        it("combined hkdf produces correct OKM", function()
            local okm = hkdf.hkdf(salt, ikm, info, 42, "sha256")
            assert.are.equal(expected_okm, to_hex(okm))
        end)
    end)
end)

-- ============================================================================
-- Edge Cases
-- ============================================================================

describe("HKDF edge cases", function()

    it("default hash is sha256", function()
        -- When hash parameter is omitted, should use SHA-256.
        local ikm  = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        local salt = h("000102030405060708090a0b0c")
        local info = h("f0f1f2f3f4f5f6f7f8f9")
        local expected_okm = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

        -- Omit the hash parameter entirely
        local okm = hkdf.hkdf(salt, ikm, info, 42)
        assert.are.equal(expected_okm, to_hex(okm))
    end)

    it("expand rejects length <= 0", function()
        local prk = string.rep("\x01", 32)
        assert.has_error(function()
            hkdf.expand(prk, "", 0, "sha256")
        end)
    end)

    it("expand rejects length > 255 * HashLen", function()
        local prk = string.rep("\x01", 32)
        -- For SHA-256: max = 255 * 32 = 8160
        assert.has_error(function()
            hkdf.expand(prk, "", 8161, "sha256")
        end)
    end)

    it("expand allows length = 255 * HashLen exactly", function()
        -- This should NOT error — 255 * 32 = 8160 is the maximum.
        local prk = string.rep("\x01", 32)
        local okm = hkdf.expand(prk, "", 8160, "sha256")
        assert.are.equal(8160, #okm)
    end)

    it("expand with length = 1 returns a single byte", function()
        local prk = string.rep("\x01", 32)
        local okm = hkdf.expand(prk, "", 1, "sha256")
        assert.are.equal(1, #okm)
    end)

    it("expand with length = HashLen returns exactly one HMAC block", function()
        local prk = string.rep("\x01", 32)
        local okm = hkdf.expand(prk, "test", 32, "sha256")
        assert.are.equal(32, #okm)
    end)

    it("rejects unsupported hash algorithm", function()
        assert.has_error(function()
            hkdf.extract("salt", "ikm", "md5")
        end)
    end)

    it("SHA-512 extract and expand work", function()
        -- Use TC1 IKM with SHA-512 and verify we get a 64-byte PRK.
        local ikm  = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        local salt = h("000102030405060708090a0b0c")
        local prk = hkdf.extract(salt, ikm, "sha512")
        assert.are.equal(64, #prk)

        local okm = hkdf.expand(prk, "info", 64, "sha512")
        assert.are.equal(64, #okm)
    end)

    it("nil salt treated as empty (uses HashLen zeros)", function()
        local ikm = h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
        local expected_prk = "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"

        local prk = hkdf.extract(nil, ikm, "sha256")
        assert.are.equal(expected_prk, to_hex(prk))
    end)
end)
