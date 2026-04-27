-- Tests for coding_adventures.aes — AES block cipher.
-- Test vectors from FIPS 197 Appendix B and C, and NIST SP 800-38A.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local aes = require("coding_adventures.aes")

local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

-- ============================================================================
-- AES-128
-- ============================================================================

describe("AES-128 (16-byte key)", function()
    it("FIPS 197 Appendix B encrypt", function()
        local key   = h("2b7e151628aed2a6abf7158809cf4f3c")
        local plain = h("3243f6a8885a308d313198a2e0370734")
        assert.are.equal(h("3925841d02dc09fbdc118597196a0b32"),
            aes.aes_encrypt_block(plain, key))
    end)

    it("FIPS 197 Appendix B decrypt", function()
        local key    = h("2b7e151628aed2a6abf7158809cf4f3c")
        local cipher = h("3925841d02dc09fbdc118597196a0b32")
        assert.are.equal(h("3243f6a8885a308d313198a2e0370734"),
            aes.aes_decrypt_block(cipher, key))
    end)

    it("FIPS 197 Appendix C.1 encrypt", function()
        local key   = h("000102030405060708090a0b0c0d0e0f")
        local plain = h("00112233445566778899aabbccddeeff")
        assert.are.equal(h("69c4e0d86a7b0430d8cdb78070b4c55a"),
            aes.aes_encrypt_block(plain, key))
    end)

    it("FIPS 197 Appendix C.1 decrypt", function()
        local key    = h("000102030405060708090a0b0c0d0e0f")
        local cipher = h("69c4e0d86a7b0430d8cdb78070b4c55a")
        assert.are.equal(h("00112233445566778899aabbccddeeff"),
            aes.aes_decrypt_block(cipher, key))
    end)

    it("round-trip multiple blocks", function()
        local key = h("2b7e151628aed2a6abf7158809cf4f3c")
        for start = 0, 240, 16 do
            local bytes = {}
            for i = start, start + 15 do bytes[#bytes+1] = string.char(i % 256) end
            local plain = table.concat(bytes)
            local ct = aes.aes_encrypt_block(plain, key)
            assert.are.equal(plain, aes.aes_decrypt_block(ct, key))
        end
    end)

    it("encrypt returns 16 bytes", function()
        local key   = h("000102030405060708090a0b0c0d0e0f")
        local plain = h("00112233445566778899aabbccddeeff")
        assert.are.equal(16, #aes.aes_encrypt_block(plain, key))
    end)
end)

-- ============================================================================
-- AES-192
-- ============================================================================

describe("AES-192 (24-byte key)", function()
    it("FIPS 197 Appendix C.2 encrypt", function()
        local key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        local plain = h("00112233445566778899aabbccddeeff")
        assert.are.equal(h("dda97ca4864cdfe06eaf70a0ec0d7191"),
            aes.aes_encrypt_block(plain, key))
    end)

    it("FIPS 197 Appendix C.2 decrypt", function()
        local key    = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        local cipher = h("dda97ca4864cdfe06eaf70a0ec0d7191")
        assert.are.equal(h("00112233445566778899aabbccddeeff"),
            aes.aes_decrypt_block(cipher, key))
    end)

    it("round-trip", function()
        local key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        local plain = h("6bc1bee22e409f96e93d7e117393172a")
        local ct = aes.aes_encrypt_block(plain, key)
        assert.are.equal(plain, aes.aes_decrypt_block(ct, key))
    end)
end)

-- ============================================================================
-- AES-256
-- ============================================================================

describe("AES-256 (32-byte key)", function()
    it("FIPS 197 Appendix C.3 encrypt", function()
        local key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        local plain = h("00112233445566778899aabbccddeeff")
        assert.are.equal(h("8ea2b7ca516745bfeafc49904b496089"),
            aes.aes_encrypt_block(plain, key))
    end)

    it("FIPS 197 Appendix C.3 decrypt", function()
        local key    = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        local cipher = h("8ea2b7ca516745bfeafc49904b496089")
        assert.are.equal(h("00112233445566778899aabbccddeeff"),
            aes.aes_decrypt_block(cipher, key))
    end)

    it("SP 800-38A AES-256 encrypt", function()
        local key   = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
        local plain = h("6bc1bee22e409f96e93d7e117393172a")
        assert.are.equal(h("f3eed1bdb5d2a03c064b5a7e3db181f8"),
            aes.aes_encrypt_block(plain, key))
    end)

    it("SP 800-38A AES-256 decrypt", function()
        local key    = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
        local cipher = h("f3eed1bdb5d2a03c064b5a7e3db181f8")
        assert.are.equal(h("6bc1bee22e409f96e93d7e117393172a"),
            aes.aes_decrypt_block(cipher, key))
    end)

    it("round-trip", function()
        local key = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        for start = 0, 240, 16 do
            local bytes = {}
            for i = start, start + 15 do bytes[#bytes+1] = string.char(i % 256) end
            local plain = table.concat(bytes)
            local ct = aes.aes_encrypt_block(plain, key)
            assert.are.equal(plain, aes.aes_decrypt_block(ct, key))
        end
    end)
end)

-- ============================================================================
-- Key Schedule
-- ============================================================================

describe("expand_key", function()
    it("AES-128 produces 11 round keys", function()
        local rks = aes.expand_key(h("2b7e151628aed2a6abf7158809cf4f3c"))
        assert.are.equal(11, #rks)
    end)

    it("AES-192 produces 13 round keys", function()
        local rks = aes.expand_key(h("000102030405060708090a0b0c0d0e0f1011121314151617"))
        assert.are.equal(13, #rks)
    end)

    it("AES-256 produces 15 round keys", function()
        local rks = aes.expand_key(h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"))
        assert.are.equal(15, #rks)
    end)

    it("raises on invalid key size (15 bytes)", function()
        assert.has_error(function()
            aes.expand_key(string.rep("\x00", 15))
        end)
    end)

    it("raises on invalid key size (33 bytes)", function()
        assert.has_error(function()
            aes.expand_key(string.rep("\x00", 33))
        end)
    end)
end)

-- ============================================================================
-- S-box Properties
-- ============================================================================

describe("S-box properties", function()
    it("SBOX has 256 entries", function()
        assert.are.equal(256, #aes.SBOX)
    end)

    it("INV_SBOX has 256 entries", function()
        assert.are.equal(256, #aes.INV_SBOX)
    end)

    it("SBOX and INV_SBOX are mutual inverses", function()
        for b = 0, 255 do
            assert.are.equal(b, aes.INV_SBOX[aes.SBOX[b+1]+1])
        end
    end)

    it("SBOX[0] == 0x63 (FIPS 197)", function()
        assert.are.equal(0x63, aes.SBOX[1])
    end)
end)

-- ============================================================================
-- Invalid Inputs
-- ============================================================================

describe("invalid inputs", function()
    it("encrypt raises on wrong block size", function()
        assert.has_error(function()
            aes.aes_encrypt_block(string.rep("\x00", 15), string.rep("\x00", 16))
        end)
    end)

    it("decrypt raises on wrong block size", function()
        assert.has_error(function()
            aes.aes_decrypt_block(string.rep("\x00", 17), string.rep("\x00", 16))
        end)
    end)

    it("encrypt raises on wrong key size", function()
        assert.has_error(function()
            aes.aes_encrypt_block(string.rep("\x00", 16), string.rep("\x00", 10))
        end)
    end)
end)
