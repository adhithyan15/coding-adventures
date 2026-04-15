-- Tests for coding_adventures.des — DES and 3DES block cipher.
--
-- Test vectors from FIPS 46-3, SP 800-20, and NIST SP 800-67.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local des = require("coding_adventures.des")

-- Convenience: decode a hex string to a byte string
local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

-- Encode bytes to hex for assertions
local function to_hex(s)
    return (s:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

-- ============================================================================
-- DES encrypt block
-- ============================================================================

describe("des_encrypt_block", function()
    it("Stallings/FIPS 46 worked example", function()
        local key   = h("133457799BBCDFF1")
        local plain = h("0123456789ABCDEF")
        assert.are.equal(h("85E813540F0AB405"), des.des_encrypt_block(plain, key))
    end)

    it("SP 800-20 Table 1 row 0 — plaintext variable, key=0101..01", function()
        local key = h("0101010101010101")
        assert.are.equal(h("8000000000000000"), des.des_encrypt_block(h("95F8A5E5DD31D900"), key))
    end)

    it("SP 800-20 Table 1 row 1", function()
        local key = h("0101010101010101")
        assert.are.equal(h("4000000000000000"), des.des_encrypt_block(h("DD7F121CA5015619"), key))
    end)

    it("SP 800-20 Table 2 row 0 — key variable, plain=0000..00", function()
        assert.are.equal(
            h("95A8D72813DAA94D"),
            des.des_encrypt_block(h("0000000000000000"), h("8001010101010101"))
        )
    end)

    it("SP 800-20 Table 2 row 1", function()
        assert.are.equal(
            h("0EEC1487DD8C26D5"),
            des.des_encrypt_block(h("0000000000000000"), h("4001010101010101"))
        )
    end)

    it("returns 8 bytes", function()
        local ct = des.des_encrypt_block(h("0000000000000000"), h("0101010101010101"))
        assert.are.equal(8, #ct)
    end)

    it("deterministic", function()
        local key   = h("FEDCBA9876543210")
        local plain = h("0123456789ABCDEF")
        assert.are.equal(des.des_encrypt_block(plain, key), des.des_encrypt_block(plain, key))
    end)

    it("raises on wrong block size", function()
        assert.has_error(function()
            des.des_encrypt_block(h("0102030405060708FF"), h("0101010101010101"))
        end)
    end)

    it("raises on wrong key size", function()
        assert.has_error(function()
            des.des_encrypt_block(h("0102030405060708"), h("0101010101"))
        end)
    end)
end)

-- ============================================================================
-- DES decrypt block
-- ============================================================================

describe("des_decrypt_block", function()
    it("decrypts FIPS vector 1", function()
        local key    = h("133457799BBCDFF1")
        local cipher = h("85E813540F0AB405")
        assert.are.equal(h("0123456789ABCDEF"), des.des_decrypt_block(cipher, key))
    end)

    it("round-trip with multiple keys", function()
        local plain = h("0123456789ABCDEF")
        local keys = {
            h("133457799BBCDFF0"),
            h("FFFFFFFFFFFFFFFF"),
            h("0000000000000000"),
            h("FEDCBA9876543210"),
        }
        for _, key in ipairs(keys) do
            local ct = des.des_encrypt_block(plain, key)
            assert.are.equal(plain, des.des_decrypt_block(ct, key))
        end
    end)
end)

-- ============================================================================
-- expand_key
-- ============================================================================

describe("expand_key", function()
    it("returns 16 subkeys", function()
        local subkeys = des.expand_key(h("0133457799BBCDFF"))
        assert.are.equal(16, #subkeys)
    end)

    it("each subkey is 6 bytes", function()
        local subkeys = des.expand_key(h("0133457799BBCDFF"))
        for _, sk in ipairs(subkeys) do
            assert.are.equal(6, #sk)
        end
    end)

    it("different keys → different subkeys", function()
        local sk1 = des.expand_key(h("0133457799BBCDFF"))
        local sk2 = des.expand_key(h("FEDCBA9876543210"))
        assert.are_not.equal(sk1[1], sk2[1])
    end)

    it("raises on wrong key size (7 bytes)", function()
        assert.has_error(function() des.expand_key(h("01020304050607")) end)
    end)

    it("raises on wrong key size (9 bytes)", function()
        assert.has_error(function() des.expand_key(h("010203040506070809")) end)
    end)
end)

-- ============================================================================
-- ECB mode
-- ============================================================================

describe("des_ecb_encrypt and des_ecb_decrypt", function()
    local KEY = h("0133457799BBCDFF")

    it("8-byte input → 16 bytes ciphertext", function()
        local ct = des.des_ecb_encrypt(h("0123456789ABCDEF"), KEY)
        assert.are.equal(16, #ct)
    end)

    it("sub-block input → 8 bytes ciphertext", function()
        local ct = des.des_ecb_encrypt("hello", KEY)
        assert.are.equal(8, #ct)
    end)

    it("16-byte input → 24 bytes ciphertext", function()
        local plain = string.rep("\x00", 16)
        local ct = des.des_ecb_encrypt(plain, KEY)
        assert.are.equal(24, #ct)
    end)

    it("empty input → 8 bytes (full padding block)", function()
        local ct = des.des_ecb_encrypt("", KEY)
        assert.are.equal(8, #ct)
    end)

    it("round-trip short message", function()
        local plain = "hello"
        assert.are.equal(plain, des.des_ecb_decrypt(des.des_ecb_encrypt(plain, KEY), KEY))
    end)

    it("round-trip exact block", function()
        local plain = "ABCDEFGH"
        assert.are.equal(plain, des.des_ecb_decrypt(des.des_ecb_encrypt(plain, KEY), KEY))
    end)

    it("round-trip multi-block", function()
        local plain = "The quick brown fox jumps"
        assert.are.equal(plain, des.des_ecb_decrypt(des.des_ecb_encrypt(plain, KEY), KEY))
    end)

    it("round-trip empty string", function()
        assert.are.equal("", des.des_ecb_decrypt(des.des_ecb_encrypt("", KEY), KEY))
    end)

    it("decryption raises on non-multiple of 8", function()
        assert.has_error(function()
            des.des_ecb_decrypt("1234567", KEY)
        end)
    end)

    it("decryption raises on empty ciphertext", function()
        assert.has_error(function()
            des.des_ecb_decrypt("", KEY)
        end)
    end)
end)

-- ============================================================================
-- 3DES / TDEA
-- ============================================================================

describe("tdea_encrypt_block and tdea_decrypt_block", function()
    local K1 = h("0123456789ABCDEF")
    local K2 = h("23456789ABCDEF01")
    local K3 = h("456789ABCDEF0123")
    local PLAIN  = h("6BC1BEE22E409F96")
    local CIPHER = h("3B6423D418DEFC23")

    it("TDEA encrypt — NIST SP 800-67 EDE vector", function()
        assert.are.equal(CIPHER, des.tdea_encrypt_block(PLAIN, K1, K2, K3))
    end)

    it("TDEA decrypt", function()
        assert.are.equal(PLAIN, des.tdea_decrypt_block(CIPHER, K1, K2, K3))
    end)

    it("TDEA round-trip with random keys", function()
        local k1 = h("FEDCBA9876543210")
        local k2 = h("0F1E2D3C4B5A6978")
        local k3 = h("7869584A3B2C1D0E")
        local plain = h("0123456789ABCDEF")
        local ct = des.tdea_encrypt_block(plain, k1, k2, k3)
        assert.are.equal(plain, des.tdea_decrypt_block(ct, k1, k2, k3))
    end)

    it("K1=K2=K3 reduces to single DES (backward compat)", function()
        local key   = h("0133457799BBCDFF")
        local plain = h("0123456789ABCDEF")
        assert.are.equal(
            des.des_encrypt_block(plain, key),
            des.tdea_encrypt_block(plain, key, key, key)
        )
    end)

    it("K1=K2=K3 decrypt reduces to single DES decrypt", function()
        local key = h("FEDCBA9876543210")
        local ct  = h("0123456789ABCDEF")
        assert.are.equal(
            des.des_decrypt_block(ct, key),
            des.tdea_decrypt_block(ct, key, key, key)
        )
    end)

    it("TDEA round-trip all-same-byte blocks", function()
        local k1 = h("1234567890ABCDEF")
        local k2 = h("FEDCBA0987654321")
        local k3 = h("0F0F0F0F0F0F0F0F")
        for _, val in ipairs({0x00, 0xFF, 0xA5, 0x5A}) do
            local plain = string.rep(string.char(val), 8)
            local ct = des.tdea_encrypt_block(plain, k1, k2, k3)
            assert.are.equal(plain, des.tdea_decrypt_block(ct, k1, k2, k3))
        end
    end)
end)
