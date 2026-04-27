-- ============================================================================
-- Tests for Ed25519 (RFC 8032)
-- ============================================================================
-- These test vectors are verified against libsodium (PyNaCl) and the
-- RFC 8032 appendix reference implementation.

package.path = "../src/?.lua;../src/?/init.lua;"
    .. "../../sha512/src/?.lua;../../sha512/src/?/init.lua;"
    .. package.path

local ed25519 = require("coding_adventures.ed25519")

-- Helper to decode hex strings to binary.
local function from_hex(hex)
    return ed25519.from_hex(hex)
end

local function to_hex(s)
    return ed25519.to_hex(s)
end

-- ============================================================================
-- RFC 8032 Section 7.1 Test Vectors (verified against libsodium)
-- ============================================================================

describe("Ed25519 RFC 8032 test vectors", function()

    -- -----------------------------------------------------------------------
    -- Test 1: Empty message
    -- -----------------------------------------------------------------------
    it("signs and verifies empty message (test vector 1)", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local expected_pub = from_hex(
            "d75a980182b10ab7d54bfed3c964073a"
            .. "0ee172f3daa62325af021a68f707511a"
        )
        local expected_sig = from_hex(
            "e5564300c360ac729086e2cc806e828a"
            .. "84877f1eb8e5d974d873e06522490155"
            .. "5fb8821590a33bacc61e39701cf9b46b"
            .. "d25bf5f0595bbe24655141438e7a100b"
        )

        local pub, sk = ed25519.generate_keypair(seed)
        assert.are.equal(to_hex(expected_pub), to_hex(pub))

        local sig = ed25519.sign("", sk)
        assert.are.equal(to_hex(expected_sig), to_hex(sig))

        assert.is_true(ed25519.verify("", sig, pub))
    end)

    -- -----------------------------------------------------------------------
    -- Test 2: One byte (0x72)
    -- -----------------------------------------------------------------------
    it("signs and verifies one-byte message (test vector 2)", function()
        local seed = from_hex(
            "4ccd089b28ff96da9db6c346ec114e0f"
            .. "5b8a319f35aba624da8cf6ed4fb8a6fb"
        )
        local expected_pub = from_hex(
            "3d4017c3e843895a92b70aa74d1b7ebc"
            .. "9c982ccf2ec4968cc0cd55f12af4660c"
        )
        local expected_sig = from_hex(
            "92a009a9f0d4cab8720e820b5f642540"
            .. "a2b27b5416503f8fb3762223ebdb69da"
            .. "085ac1e43e15996e458f3613d0f11d8c"
            .. "387b2eaeb4302aeeb00d291612bb0c00"
        )
        local message = from_hex("72")

        local pub, sk = ed25519.generate_keypair(seed)
        assert.are.equal(to_hex(expected_pub), to_hex(pub))

        local sig = ed25519.sign(message, sk)
        assert.are.equal(to_hex(expected_sig), to_hex(sig))

        assert.is_true(ed25519.verify(message, sig, pub))
    end)

    -- -----------------------------------------------------------------------
    -- Test 3: Two bytes (0xaf82)
    -- -----------------------------------------------------------------------
    it("signs and verifies two-byte message (test vector 3)", function()
        local seed = from_hex(
            "c5aa8df43f9f837bedb7442f31dcb7b1"
            .. "66d38535076f094b85ce3a2e0b4458f7"
        )
        local expected_pub = from_hex(
            "fc51cd8e6218a1a38da47ed00230f058"
            .. "0816ed13ba3303ac5deb911548908025"
        )
        local expected_sig = from_hex(
            "6291d657deec24024827e69c3abe01a3"
            .. "0ce548a284743a445e3680d7db5ac3ac"
            .. "18ff9b538d16f290ae67f760984dc659"
            .. "4a7c15e9716ed28dc027beceea1ec40a"
        )
        local message = from_hex("af82")

        local pub, sk = ed25519.generate_keypair(seed)
        assert.are.equal(to_hex(expected_pub), to_hex(pub))

        local sig = ed25519.sign(message, sk)
        assert.are.equal(to_hex(expected_sig), to_hex(sig))

        assert.is_true(ed25519.verify(message, sig, pub))
    end)

    -- -----------------------------------------------------------------------
    -- Test 4: 1023 bytes
    -- -----------------------------------------------------------------------
    it("signs and verifies 1023-byte message (test vector 4)", function()
        local seed = from_hex(
            "f5e5767cf153319517630f226876b86c"
            .. "8160cc583bc013744c6bf255f5cc0ee5"
        )
        local expected_pub = from_hex(
            "278117fc144c72340f67d0f2316e8386"
            .. "ceffbf2b2428c9c51fef7c597f1d426e"
        )
        local expected_sig = from_hex(
            "d686294b743c6760c6a78a2c4c2fc761"
            .. "15c2600b8f083acde59e7cee32578c0f"
            .. "59ea4219ab9b5896795e4e2b87a30270"
            .. "aa0e3099eee944e9e67a1b22df41ff07"
        )
        local message = from_hex(
            "08b8b2b733424243760fe426a4b54908"
            .. "632110a66c2f6591eabd3345e3e4eb98"
            .. "fa6e264bf09efe12ee50f8f54e9f77b1"
            .. "e355f6c50544e23fb1433ddf73be84d8"
            .. "79de7c0046dc4996d9e773f4bc9efe57"
            .. "38829adb26c81b37c93a1b270b20329d"
            .. "658675fc6ea534e0810a4432826bf58c"
            .. "941efb65d57a338bbd2e26640f89ffbc"
            .. "1a858efcb8550ee3a5e1998bd177e93a"
            .. "7363c344fe6b199ee5d02e82d522c4fe"
            .. "ba15452f80288a821a579116ec6dad2b"
            .. "3b310da903401aa62100ab5d1a36553e"
            .. "06203b33890cc9b832f79ef80560ccb9"
            .. "a39ce767967ed628c6ad573cb116dbef"
            .. "fefd75499da96bd68a8a97b928a8bbc1"
            .. "03b6621fcde2beca1231d206be6cd9ec"
            .. "7aff6f6c94fcd7204ed3455c68c83f4a"
            .. "41da4af2b74ef5c53f1d8ac70bdcb7ed"
            .. "185ce81bd84359d44254d95629e9855a"
            .. "94a7c1958d1f8ada5d0532ed8a5aa3fb"
            .. "2d17ba70eb6248e594e1a2297acbbb39"
            .. "d502f1a8c6eb6f1ce22b3de1a1f40cc2"
            .. "4554119a831a9aad6079cad88425de6b"
            .. "de1a9187ebb6092cf67bf2b13fd65f27"
            .. "088d78b7e883c8759d2c4f5c65adb755"
            .. "3878ad575f9fad878e80a0c9ba63bcbc"
            .. "c2732e69485bbc9c90bfbd62481d9089"
            .. "beccf80cfe2df16a2cf65bd92dd597b0"
            .. "7e0917af48bbb75fed413d238f5555a7"
            .. "a569d80c3414a8d0859dc65a46128bab"
            .. "27af87a71314f318c782b23ebfe808b8"
            .. "2b0ce26401d2e22f04d83d1255dc51ad"
            .. "dd3b75a2b1ae0784504df543af8969be"
            .. "3ea7082ff7fc9888c144da2af58429ec"
            .. "96031dbcad3dad9af0dcbaaaf268cb8f"
            .. "cffead94f3c7ca495e056a9b47acdb75"
            .. "1fb73e666c6c655ade8297297d07ad1b"
            .. "a5e43f1bca32301651339e22904cc8c4"
            .. "2f58c30c04aafdb038dda0847dd988dc"
            .. "da6f3bfd15c4b4c4525004aa06eeff8c"
            .. "a61783aacec57fb3d1f92b0fe2fd1a85"
            .. "f6724517b65e614ad6808d6f6ee34dff"
            .. "7310fdc82aebfd904b01e1dc54b29270"
            .. "94b2db68d6f903b68401adebf5a7e08d"
            .. "78ff4ef5d63653a65040cf9bfd4aca79"
            .. "84a74d37145986780fc0b16ac451649d"
            .. "e6188a7dbdf191f64b5fc5e2ab47b57f"
            .. "7f7276cd419c17a3ca8e1b939ae49e48"
            .. "8acba6b965610b5480109c8b17b80e1b"
            .. "7b750dfc7598d5d5011fd2dcc5600a32"
            .. "ef5b52a1ecc820e308aa342721aac094"
            .. "3bf6686b64b2579376504ccc493d97e6"
            .. "aed3fb0f9cd71a43dd497f01f17c0e2c"
            .. "b3797aa2a2f256656168e6c496afc5fb"
            .. "93246f6b1116398a346f1a641f3b041e"
            .. "989f7914f90cc2c7fff357876e506b50"
            .. "d334ba77c225bc307ba537152f3f1610"
            .. "e4eafe595f6d9d90d11faa933a15ef13"
            .. "69546868a7f3a45a96768d40fd9d0341"
            .. "2c091c6315cf4fde7cb68606937380db"
            .. "2eaaa707b4c4185c32eddcdd306705e4"
            .. "dc1ffc872eeee475a64dfac86aba41c0"
            .. "618983f8741c5ef68d3a101e8a3b8cac"
            .. "60c905c15fc910840b94c00a0b9d00"
        )

        local pub, sk = ed25519.generate_keypair(seed)
        assert.are.equal(to_hex(expected_pub), to_hex(pub))

        local sig = ed25519.sign(message, sk)
        assert.are.equal(to_hex(expected_sig), to_hex(sig))

        assert.is_true(ed25519.verify(message, sig, pub))
    end)
end)

