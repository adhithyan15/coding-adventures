-- ============================================================================
-- Tests for Caesar Cipher
-- ============================================================================
-- These tests use the Busted testing framework (https://lunarmodules.github.io/busted/).
-- Run with: busted . --verbose --pattern=test_
--
-- The test suite covers:
--   - Basic encryption and decryption
--   - Round-trip (encrypt then decrypt returns original)
--   - Case preservation
--   - Non-alphabetic character passthrough
--   - Empty string handling
--   - Negative shifts
--   - Shift wrapping (shifts > 26 or < -26)
--   - ROT13 self-inverse property
--   - Brute-force attack
--   - Frequency analysis
--   - Edge cases
-- ============================================================================

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path
local caesar = require("coding_adventures.caesar_cipher")

-- ============================================================================
-- Version
-- ============================================================================

describe("Caesar Cipher", function()
    describe("VERSION", function()
        it("has a version string", function()
            assert.is_not_nil(caesar.VERSION)
            assert.are.equal("0.1.0", caesar.VERSION)
        end)
    end)

    -- ========================================================================
    -- encrypt
    -- ========================================================================

    describe("encrypt", function()
        it("shifts uppercase letters by the given amount", function()
            assert.are.equal("KHOOR", caesar.encrypt("HELLO", 3))
        end)

        it("shifts lowercase letters by the given amount", function()
            assert.are.equal("khoor", caesar.encrypt("hello", 3))
        end)

        it("preserves case of each letter", function()
            local result = caesar.encrypt("HeLLo", 3)
            assert.are.equal("KhOOr", result)
        end)

        it("passes through non-alphabetic characters unchanged", function()
            assert.are.equal("Khoor, Zruog!", caesar.encrypt("Hello, World!", 3))
        end)

        it("handles digits and special characters", function()
            assert.are.equal("123 !@#", caesar.encrypt("123 !@#", 5))
        end)

        it("handles an empty string", function()
            assert.are.equal("", caesar.encrypt("", 3))
        end)

        it("handles shift of 0 (identity)", function()
            assert.are.equal("Hello", caesar.encrypt("Hello", 0))
        end)

        it("handles shift of 26 (full wrap, same as 0)", function()
            assert.are.equal("Hello", caesar.encrypt("Hello", 26))
        end)

        it("handles negative shifts", function()
            -- Shifting by -3 is the same as shifting by 23
            assert.are.equal("EBIIL", caesar.encrypt("HELLO", -3))
        end)

        it("handles large positive shifts (wrapping)", function()
            -- shift=29 is equivalent to shift=3 (29 % 26 = 3)
            assert.are.equal("KHOOR", caesar.encrypt("HELLO", 29))
        end)

        it("handles large negative shifts (wrapping)", function()
            -- shift=-29 is equivalent to shift=-3, which is shift=23
            assert.are.equal("EBIIL", caesar.encrypt("HELLO", -29))
        end)

        it("wraps Z to A with shift=1", function()
            assert.are.equal("A", caesar.encrypt("Z", 1))
            assert.are.equal("a", caesar.encrypt("z", 1))
        end)

        it("wraps A to Z with shift=-1", function()
            assert.are.equal("Z", caesar.encrypt("A", -1))
            assert.are.equal("z", caesar.encrypt("a", -1))
        end)

        it("encrypts the full alphabet", function()
            assert.are.equal(
                "DEFGHIJKLMNOPQRSTUVWXYZABC",
                caesar.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 3)
            )
        end)

        it("handles a single character", function()
            assert.are.equal("D", caesar.encrypt("A", 3))
        end)

        it("handles strings with only non-alpha characters", function()
            assert.are.equal("12345!!! ???", caesar.encrypt("12345!!! ???", 7))
        end)
    end)

    -- ========================================================================
    -- decrypt
    -- ========================================================================

    describe("decrypt", function()
        it("reverses encryption with the same shift", function()
            assert.are.equal("HELLO", caesar.decrypt("KHOOR", 3))
        end)

        it("handles lowercase decryption", function()
            assert.are.equal("hello", caesar.decrypt("khoor", 3))
        end)

        it("preserves case during decryption", function()
            assert.are.equal("HeLLo", caesar.decrypt("KhOOr", 3))
        end)

        it("passes through non-alpha characters", function()
            assert.are.equal("Hello, World!", caesar.decrypt("Khoor, Zruog!", 3))
        end)

        it("handles empty string", function()
            assert.are.equal("", caesar.decrypt("", 5))
        end)

        it("handles shift of 0", function()
            assert.are.equal("test", caesar.decrypt("test", 0))
        end)

        it("handles negative shifts (decrypting with negative = encrypting)", function()
            -- decrypt with shift=-3 is like encrypt with shift=3
            assert.are.equal("KHOOR", caesar.decrypt("HELLO", -3))
        end)
    end)

    -- ========================================================================
    -- Round-trip (encrypt then decrypt)
    -- ========================================================================

    describe("round-trip", function()
        it("encrypt then decrypt returns the original for shift=3", function()
            local original = "The quick brown fox jumps over the lazy dog!"
            local encrypted = caesar.encrypt(original, 3)
            local decrypted = caesar.decrypt(encrypted, 3)
            assert.are.equal(original, decrypted)
        end)

        it("encrypt then decrypt returns the original for shift=13", function()
            local original = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            local encrypted = caesar.encrypt(original, 13)
            local decrypted = caesar.decrypt(encrypted, 13)
            assert.are.equal(original, decrypted)
        end)

        it("encrypt then decrypt returns the original for shift=25", function()
            local original = "Zebra 123"
            local encrypted = caesar.encrypt(original, 25)
            local decrypted = caesar.decrypt(encrypted, 25)
            assert.are.equal(original, decrypted)
        end)

        it("works for all shifts 0-25", function()
            local original = "Test message with CAPS and 123!"
            for shift = 0, 25 do
                local encrypted = caesar.encrypt(original, shift)
                local decrypted = caesar.decrypt(encrypted, shift)
                assert.are.equal(original, decrypted,
                    "Round-trip failed for shift=" .. shift)
            end
        end)
    end)

    -- ========================================================================
    -- rot13
    -- ========================================================================

    describe("rot13", function()
        it("shifts by 13 positions", function()
            assert.are.equal("URYYB", caesar.rot13("HELLO"))
        end)

        it("is self-inverse: applying twice returns the original", function()
            local original = "Hello, World!"
            assert.are.equal(original, caesar.rot13(caesar.rot13(original)))
        end)

        it("handles the full alphabet", function()
            assert.are.equal(
                "NOPQRSTUVWXYZABCDEFGHIJKLM",
                caesar.rot13("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            )
        end)

        it("preserves case", function()
            assert.are.equal("Uryyb", caesar.rot13("Hello"))
        end)

        it("passes through non-alpha characters", function()
            assert.are.equal("123 !@#", caesar.rot13("123 !@#"))
        end)

        it("handles empty string", function()
            assert.are.equal("", caesar.rot13(""))
        end)
    end)

    -- ========================================================================
    -- brute_force
    -- ========================================================================

    describe("brute_force", function()
        it("returns 25 results (shifts 1 through 25)", function()
            local results = caesar.brute_force("KHOOR")
            assert.are.equal(25, #results)
        end)

        it("each result has a shift and plaintext field", function()
            local results = caesar.brute_force("KHOOR")
            for _, entry in ipairs(results) do
                assert.is_not_nil(entry.shift)
                assert.is_not_nil(entry.plaintext)
            end
        end)

        it("shifts are numbered 1 through 25", function()
            local results = caesar.brute_force("KHOOR")
            for i, entry in ipairs(results) do
                assert.are.equal(i, entry.shift)
            end
        end)

        it("contains the correct plaintext at the right shift", function()
            -- "KHOOR" was encrypted with shift=3, so decrypting with shift=3
            -- should yield "HELLO"
            local results = caesar.brute_force("KHOOR")
            assert.are.equal("HELLO", results[3].plaintext)
        end)

        it("handles lowercase ciphertext", function()
            local results = caesar.brute_force("khoor")
            assert.are.equal("hello", results[3].plaintext)
        end)

        it("handles mixed case ciphertext", function()
            local results = caesar.brute_force("KhOOr")
            assert.are.equal("HeLLo", results[3].plaintext)
        end)

        it("handles empty string", function()
            local results = caesar.brute_force("")
            assert.are.equal(25, #results)
            for _, entry in ipairs(results) do
                assert.are.equal("", entry.plaintext)
            end
        end)
    end)

    -- ========================================================================
    -- frequency_analysis
    -- ========================================================================

    describe("frequency_analysis", function()
        it("returns a table with shift and plaintext fields", function()
            local result = caesar.frequency_analysis("KHOOR")
            assert.is_not_nil(result.shift)
            assert.is_not_nil(result.plaintext)
        end)

        it("identifies shift=3 for a short known ciphertext", function()
            -- "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG" encrypted with shift=3
            local plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
            local ciphertext = caesar.encrypt(plaintext, 3)
            local result = caesar.frequency_analysis(ciphertext)
            assert.are.equal(3, result.shift)
            assert.are.equal(plaintext, result.plaintext)
        end)

        it("identifies shift=0 for already-plain English text", function()
            local text = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
            local result = caesar.frequency_analysis(text)
            assert.are.equal(0, result.shift)
        end)

        it("identifies shift=13 for ROT13 ciphertext", function()
            local plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
            local ciphertext = caesar.rot13(plaintext)
            local result = caesar.frequency_analysis(ciphertext)
            assert.are.equal(13, result.shift)
            assert.are.equal(plaintext, result.plaintext)
        end)

        it("works on longer text with shift=7", function()
            local plaintext = "IN CRYPTOGRAPHY A CAESAR CIPHER IS ONE OF THE SIMPLEST " ..
                              "AND MOST WIDELY KNOWN ENCRYPTION TECHNIQUES IT IS A TYPE " ..
                              "OF SUBSTITUTION CIPHER IN WHICH EACH LETTER IN THE PLAINTEXT " ..
                              "IS REPLACED BY A LETTER SOME FIXED NUMBER OF POSITIONS DOWN " ..
                              "THE ALPHABET"
            local ciphertext = caesar.encrypt(plaintext, 7)
            local result = caesar.frequency_analysis(ciphertext)
            assert.are.equal(7, result.shift)
            assert.are.equal(plaintext, result.plaintext)
        end)

        it("handles text with only non-alpha characters", function()
            local result = caesar.frequency_analysis("12345!!! ???")
            -- With no letters, shift=0 should win by default
            assert.are.equal(0, result.shift)
        end)

        it("handles empty string", function()
            local result = caesar.frequency_analysis("")
            assert.are.equal(0, result.shift)
            assert.are.equal("", result.plaintext)
        end)
    end)

    -- ========================================================================
    -- ENGLISH_FREQUENCIES
    -- ========================================================================

    describe("ENGLISH_FREQUENCIES", function()
        it("contains 26 entries", function()
            local count = 0
            for _ in pairs(caesar.ENGLISH_FREQUENCIES) do
                count = count + 1
            end
            assert.are.equal(26, count)
        end)

        it("has entries for all letters A-Z", function()
            for c = string.byte("A"), string.byte("Z") do
                local letter = string.char(c)
                assert.is_not_nil(caesar.ENGLISH_FREQUENCIES[letter],
                    "Missing frequency for " .. letter)
            end
        end)

        it("all frequencies are positive numbers", function()
            for letter, freq in pairs(caesar.ENGLISH_FREQUENCIES) do
                assert.is_true(type(freq) == "number",
                    "Frequency for " .. letter .. " is not a number")
                assert.is_true(freq > 0,
                    "Frequency for " .. letter .. " is not positive")
            end
        end)

        it("frequencies sum to approximately 100", function()
            local sum = 0
            for _, freq in pairs(caesar.ENGLISH_FREQUENCIES) do
                sum = sum + freq
            end
            -- Allow some rounding tolerance
            assert.is_true(sum > 99.9 and sum < 100.1,
                "Frequencies sum to " .. sum .. ", expected ~100")
        end)

        it("E is the most frequent letter", function()
            local max_freq = 0
            local max_letter = ""
            for letter, freq in pairs(caesar.ENGLISH_FREQUENCIES) do
                if freq > max_freq then
                    max_freq = freq
                    max_letter = letter
                end
            end
            assert.are.equal("E", max_letter)
        end)
    end)
end)
