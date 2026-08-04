package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local hashes = require("coding_adventures.hash_functions")

describe("hash-functions", function()
    it("matches FNV-1a 32-bit vectors and handles arbitrary bytes", function()
        assert.equals(2166136261, hashes.fnv1a_32(""))
        assert.equals(3826002220, hashes.fnv1a_32("a"))
        assert.equals(440920331, hashes.fnv1a_32("abc"))
        assert.equals(1335831723, hashes.fnv1a_32("hello"))
        assert.equals(3214735720, hashes.fnv1a_32("foobar"))

        local all_bytes = {}
        for byte = 0, 255 do
            all_bytes[#all_bytes + 1] = string.char(byte)
        end
        local payload = table.concat(all_bytes)
        assert.equals(hashes.fnv1a_32(payload), hashes.fnv1a_32(payload))
        assert.not_equals(hashes.fnv1a_32("a\0b"), hashes.fnv1a_32("ab"))
    end)

    it("matches exact FNV-1a 64-bit words", function()
        assert.equals("cbf29ce484222325", hashes.uint64_hex(hashes.fnv1a_64("")))
        assert.equals("af63dc4c8601ec8c", hashes.uint64_hex(hashes.fnv1a_64("a")))
        assert.equals("e71fa2190541574b", hashes.uint64_hex(hashes.fnv1a_64("abc")))
        assert.equals("a430d84680aabd0b", hashes.uint64_hex(hashes.fnv1a_64("hello")))
    end)

    it("matches DJB2 vectors and preserves 64-bit wrapping", function()
        assert.equals(5381, hashes.djb2(""))
        assert.equals(177670, hashes.djb2("a"))
        assert.equals(193485963, hashes.djb2("abc"))
        assert.equals(210714636441, hashes.djb2("hello"))
        assert.equals("cb2c236ad13cc66d", hashes.uint64_hex(hashes.djb2(string.rep("a", 1000))))
    end)

    it("computes polynomial hashes without integer overflow", function()
        assert.equals(0, hashes.polynomial_rolling(""))
        assert.equals(97, hashes.polynomial_rolling("a"))
        assert.equals(3105, hashes.polynomial_rolling("ab"))
        assert.equals(96354, hashes.polynomial_rolling("abc"))
        assert.not_equals(
            hashes.polynomial_rolling("hello", 31),
            hashes.polynomial_rolling("hello", 37)
        )
        assert.is_true(hashes.polynomial_rolling("hello world", 31, 100) < 100)

        local long = string.rep("hash me", 500)
        local default_modulus = (1 << 61) - 1
        assert.is_true(hashes.polynomial_rolling(long) < default_modulus)
        assert.has_error(function()
            hashes.polynomial_rolling("x", 31, 0)
        end, "modulus must be positive")
    end)

    it("matches MurmurHash3 vectors and all tail paths", function()
        assert.equals(0, hashes.murmur3_32("", 0))
        assert.equals(0x514e28b7, hashes.murmur3_32("", 1))
        assert.equals(0x3c2569b2, hashes.murmur3_32("a", 0))
        assert.equals(0xb3dd93fa, hashes.murmur3_32("abc", 0))
        assert.not_equals(hashes.murmur3_32("abcd"), hashes.murmur3_32("abce"))
        assert.is_true(hashes.murmur3_32("abcde") >= 0)
        assert.is_true(hashes.murmur3_32("abcdef") >= 0)
        assert.is_true(hashes.murmur3_32("abcdefg") >= 0)
        assert.not_equals(hashes.murmur3_32("hello", 0), hashes.murmur3_32("hello", 1))
    end)

    it("computes deterministic avalanche and distribution metrics", function()
        local fnv_score = hashes.avalanche_score(hashes.fnv1a_32, 32, 8)
        local murmur_score = hashes.avalanche_score(hashes.murmur3_32, 32, 8)
        assert.is_true(fnv_score >= 0 and fnv_score <= 1)
        assert.is_true(murmur_score >= 0 and murmur_score <= 1)
        assert.equals(fnv_score, hashes.avalanche_score(hashes.fnv1a_32, 32, 8))

        local chi_squared = hashes.distribution_test(function()
            return 0
        end, { "a", "b", "c", "d" }, 4)
        assert.equals(12, chi_squared)

        local signed_word_chi = hashes.distribution_test(
            hashes.fnv1a_64,
            { "a", "b", "c", "d", "e" },
            7
        )
        assert.is_true(signed_word_chi >= 0)
    end)

    it("validates public inputs", function()
        assert.has_error(function()
            hashes.fnv1a_32({})
        end, "data must be a string")
        assert.has_error(function()
            hashes.murmur3_32("x", 1.5)
        end, "seed must be an integer")
        assert.has_error(function()
            hashes.avalanche_score(hashes.fnv1a_32, 0, 1)
        end, "output_bits must be in 1..64")
        assert.has_error(function()
            hashes.avalanche_score(hashes.fnv1a_32, 32, 0)
        end, "sample_size must be positive")
        assert.has_error(function()
            hashes.distribution_test(hashes.fnv1a_32, {}, 10)
        end, "inputs must be a non-empty array")
        assert.has_error(function()
            hashes.distribution_test(hashes.fnv1a_32, { "x" }, 0)
        end, "num_buckets must be positive")
    end)
end)
