local hash_functions = require("coding_adventures.hash_functions")
local hash_map = require("coding_adventures.hash_map")

local function for_each_strategy(callback)
    for _, strategy in ipairs({ "chaining", "open_addressing" }) do
        callback(strategy)
    end
end

local function entries_as_table(map)
    local result = {}
    for _, entry in ipairs(map:entries()) do
        result[entry[1]] = entry[2]
    end
    return result
end

local function colliding_strings(capacity)
    local first_by_bucket = {}
    for index = 1, 1000 do
        local key = "collision-" .. index
        local bucket = hash_functions.fnv1a_32("string:" .. key) % capacity
        if first_by_bucket[bucket] then
            return first_by_bucket[bucket], key
        end
        first_by_bucket[bucket] = key
    end
    error("failed to find colliding keys")
end

describe("HashMap", function()
    it("stores, overwrites, and deletes keys with both strategies", function()
        for_each_strategy(function(strategy)
            local map = hash_map.new(8, strategy)
            assert.equals(map, map:set("hello", 42))
            map:set("world", 7):set("hello", 99)
            assert.equals(2, map:size())
            assert.equals(99, map:get("hello"))
            assert.is_true(map:has("world"))
            assert.is_true(map:delete("hello"))
            assert.is_false(map:delete("missing"))
            assert.is_false(map:has("hello"))
            assert.equals(1, #map)
        end)
    end)

    it("stores nil values without confusing membership", function()
        for_each_strategy(function(strategy)
            local map = hash_map.new(8, strategy):set("nil-value", nil)
            assert.is_true(map:has("nil-value"))
            assert.is_nil(map:get("nil-value"))
            assert.equals(1, map:size())
            assert.equals(1, map:values().n)
        end)
    end)

    it("handles collisions and resizes separate chains", function()
        local map = hash_map.new(2, "chaining")
        map:set("cat", 1):set("car", 2):set("cab", 3)
        assert.equals(4, map:capacity())
        assert.equals(1, map:get("cat"))
        assert.equals(2, map:get("car"))
        assert.equals(3, map:get("cab"))
        assert.is_true(map:load_factor() <= 1.0)
    end)

    it("preserves open-addressing probe chains across tombstones", function()
        local first, second = colliding_strings(8)
        local map = hash_map.new(8, "open_addressing")
        map:set(first, 1):set(second, 2)
        assert.is_true(map:delete(first))
        assert.equals(2, map:get(second))
        map:set("replacement", 3)
        assert.equals(3, map:get("replacement"))
    end)

    it("resizes open addressing at a 0.75 load factor", function()
        local map = hash_map.new(4, "open")
        for index = 1, 4 do
            map:set("key-" .. index, index)
        end
        assert.equals(8, map:capacity())
        assert.equals(4, map:size())
        for index = 1, 4 do
            assert.equals(index, map:get("key-" .. index))
        end
    end)

    it("supports every packaged hash function", function()
        for _, hash_fn in ipairs({ "fnv1a", "fnv1a_32", "murmur3", "murmur3_32", "djb2" }) do
            local map = hash_map.new(4, "open_addressing", hash_fn)
            map:set("Ada", "Lovelace")
            assert.equals("Lovelace", map:get("Ada"))
        end
    end)

    it("provides bulk, construction, merge, clone, and clear operations", function()
        local left = hash_map.from_entries({ { "a", 1 }, { "b", 2 } })
        local right = hash_map.from_entries({ { "b", 99 }, { "c", 3 } })
        local merged = hash_map.merge(left, right)
        local values = entries_as_table(merged)
        assert.same({ a = 1, b = 99, c = 3 }, values)
        assert.equals(3, #merged:keys())
        assert.equals(3, #merged:values())
        local clone = merged:clone()
        clone:set("d", 4)
        assert.is_false(merged:has("d"))
        assert.equals(0, clone:clear():size())
    end)

    it("offers functional wrappers that leave inputs unchanged", function()
        local empty = hash_map.new()
        local filled = hash_map.set(empty, "a", 1)
        local removed = hash_map.delete(filled, "a")
        assert.equals(0, hash_map.size(empty))
        assert.equals(1, hash_map.get(filled, "a"))
        assert.is_false(hash_map.has(removed, "a"))
        assert.matches("HashMap", tostring(filled), nil, true)
    end)

    it("rejects invalid configurations and nil keys", function()
        assert.has_error(function() hash_map.new(0) end, "capacity must be a positive integer")
        assert.has_error(function() hash_map.new(1, "quadratic") end,
            "strategy must be 'chaining' or 'open_addressing'")
        assert.has_error(function() hash_map.new(1, nil, "sha256") end,
            "hash_fn must be 'fnv1a', 'murmur3', or 'djb2'")
        assert.has_error(function() hash_map.new():set(nil, 1) end, "key must not be nil")
        assert.has_error(function() hash_map.from_entries({ { nil, 1 } }) end,
            "each entry must be a key-value pair with a non-nil key")
    end)
end)
