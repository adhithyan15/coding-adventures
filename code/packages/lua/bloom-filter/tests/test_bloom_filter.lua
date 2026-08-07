local bloom_filter = require("coding_adventures.bloom_filter")

describe("BloomFilter", function()
    it("starts empty with the default sizing", function()
        local filter = bloom_filter.new()
        assert.equals(9586, filter:bit_count())
        assert.equals(7, filter:hash_count())
        assert.equals(1199, filter:size_bytes())
        assert.equals(0, filter:bits_set())
        assert.equals(0, filter:fill_ratio())
        assert.equals(0, filter:estimated_false_positive_rate())
        assert.is_false(filter:is_over_capacity())
        assert.is_false(filter:contains("anything"))
    end)

    it("has no false negatives for inserted values", function()
        local filter = bloom_filter.new(1000, 0.01)
        for index = 1, 250 do
            filter:add("item-" .. index)
        end
        for index = 1, 250 do
            assert.is_true(filter:contains("item-" .. index))
        end
        assert.is_true(filter:bits_set() > 0)
    end)

    it("supports explicit parameters", function()
        local filter = bloom_filter.from_params(10000, 7)
        assert.equals(10000, filter:bit_count())
        assert.equals(7, filter:hash_count())
        assert.equals(1250, filter:size_bytes())
        filter:add("hello")
        assert.is_true(filter:contains("hello"))
        assert.is_false(filter:is_over_capacity())
    end)

    it("counts each set bit only once", function()
        local filter = bloom_filter.new(100, 0.01)
        filter:add("duplicate")
        local after_first = filter:bits_set()
        filter:add("duplicate")
        assert.equals(after_first, filter:bits_set())
    end)

    it("computes sizing helpers", function()
        local bit_count = bloom_filter.optimal_m(1000000, 0.01)
        assert.equals(9585059, bit_count)
        assert.equals(7, bloom_filter.optimal_k(bit_count, 1000000))
        assert.equals(834632, bloom_filter.capacity_for_memory(1000000, 0.01))
        assert.equals(0, bloom_filter.capacity_for_memory(0, 0.01))
    end)

    it("tracks capacity and renders useful statistics", function()
        local filter = bloom_filter.new(3, 0.01)
        filter:add("a")
        filter:add("b")
        filter:add("c")
        assert.is_false(filter:is_over_capacity())
        filter:add("d")
        assert.is_true(filter:is_over_capacity())
        assert.is_true(filter:estimated_false_positive_rate() > 0)
        assert.matches("BloomFilter", tostring(filter), nil, true)
        assert.matches("bits_set=", filter:to_string(), nil, true)
    end)

    it("encodes scalar and composite elements deterministically", function()
        local filter = bloom_filter.new(100, 0.01)
        local values = {
            "cafe\204\129",
            42,
            3.14,
            true,
            { name = "Ada", tags = { "math", "code" } },
        }
        for _, value in ipairs(values) do
            filter:add(value)
            assert.is_true(filter:contains(value))
        end
        assert.is_true(filter:contains({ tags = { "math", "code" }, name = "Ada" }))
        filter:add(nil)
        assert.is_true(filter:contains(nil))
    end)

    it("rejects invalid parameters and cyclic elements", function()
        assert.has_error(function()
            bloom_filter.new(0, 0.01)
        end, "expected_items must be a positive integer")
        assert.has_error(function()
            bloom_filter.new(1, 0)
        end, "false_positive_rate must be in the open interval (0, 1)")
        assert.has_error(function()
            bloom_filter.new(1, 0 / 0)
        end, "false_positive_rate must be in the open interval (0, 1)")
        assert.has_error(function()
            bloom_filter.from_params(0, 1)
        end, "bit_count must be a positive integer")
        assert.has_error(function()
            bloom_filter.from_params(1, 0)
        end, "hash_count must be a positive integer")
        local cyclic = {}
        cyclic.self = cyclic
        local filter = bloom_filter.new()
        assert.has_error(function()
            filter:add(cyclic)
        end, "element tables must not contain cycles")
        assert.has_error(function()
            filter:add(function() end)
        end, "element must be nil, a scalar, or a table")
    end)
end)
