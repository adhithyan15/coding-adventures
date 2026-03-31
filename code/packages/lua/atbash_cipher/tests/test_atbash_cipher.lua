-- ============================================================================
-- Comprehensive tests for the Atbash cipher implementation.
-- ============================================================================
--
-- These tests verify that the Atbash cipher correctly reverses the alphabet
-- for both uppercase and lowercase letters, preserves non-alphabetic
-- characters, and satisfies the self-inverse property.

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local m = require("coding_adventures.atbash_cipher")

describe("atbash-cipher", function()
    it("has a VERSION", function()
        assert.is_not_nil(m.VERSION)
        assert.equals("0.1.0", m.VERSION)
    end)
end)

describe("encrypt", function()
    -- Basic Encryption

    it("encrypts HELLO to SVOOL", function()
        -- H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
        assert.equals("SVOOL", m.encrypt("HELLO"))
    end)

    it("encrypts hello to svool (case preservation)", function()
        assert.equals("svool", m.encrypt("hello"))
    end)

    it("encrypts mixed case with punctuation", function()
        assert.equals("Svool, Dliow! 123", m.encrypt("Hello, World! 123"))
    end)

    it("reverses full uppercase alphabet", function()
        assert.equals("ZYXWVUTSRQPONMLKJIHGFEDCBA", m.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"))
    end)

    it("reverses full lowercase alphabet", function()
        assert.equals("zyxwvutsrqponmlkjihgfedcba", m.encrypt("abcdefghijklmnopqrstuvwxyz"))
    end)

    -- Case Preservation

    it("preserves uppercase", function()
        assert.equals("ZYX", m.encrypt("ABC"))
    end)

    it("preserves lowercase", function()
        assert.equals("zyx", m.encrypt("abc"))
    end)

    it("preserves mixed case", function()
        assert.equals("ZyXwVu", m.encrypt("AbCdEf"))
    end)

    -- Non-Alpha Passthrough

    it("passes digits through unchanged", function()
        assert.equals("12345", m.encrypt("12345"))
    end)

    it("passes punctuation through unchanged", function()
        assert.equals("!@#$%", m.encrypt("!@#$%"))
    end)

    it("passes spaces through unchanged", function()
        assert.equals("   ", m.encrypt("   "))
    end)

    it("handles mixed alpha and digits", function()
        assert.equals("Z1Y2X3", m.encrypt("A1B2C3"))
    end)

    it("passes newlines and tabs through", function()
        assert.equals("Z\nY\tX", m.encrypt("A\nB\tC"))
    end)

    -- Edge Cases

    it("handles empty string", function()
        assert.equals("", m.encrypt(""))
    end)

    it("handles single uppercase letters", function()
        assert.equals("Z", m.encrypt("A"))
        assert.equals("A", m.encrypt("Z"))
        assert.equals("N", m.encrypt("M"))
        assert.equals("M", m.encrypt("N"))
    end)

    it("handles single lowercase letters", function()
        assert.equals("z", m.encrypt("a"))
        assert.equals("a", m.encrypt("z"))
    end)

    it("handles single digit", function()
        assert.equals("5", m.encrypt("5"))
    end)

    it("no letter maps to itself", function()
        -- 25 - p == p only when p == 12.5, which is not an integer
        for i = 0, 25 do
            local upper = string.char(65 + i)
            assert.is_not.equals(upper, m.encrypt(upper))

            local lower = string.char(97 + i)
            assert.is_not.equals(lower, m.encrypt(lower))
        end
    end)
end)

describe("self-inverse property", function()
    it("is self-inverse for HELLO", function()
        assert.equals("HELLO", m.encrypt(m.encrypt("HELLO")))
    end)

    it("is self-inverse for lowercase", function()
        assert.equals("hello", m.encrypt(m.encrypt("hello")))
    end)

    it("is self-inverse for mixed input", function()
        local input = "Hello, World! 123"
        assert.equals(input, m.encrypt(m.encrypt(input)))
    end)

    it("is self-inverse for full alphabet", function()
        local alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        assert.equals(alpha, m.encrypt(m.encrypt(alpha)))
    end)

    it("is self-inverse for empty string", function()
        assert.equals("", m.encrypt(m.encrypt("")))
    end)

    it("is self-inverse for long text", function()
        local text = "The quick brown fox jumps over the lazy dog! 42"
        assert.equals(text, m.encrypt(m.encrypt(text)))
    end)
end)

describe("decrypt", function()
    it("decrypts SVOOL to HELLO", function()
        assert.equals("HELLO", m.decrypt("SVOOL"))
    end)

    it("decrypts svool to hello", function()
        assert.equals("hello", m.decrypt("svool"))
    end)

    it("is the inverse of encrypt", function()
        local texts = {"HELLO", "hello", "Hello, World! 123", "", "42"}
        for _, text in ipairs(texts) do
            assert.equals(text, m.decrypt(m.encrypt(text)))
        end
    end)

    it("produces same output as encrypt", function()
        local texts = {"HELLO", "svool", "Test!", ""}
        for _, text in ipairs(texts) do
            assert.equals(m.encrypt(text), m.decrypt(text))
        end
    end)
end)
