-- Comprehensive tests for the Scytale cipher implementation.

local scytale = require("coding_adventures.scytale_cipher")

describe("ScytaleCipher", function()
    -- Encryption tests
    describe("encrypt", function()
        it("encrypts HELLO WORLD with key=3", function()
            assert.equals("HLWLEOODL R ", scytale.encrypt("HELLO WORLD", 3))
        end)

        it("encrypts ABCDEF with key=2", function()
            assert.equals("ACEBDF", scytale.encrypt("ABCDEF", 2))
        end)

        it("encrypts ABCDEF with key=3", function()
            assert.equals("ADBECF", scytale.encrypt("ABCDEF", 3))
        end)

        it("handles key equal to text length", function()
            assert.equals("ABCD", scytale.encrypt("ABCD", 4))
        end)

        it("returns empty for empty string", function()
            assert.equals("", scytale.encrypt("", 2))
        end)

        it("raises on key < 2", function()
            assert.has_error(function() scytale.encrypt("HELLO", 1) end)
        end)

        it("raises on key > text length", function()
            assert.has_error(function() scytale.encrypt("HI", 3) end)
        end)
    end)

    -- Decryption tests
    describe("decrypt", function()
        it("decrypts HELLO WORLD with key=3", function()
            assert.equals("HELLO WORLD", scytale.decrypt("HLWLEOODL R ", 3))
        end)

        it("decrypts ACEBDF with key=2", function()
            assert.equals("ABCDEF", scytale.decrypt("ACEBDF", 2))
        end)

        it("returns empty for empty string", function()
            assert.equals("", scytale.decrypt("", 2))
        end)

        it("raises on invalid key", function()
            assert.has_error(function() scytale.decrypt("HELLO", 0) end)
            assert.has_error(function() scytale.decrypt("HI", 3) end)
        end)
    end)

    -- Round trip tests
    describe("round trip", function()
        it("round-trips HELLO WORLD", function()
            local text = "HELLO WORLD"
            assert.equals(text, scytale.decrypt(scytale.encrypt(text, 3), 3))
        end)

        it("round-trips with various keys", function()
            local text = "The quick brown fox jumps over the lazy dog!"
            local n = #text
            for key = 2, math.floor(n / 2) do
                local ct = scytale.encrypt(text, key)
                local pt = scytale.decrypt(ct, key)
                assert.equals(text, pt, "Round trip failed for key=" .. key)
            end
        end)
    end)

    -- Brute force tests
    describe("brute_force", function()
        it("finds original text", function()
            local original = "HELLO WORLD"
            local ct = scytale.encrypt(original, 3)
            local results = scytale.brute_force(ct)
            local found = false
            for _, r in ipairs(results) do
                if r.key == 3 and r.text == original then
                    found = true
                    break
                end
            end
            assert.is_true(found)
        end)

        it("returns all keys 2 to n/2", function()
            local results = scytale.brute_force("ABCDEFGHIJ")
            assert.equals(4, #results)
            assert.equals(2, results[1].key)
            assert.equals(3, results[2].key)
            assert.equals(4, results[3].key)
            assert.equals(5, results[4].key)
        end)

        it("returns empty for short text", function()
            assert.equals(0, #scytale.brute_force("AB"))
        end)
    end)

    -- Padding tests
    describe("padding", function()
        it("strips padding on decrypt", function()
            local ct = scytale.encrypt("HELLO", 3)
            assert.equals("HELLO", scytale.decrypt(ct, 3))
        end)

        it("no padding when evenly divisible", function()
            local ct = scytale.encrypt("ABCDEF", 2)
            assert.equals(6, #ct)
        end)
    end)
end)