-- ============================================================================
-- Verification Failure Tests
-- ============================================================================

describe("Ed25519 verification edge cases", function()

    it("rejects signature with wrong message", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, sk = ed25519.generate_keypair(seed)
        local sig = ed25519.sign("hello", sk)
        assert.is_false(ed25519.verify("world", sig, pub))
    end)

    it("rejects signature with wrong public key", function()
        local seed1 = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local seed2 = from_hex(
            "4ccd089b28ff96da9db6c346ec114e0f"
            .. "5b8a319f35aba624da8cf6ed4fb8a6fb"
        )
        local _, sk1 = ed25519.generate_keypair(seed1)
        local pub2, _ = ed25519.generate_keypair(seed2)
        local sig = ed25519.sign("hello", sk1)
        assert.is_false(ed25519.verify("hello", sig, pub2))
    end)

    it("rejects tampered signature", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, sk = ed25519.generate_keypair(seed)
        local sig = ed25519.sign("hello", sk)
        -- Flip a bit in the signature
        local tampered = string.char(string.byte(sig, 1) ~ 1) .. sig:sub(2)
        assert.is_false(ed25519.verify("hello", tampered, pub))
    end)

    it("rejects invalid signature length", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, _ = ed25519.generate_keypair(seed)
        assert.is_false(ed25519.verify("hello", "short", pub))
    end)

    it("rejects invalid public key length", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local _, sk = ed25519.generate_keypair(seed)
        local sig = ed25519.sign("hello", sk)
        assert.is_false(ed25519.verify("hello", sig, "short"))
    end)
end)

