-- ============================================================================
-- X25519 Test Suite
-- ============================================================================
-- Tests against all RFC 7748 test vectors.
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local x25519_mod = require("coding_adventures.x25519")
local x25519 = x25519_mod.x25519
local x25519_base = x25519_mod.x25519_base
local generate_keypair = x25519_mod.generate_keypair
local from_hex = x25519_mod.from_hex
local to_hex = x25519_mod.to_hex

describe("X25519", function()

    -- -----------------------------------------------------------------------
    -- RFC 7748 Test Vector 1
    -- -----------------------------------------------------------------------
    describe("RFC 7748 test vector 1", function()
        it("should produce correct output", function()
            local scalar = from_hex("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
            local u = from_hex("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
            local expected = "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"

            local result = x25519(scalar, u)
            assert.are.equal(expected, to_hex(result))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- RFC 7748 Test Vector 2
    -- -----------------------------------------------------------------------
    describe("RFC 7748 test vector 2", function()
        it("should produce correct output", function()
            local scalar = from_hex("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d")
            local u = from_hex("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493")
            local expected = "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957"

            local result = x25519(scalar, u)
            assert.are.equal(expected, to_hex(result))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Base Point Multiplication — Alice
    -- -----------------------------------------------------------------------
    describe("base point multiplication (Alice)", function()
        it("should generate Alice's public key", function()
            local alice_priv = from_hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
            local expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

            local result = x25519_base(alice_priv)
            assert.are.equal(expected, to_hex(result))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Base Point Multiplication — Bob
    -- -----------------------------------------------------------------------
    describe("base point multiplication (Bob)", function()
        it("should generate Bob's public key", function()
            local bob_priv = from_hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
            local expected = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"

            local result = x25519_base(bob_priv)
            assert.are.equal(expected, to_hex(result))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Diffie-Hellman Shared Secret
    -- -----------------------------------------------------------------------
    describe("Diffie-Hellman shared secret", function()
        it("should produce the same secret for both parties", function()
            local alice_priv = from_hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
            local bob_priv = from_hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
            local expected = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

            local alice_pub = x25519_base(alice_priv)
            local bob_pub = x25519_base(bob_priv)

            local shared_alice = x25519(alice_priv, bob_pub)
            local shared_bob = x25519(bob_priv, alice_pub)

            assert.are.equal(expected, to_hex(shared_alice))
            assert.are.equal(expected, to_hex(shared_bob))
            assert.are.equal(shared_alice, shared_bob)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- generate_keypair
    -- -----------------------------------------------------------------------
    describe("generate_keypair", function()
        it("should return private key and derived public key", function()
            local alice_priv = from_hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
            local expected_pub = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

            local priv, pub = generate_keypair(alice_priv)

            assert.are.equal(alice_priv, priv)
            assert.are.equal(expected_pub, to_hex(pub))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Iterated Test — 1 iteration
    -- -----------------------------------------------------------------------
    describe("iterated test", function()
        it("should produce correct result after 1 iteration from k=u=9", function()
            local nine = string.char(9) .. string.rep(string.char(0), 31)
            local expected = "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079"

            local result = x25519(nine, nine)
            assert.are.equal(expected, to_hex(result))
        end)

        -- -----------------------------------------------------------------
        -- Iterated Test — 1000 iterations
        -- -----------------------------------------------------------------
        it("should produce correct result after 1000 iterations from k=u=9", function()
            local k = string.char(9) .. string.rep(string.char(0), 31)
            local u = string.char(9) .. string.rep(string.char(0), 31)

            for _ = 1, 1000 do
                local new_k = x25519(k, u)
                u = k
                k = new_k
            end

            local expected = "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51"
            assert.are.equal(expected, to_hex(k))
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Input Validation
    -- -----------------------------------------------------------------------
    describe("input validation", function()
        it("should reject short scalar", function()
            assert.has_error(function()
                x25519("\x01\x02\x03", string.rep("\x00", 32))
            end, "scalar must be 32 bytes")
        end)

        it("should reject short u_point", function()
            assert.has_error(function()
                x25519(string.rep("\x00", 32), "\x01\x02\x03")
            end, "u_point must be 32 bytes")
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Hex Utilities
    -- -----------------------------------------------------------------------
    describe("hex utilities", function()
        it("should round-trip through hex encoding", function()
            local original = string.char(0xDE, 0xAD, 0xBE, 0xEF)
            assert.are.equal(original, from_hex(to_hex(original)))
        end)
    end)
end)
