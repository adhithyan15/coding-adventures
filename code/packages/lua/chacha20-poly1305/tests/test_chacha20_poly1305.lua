-- ============================================================================
-- Tests for coding_adventures.chacha20_poly1305
-- ============================================================================
--
-- Test vectors from RFC 8439 (Sections 2.4.2, 2.5.2, and 2.8.2).
-- ============================================================================

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local cc = require("coding_adventures.chacha20_poly1305")

--- Decode a hex string into a byte string.
local function h(hex)
    return (hex:gsub("..", function(x) return string.char(tonumber(x, 16)) end))
end

--- Encode a byte string as a lowercase hex string.
local function to_hex(s)
    local parts = {}
    for i = 1, #s do
        parts[i] = string.format("%02x", string.byte(s, i))
    end
    return table.concat(parts)
end

-- ============================================================================
-- ChaCha20 — RFC 8439 Section 2.4.2
-- ============================================================================

describe("ChaCha20 stream cipher", function()
    it("RFC 8439 Section 2.4.2 — Sunscreen test vector", function()
        -- This is the canonical ChaCha20 test vector from the RFC.
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000004a00000000")
        local counter = 1

        local plaintext =
            "Ladies and Gentlemen of the class of '99: " ..
            "If I could offer you only one tip for the future, " ..
            "sunscreen would be it."

        local expected_ct = h(
            "6e2e359a2568f98041ba0728dd0d6981" ..
            "e97e7aec1d4360c20a27afccfd9fae0b" ..
            "f91b65c5524733ab8f593dabcd62b357" ..
            "1639d624e65152ab8f530c359f0861d8" ..
            "07ca0dbf500d6a6156a38e088a22b65e" ..
            "52bc514d16ccf806818ce91ab7793736" ..
            "5af90bbf74a35be6b40b8eedf2785e42" ..
            "874d"
        )

        local ct = cc.chacha20_encrypt(plaintext, key, nonce, counter)
        assert.are.equal(to_hex(expected_ct), to_hex(ct))
    end)

    it("encrypt then decrypt (round-trip)", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000004a00000000")
        local plaintext = "Hello, ChaCha20!"

        local ct = cc.chacha20_encrypt(plaintext, key, nonce, 0)
        local recovered = cc.chacha20_encrypt(ct, key, nonce, 0)
        assert.are.equal(plaintext, recovered)
    end)

    it("empty plaintext", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local ct = cc.chacha20_encrypt("", key, nonce, 0)
        assert.are.equal("", ct)
    end)

    it("single byte", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local ct = cc.chacha20_encrypt("X", key, nonce, 0)
        assert.are.equal(1, #ct)
        -- Round-trip
        local pt = cc.chacha20_encrypt(ct, key, nonce, 0)
        assert.are.equal("X", pt)
    end)

    it("multi-block plaintext (> 64 bytes)", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local plaintext = string.rep("A", 200)
        local ct = cc.chacha20_encrypt(plaintext, key, nonce, 0)
        assert.are.equal(200, #ct)
        local recovered = cc.chacha20_encrypt(ct, key, nonce, 0)
        assert.are.equal(plaintext, recovered)
    end)

    it("rejects invalid key length", function()
        assert.has_error(function()
            cc.chacha20_encrypt("test", "short", h("000000000000000000000000"), 0)
        end)
    end)

    it("rejects invalid nonce length", function()
        assert.has_error(function()
            cc.chacha20_encrypt("test",
                h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                "short", 0)
        end)
    end)
end)

-- ============================================================================
-- Poly1305 — RFC 8439 Section 2.5.2
-- ============================================================================

describe("Poly1305 MAC", function()
    it("RFC 8439 Section 2.5.2 — CFRG test vector", function()
        local key = h(
            "85d6be7857556d337f4452fe42d506a8" ..
            "0103808afb0db2fd4abff6af4149f51b"
        )
        local message = "Cryptographic Forum Research Group"
        local expected_tag = h("a8061dc1305136c6c22b8baf0c0127a9")

        local tag = cc.poly1305_mac(message, key)
        assert.are.equal(to_hex(expected_tag), to_hex(tag))
    end)

    it("empty message", function()
        -- With an empty message, the accumulator is never updated.
        -- tag = (0 + s) mod 2^128 = s
        local key = h(
            "00000000000000000000000000000000" ..
            "01020304050607080910111213141516"
        )
        local tag = cc.poly1305_mac("", key)
        assert.are.equal(16, #tag)
        -- tag should equal s (the second 16 bytes of the key)
        assert.are.equal(to_hex(h("01020304050607080910111213141516")), to_hex(tag))
    end)

    it("rejects invalid key length", function()
        assert.has_error(function()
            cc.poly1305_mac("test", "short")
        end)
    end)

    it("single byte message", function()
        local key = h(
            "85d6be7857556d337f4452fe42d506a8" ..
            "0103808afb0db2fd4abff6af4149f51b"
        )
        local tag = cc.poly1305_mac("A", key)
        assert.are.equal(16, #tag)
    end)

    it("exactly 16-byte message (one full block)", function()
        local key = h(
            "85d6be7857556d337f4452fe42d506a8" ..
            "0103808afb0db2fd4abff6af4149f51b"
        )
        local tag = cc.poly1305_mac("0123456789abcdef", key)
        assert.are.equal(16, #tag)
    end)

    it("17-byte message (two blocks: 16 + 1)", function()
        local key = h(
            "85d6be7857556d337f4452fe42d506a8" ..
            "0103808afb0db2fd4abff6af4149f51b"
        )
        local tag = cc.poly1305_mac("0123456789abcdefg", key)
        assert.are.equal(16, #tag)
    end)
end)

-- ============================================================================
-- AEAD — RFC 8439 Section 2.8.2
-- ============================================================================

describe("AEAD ChaCha20-Poly1305", function()
    it("RFC 8439 Section 2.8.2 — encryption", function()
        local key = h(
            "808182838485868788898a8b8c8d8e8f" ..
            "909192939495969798999a9b9c9d9e9f"
        )
        local nonce = h("070000004041424344454647")
        local aad = h("50515253c0c1c2c3c4c5c6c7")
        local plaintext =
            "Ladies and Gentlemen of the class of '99: " ..
            "If I could offer you only one tip for the future, " ..
            "sunscreen would be it."

        local expected_ct = h(
            "d31a8d34648e60db7b86afbc53ef7ec2" ..
            "a4aded51296e08fea9e2b5a736ee62d6" ..
            "3dbea45e8ca9671282fafb69da92728b" ..
            "1a71de0a9e060b2905d6a5b67ecd3b36" ..
            "92ddbd7f2d778b8c9803aee328091b58" ..
            "fab324e4fad675945585808b4831d7bc" ..
            "3ff4def08e4b7a9de576d26586cec64b" ..
            "6116"
        )
        local expected_tag = h("1ae10b594f09e26a7e902ecbd0600691")

        local ct, tag = cc.aead_encrypt(plaintext, key, nonce, aad)
        assert.are.equal(to_hex(expected_ct), to_hex(ct))
        assert.are.equal(to_hex(expected_tag), to_hex(tag))
    end)

    it("RFC 8439 Section 2.8.2 — decryption", function()
        local key = h(
            "808182838485868788898a8b8c8d8e8f" ..
            "909192939495969798999a9b9c9d9e9f"
        )
        local nonce = h("070000004041424344454647")
        local aad = h("50515253c0c1c2c3c4c5c6c7")
        local ciphertext = h(
            "d31a8d34648e60db7b86afbc53ef7ec2" ..
            "a4aded51296e08fea9e2b5a736ee62d6" ..
            "3dbea45e8ca9671282fafb69da92728b" ..
            "1a71de0a9e060b2905d6a5b67ecd3b36" ..
            "92ddbd7f2d778b8c9803aee328091b58" ..
            "fab324e4fad675945585808b4831d7bc" ..
            "3ff4def08e4b7a9de576d26586cec64b" ..
            "6116"
        )
        local tag = h("1ae10b594f09e26a7e902ecbd0600691")

        local expected_pt =
            "Ladies and Gentlemen of the class of '99: " ..
            "If I could offer you only one tip for the future, " ..
            "sunscreen would be it."

        local pt = cc.aead_decrypt(ciphertext, key, nonce, aad, tag)
        assert.are.equal(expected_pt, pt)
    end)

    it("round-trip encrypt/decrypt", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local aad = "some metadata"
        local plaintext = "secret message!"

        local ct, tag = cc.aead_encrypt(plaintext, key, nonce, aad)
        local recovered = cc.aead_decrypt(ct, key, nonce, aad, tag)
        assert.are.equal(plaintext, recovered)
    end)

    it("authentication failure — wrong tag", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local aad = "metadata"
        local plaintext = "secret"

        local ct, _ = cc.aead_encrypt(plaintext, key, nonce, aad)
        local bad_tag = string.rep("\0", 16)
        local result, err = cc.aead_decrypt(ct, key, nonce, aad, bad_tag)
        assert.is_nil(result)
        assert.are.equal("authentication failed", err)
    end)

    it("authentication failure — tampered ciphertext", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local aad = "metadata"
        local plaintext = "secret"

        local ct, tag = cc.aead_encrypt(plaintext, key, nonce, aad)
        -- Flip one bit in the ciphertext
        local tampered = string.char(string.byte(ct, 1) ~ 1) .. string.sub(ct, 2)
        local result, err = cc.aead_decrypt(tampered, key, nonce, aad, tag)
        assert.is_nil(result)
        assert.are.equal("authentication failed", err)
    end)

    it("authentication failure — wrong AAD", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local plaintext = "secret"

        local ct, tag = cc.aead_encrypt(plaintext, key, nonce, "correct aad")
        local result, err = cc.aead_decrypt(ct, key, nonce, "wrong aad", tag)
        assert.is_nil(result)
        assert.are.equal("authentication failed", err)
    end)

    it("empty plaintext with AAD", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")
        local aad = "authenticate this"

        local ct, tag = cc.aead_encrypt("", key, nonce, aad)
        assert.are.equal("", ct)
        assert.are.equal(16, #tag)

        local recovered = cc.aead_decrypt(ct, key, nonce, aad, tag)
        assert.are.equal("", recovered)
    end)

    it("empty AAD", function()
        local key = h(
            "000102030405060708090a0b0c0d0e0f" ..
            "101112131415161718191a1b1c1d1e1f"
        )
        local nonce = h("000000000000000000000000")

        local ct, tag = cc.aead_encrypt("hello", key, nonce, "")
        local recovered = cc.aead_decrypt(ct, key, nonce, "", tag)
        assert.are.equal("hello", recovered)
    end)
end)