-- ============================================================================
-- Key Generation Tests
-- ============================================================================

describe("Ed25519 key generation", function()

    it("produces 32-byte public key", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, _ = ed25519.generate_keypair(seed)
        assert.are.equal(32, #pub)
    end)

    it("produces 64-byte secret key", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local _, sk = ed25519.generate_keypair(seed)
        assert.are.equal(64, #sk)
    end)

    it("secret key starts with seed", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, sk = ed25519.generate_keypair(seed)
        assert.are.equal(seed, sk:sub(1, 32))
        assert.are.equal(pub, sk:sub(33, 64))
    end)

    it("rejects wrong seed length", function()
        assert.has_error(function()
            ed25519.generate_keypair("short")
        end)
    end)
end)

-- ============================================================================
-- Sign/Verify Round-Trip Tests
-- ============================================================================

describe("Ed25519 round-trip", function()

    it("sign and verify with various message lengths", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local pub, sk = ed25519.generate_keypair(seed)

        -- Empty message
        local sig = ed25519.sign("", sk)
        assert.is_true(ed25519.verify("", sig, pub))

        -- Short message
        sig = ed25519.sign("test", sk)
        assert.is_true(ed25519.verify("test", sig, pub))

        -- Longer message
        local long_msg = string.rep("a", 256)
        sig = ed25519.sign(long_msg, sk)
        assert.is_true(ed25519.verify(long_msg, sig, pub))
    end)

    it("produces deterministic signatures", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local _, sk = ed25519.generate_keypair(seed)

        local sig1 = ed25519.sign("hello", sk)
        local sig2 = ed25519.sign("hello", sk)
        assert.are.equal(to_hex(sig1), to_hex(sig2))
    end)

    it("produces 64-byte signatures", function()
        local seed = from_hex(
            "9d61b19deffd5a60ba844af492ec2cc4"
            .. "4449c5697b326919703bac031cae7f60"
        )
        local _, sk = ed25519.generate_keypair(seed)
        local sig = ed25519.sign("hello", sk)
        assert.are.equal(64, #sig)
    end)
end)
