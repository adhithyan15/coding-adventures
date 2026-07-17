local hash_set = require("coding_adventures.hash_set")

local function sorted(values)
    table.sort(values)
    return values
end

local function for_each_strategy(callback)
    for _, strategy in ipairs({ "chaining", "open_addressing" }) do
        callback(strategy)
    end
end

describe("HashSet", function()
    it("deduplicates elements and checks membership with both strategies", function()
        for_each_strategy(function(strategy)
            local set = hash_set.from_list({ 1, 1, 2, 2, 3 }, 4, strategy)
            assert.equals(3, set:size())
            assert.equals(3, #set)
            assert.is_true(set:contains(1))
            assert.is_true(set:has(2))
            assert.is_false(set:contains(4))
            assert.is_false(set:is_empty())
            assert.same({ 1, 2, 3 }, sorted(set:to_list()))
        end)
    end)

    it("adds, removes, and discards persistently", function()
        local base = hash_set.from_list({ "alpha", "beta" })
        local added = base:add("gamma")
        local removed = added:remove("alpha")
        local unchanged = removed:discard("missing")
        assert.same({ "alpha", "beta" }, sorted(base:to_list()))
        assert.same({ "alpha", "beta", "gamma" }, sorted(added:to_list()))
        assert.same({ "beta", "gamma" }, sorted(removed:to_list()))
        assert.same({ "beta", "gamma" }, sorted(unchanged:to_list()))
    end)

    it("supports complete set algebra", function()
        local left = hash_set.from_list({ 1, 2, 3, 4, 5 })
        local right = hash_set.from_list({ 3, 4, 5, 6, 7 })
        assert.same({ 1, 2, 3, 4, 5, 6, 7 }, sorted(left:union(right):to_list()))
        assert.same({ 3, 4, 5 }, sorted(left:intersection(right):to_list()))
        assert.same({ 1, 2 }, sorted(left:difference(right):to_list()))
        assert.same({ 1, 2, 6, 7 }, sorted(left:symmetric_difference(right):to_list()))
        assert.same({ 1, 2, 3, 4, 5 }, sorted(left:to_list()))
    end)

    it("supports subset, superset, disjoint, and equality checks", function()
        local subset = hash_set.from_list({ 1, 2, 3 })
        local superset = hash_set.from_list({ 1, 2, 3, 4, 5 })
        local disjoint = hash_set.from_list({ 10, 20 })
        assert.is_true(subset:is_subset(superset))
        assert.is_true(superset:is_superset(subset))
        assert.is_false(superset:is_subset(subset))
        assert.is_true(subset:is_disjoint(disjoint))
        assert.is_false(subset:is_disjoint(superset))
        assert.is_true(subset:equals(hash_set.from_list({ 3, 2, 1 })))
        assert.is_false(subset:equals(superset))
        assert.is_true(hash_set.new():is_subset(subset))
    end)

    it("preserves hash-map options across persistent operations", function()
        local set = hash_set.with_options(4, "open", "murmur3_32")
            :add("Ada")
            :add("Grace")
        assert.equals("open_addressing", set:strategy())
        assert.equals("murmur3", set:hash_fn())
        assert.is_true(set:capacity() >= 4)
        local seeded = hash_set.from_list_with_options(
            { "a", "b", "b", "c" },
            2,
            "chaining",
            "djb2"
        )
        assert.equals(3, seeded:size())
        assert.equals("chaining", seeded:strategy())
        assert.equals("djb2", seeded:hash_fn())
        assert.equals("open_addressing", set:intersection(seeded):strategy())
        assert.equals("open_addressing", set:union(seeded):strategy())
    end)

    it("offers free-function wrappers", function()
        local set = hash_set.from_list({ 10, 20 })
        set = hash_set.add(set, 30)
        assert.is_true(hash_set.contains(set, 30))
        set = hash_set.remove(set, 20)
        assert.is_false(hash_set.has(set, 20))
        set = hash_set.discard(set, 99)
        local other = hash_set.from_list({ 30, 40 })
        local unioned = hash_set.union(set, other)
        assert.same({ 10, 30, 40 }, sorted(hash_set.to_list(unioned)))
        assert.same({ 30 }, sorted(hash_set.intersection(set, other):to_list()))
        assert.same({ 10 }, sorted(hash_set.difference(set, other):to_list()))
        assert.same({ 10, 40 }, sorted(hash_set.symmetric_difference(set, other):to_list()))
        assert.is_true(hash_set.is_subset(set, unioned))
        assert.is_true(hash_set.is_superset(unioned, set))
        assert.is_true(hash_set.is_disjoint(set, hash_set.from_list({ 999 })))
        assert.is_true(hash_set.equals(set, set:clone()))
    end)

    it("uses identity semantics for reference elements", function()
        local reference = {}
        local other_reference = {}
        local set = hash_set.from_list({ reference, reference, other_reference })
        assert.equals(2, set:size())
        assert.is_true(set:contains(reference))
        assert.is_true(set:contains(other_reference))
        assert.is_false(set:contains({}))
        assert.is_true(set:remove(reference):contains(other_reference))
    end)

    it("retains all elements through underlying map resizes", function()
        for_each_strategy(function(strategy)
            local set = hash_set.new(2, strategy)
            for index = 1, 100 do
                set = set:add("key-" .. index)
            end
            assert.equals(100, set:size())
            assert.is_true(set:capacity() >= 100)
            for index = 1, 100 do
                assert.is_true(set:contains("key-" .. index))
            end
        end)
    end)

    it("rejects invalid constructors and nil elements", function()
        assert.has_error(function() hash_set.from_list("not-an-array") end,
            "elements must be an array")
        assert.has_error(function() hash_set.new(0) end,
            "capacity must be a positive integer")
        assert.has_error(function() hash_set.new(4, "quadratic") end,
            "strategy must be 'chaining' or 'open_addressing'")
        assert.has_error(function() hash_set.new(4, nil, "sha256") end,
            "hash_fn must be 'fnv1a', 'murmur3', or 'djb2'")
        assert.has_error(function() hash_set.new():add(nil) end,
            "element must not be nil")
    end)
end)
