-- Comprehensive tests for the Vigenere cipher implementation.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local vigenere = require("coding_adventures.vigenere_cipher")

-- A long English text for cryptanalysis testing. Needs to be 200+ characters
-- to give the IC analysis enough statistical data to work with.
local LONG_ENGLISH_TEXT = "The Vigenere cipher was long considered unbreakable and was known as "
    .. "le chiffre indechiffrable for three hundred years until Friedrich "
    .. "Kasiski published a general method of cryptanalysis in eighteen "
    .. "sixty three which exploits the repeating nature of the keyword to "
    .. "determine the key length and then uses frequency analysis on each "
    .. "group of letters encrypted with the same key letter to recover the "
    .. "original plaintext message without knowing the secret keyword at all "
    .. "this technique works because each group of letters encrypted with the "
    .. "same key letter forms a simple caesar cipher which can be broken by "
    .. "comparing the frequency distribution of letters against the expected "
    .. "frequencies found in normal english language text passages and "
    .. "selecting the shift value that produces the closest match"

describe("VigenereCipher", function()

    -- =====================================================================
    -- Encryption Tests
    -- =====================================================================

    describe("encrypt", function()
        it("encrypts ATTACKATDAWN with LEMON (parity vector)", function()
            assert.equals("LXFOPVEFRNHR", vigenere.encrypt("ATTACKATDAWN", "LEMON"))
        end)

        it("encrypts mixed case with punctuation (parity vector)", function()
            assert.equals("Rijvs, Uyvjn!", vigenere.encrypt("Hello, World!", "key"))
        end)

        it("preserves non-alpha characters unchanged", function()
            assert.equals("L-X-F", vigenere.encrypt("A-T-T", "LEM"))
        end)

        it("key wraps around when shorter than plaintext", function()
            -- Key "AB" means shifts of [0,1,0,1,...]
            -- A(+0)=A, B(+1)=C, B(+0)=B, A(+1)=B
            assert.equals("ACBB", vigenere.encrypt("ABBA", "AB"))
        end)

        it("handles single character", function()
            assert.equals("B", vigenere.encrypt("A", "B"))
        end)

        it("handles lowercase key with uppercase plaintext", function()
            assert.equals("LXFOPVEFRNHR", vigenere.encrypt("ATTACKATDAWN", "lemon"))
        end)

        it("handles uppercase key with lowercase plaintext", function()
            assert.equals("lxfopvefrnhr", vigenere.encrypt("attackatdawn", "LEMON"))
        end)

        it("empty plaintext returns empty string", function()
            assert.equals("", vigenere.encrypt("", "key"))
        end)

        it("errors on empty key", function()
            assert.has_error(function() vigenere.encrypt("hello", "") end)
        end)

        it("errors on non-alpha key", function()
            assert.has_error(function() vigenere.encrypt("hello", "key1") end)
        end)
    end)

    -- =====================================================================
    -- Decryption Tests
    -- =====================================================================

    describe("decrypt", function()
        it("decrypts LXFOPVEFRNHR with LEMON (parity vector)", function()
            assert.equals("ATTACKATDAWN", vigenere.decrypt("LXFOPVEFRNHR", "LEMON"))
        end)

        it("decrypts mixed case with punctuation (parity vector)", function()
            assert.equals("Hello, World!", vigenere.decrypt("Rijvs, Uyvjn!", "key"))
        end)

        it("preserves non-alpha characters", function()
            assert.equals("A-T-T", vigenere.decrypt("L-X-F", "LEM"))
        end)

        it("handles single character", function()
            assert.equals("A", vigenere.decrypt("B", "B"))
        end)

        it("empty ciphertext returns empty string", function()
            assert.equals("", vigenere.decrypt("", "key"))
        end)

        it("errors on empty key", function()
            assert.has_error(function() vigenere.decrypt("hello", "") end)
        end)
    end)

    -- =====================================================================
    -- Round Trip Tests
    -- =====================================================================

    describe("round trip", function()
        it("encrypt then decrypt returns original (uppercase)", function()
            local text = "ATTACKATDAWN"
            assert.equals(text, vigenere.decrypt(vigenere.encrypt(text, "LEMON"), "LEMON"))
        end)

        it("encrypt then decrypt returns original (mixed case + punct)", function()
            local text = "Hello, World! This is a test of the Vigenere cipher."
            assert.equals(text, vigenere.decrypt(vigenere.encrypt(text, "secret"), "secret"))
        end)

        it("works with long text and various keys", function()
            local keys = {"A", "KEY", "LONGER", "VERYLONGKEYWORD"}
            for _, k in ipairs(keys) do
                local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, k)
                local pt = vigenere.decrypt(ct, k)
                assert.equals(LONG_ENGLISH_TEXT, pt, "Round trip failed for key=" .. k)
            end
        end)
    end)

    -- =====================================================================
    -- Key Length Detection Tests
    -- =====================================================================

    describe("find_key_length", function()
        it("detects key length for known encryption", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "SECRET")
            local detected = vigenere.find_key_length(ct)
            -- The detected length should be the actual length or a multiple
            assert.equals(0, detected % 6,
                "Expected key length 6 or multiple, got " .. detected)
        end)

        it("detects key length 3", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "KEY")
            local detected = vigenere.find_key_length(ct)
            assert.equals(0, detected % 3,
                "Expected key length 3 or multiple, got " .. detected)
        end)

        it("respects max_length parameter", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "SECRET")
            local detected = vigenere.find_key_length(ct, 4)
            -- With max_length=4, it cannot detect 6, so it should return
            -- something in range 2..4
            assert.is_true(detected >= 1 and detected <= 4)
        end)
    end)

    -- =====================================================================
    -- Key Finding Tests
    -- =====================================================================

    describe("find_key", function()
        it("finds correct key given correct length", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "SECRET")
            local key = vigenere.find_key(ct, 6)
            assert.equals("SECRET", key)
        end)

        it("finds a 3-letter key", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "KEY")
            local key = vigenere.find_key(ct, 3)
            assert.equals("KEY", key)
        end)
    end)

    -- =====================================================================
    -- Full Break Tests
    -- =====================================================================

    describe("break_cipher", function()
        it("breaks a cipher and recovers plaintext", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "SECRET")
            local key, pt = vigenere.break_cipher(ct)
            -- The recovered key may be the original or a repeated version
            -- (IC analysis can find multiples of the true key length).
            -- The critical check is that decryption with the recovered key
            -- produces the original plaintext.
            assert.equals(LONG_ENGLISH_TEXT, pt)
            -- Verify key length is a multiple of the true length
            assert.equals(0, #key % 6,
                "Expected key length multiple of 6, got " .. #key)
        end)

        it("breaks a cipher with a 3-letter key", function()
            local ct = vigenere.encrypt(LONG_ENGLISH_TEXT, "KEY")
            local key, pt = vigenere.break_cipher(ct)
            assert.equals(LONG_ENGLISH_TEXT, pt)
            assert.equals(0, #key % 3,
                "Expected key length multiple of 3, got " .. #key)
        end)
    end)

    -- =====================================================================
    -- Edge Cases
    -- =====================================================================

    describe("edge cases", function()
        it("key A is identity (no shift)", function()
            local text = "Hello, World!"
            assert.equals(text, vigenere.encrypt(text, "A"))
        end)

        it("key Z shifts by 25", function()
            assert.equals("Z", vigenere.encrypt("A", "Z"))
            assert.equals("A", vigenere.encrypt("B", "Z"))
        end)

        it("numbers and symbols pass through", function()
            local text = "Test 123 !@# end"
            local ct = vigenere.encrypt(text, "KEY")
            local pt = vigenere.decrypt(ct, "KEY")
            assert.equals(text, pt)
        end)

        it("key does not advance on non-alpha", function()
            -- With key "AB" (shifts 0,1), spaces should not advance key
            -- "A B" -> A(shift 0)=A, space, B(shift 1)=C
            assert.equals("A C", vigenere.encrypt("A B", "AB"))
        end)
    end)
end)
