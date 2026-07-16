package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    "../../avl-tree/src/?.lua",
    "../../avl-tree/src/?/init.lua",
    package.path,
}, ";")

local module = require("coding_adventures.tree_set")
local TreeSet = module.TreeSet

describe("TreeSet basics", function()
    it("deduplicates, sorts, mutates, and iterates", function()
        local set = TreeSet.new({5, 1, 3, 3, 9})
        assert.equals(set, set:add(7))

        assert.are.same({1, 3, 5, 7, 9}, set:to_sorted_array())
        assert.are.same({1, 3, 5, 7, 9}, set:to_list())
        assert.equals(5, set:size())
        assert.equals(5, set:length())
        assert.equals(5, #set)
        assert.is_true(set:contains(7))
        assert.is_true(set:has(3))
        assert.is_false(set:has(2))

        local iterated = {}
        for value in set:values() do
            iterated[#iterated + 1] = value
        end
        assert.are.same({1, 3, 5, 7, 9}, iterated)
        assert.equals("TreeSet([1, 3, 5, 7, 9])", tostring(set))
        assert.is_true(set:backend():is_valid_avl())
    end)

    it("supports deletion aliases and defensive arrays", function()
        local set = TreeSet.from_values({1, 2, 3, 4})
        assert.is_true(set:delete(2))
        assert.is_true(set:remove(3))
        assert.is_false(set:discard(99))
        assert.is_false(set:delete(nil))
        assert.are.same({1, 4}, set:to_array())

        local snapshot = set:to_list()
        snapshot[1] = 999
        assert.are.same({1, 4}, set:to_list())
        assert.is_true(set:backend():is_valid_avl())
    end)
end)

describe("TreeSet order queries", function()
    it("provides boundaries, rank, and selection", function()
        local set = module.from_values({10, 20, 30, 40})

        assert.equals(10, set:min())
        assert.equals(40, set:max())
        assert.equals(10, set:first())
        assert.equals(40, set:last())
        assert.equals(20, set:predecessor(30))
        assert.equals(40, set:successor(30))
        assert.equals(0, set:rank(5))
        assert.equals(2, set:rank(25))
        assert.equals(10, set:by_rank(0))
        assert.equals(40, set:by_rank(3))
        assert.is_nil(set:by_rank(-1))
        assert.equals(30, set:kth_smallest(3))
        assert.is_nil(set:kth_smallest(0))
    end)

    it("handles empty sets", function()
        local set = TreeSet.empty()
        assert.is_true(set:is_empty())
        assert.is_nil(set:min())
        assert.is_nil(set:max())
        assert.is_nil(set:first())
        assert.is_nil(set:last())
        assert.is_nil(set:predecessor(1))
        assert.is_nil(set:successor(1))
        assert.is_nil(set:by_rank(0))
        assert.equals(0, set:rank(1))
        assert.equals("TreeSet([])", tostring(set))
    end)

    it("supports inclusive and exclusive ranges", function()
        local set = TreeSet.from_values({1, 3, 5, 7, 9})
        assert.are.same({3, 5, 7}, set:range(3, 7))
        assert.are.same({5}, set:range(3, 7, false))
        assert.are.same({}, set:range(10, 20))
        assert.are.same({}, set:range(7, 3))
    end)
end)

describe("TreeSet algebra", function()
    it("combines sets without mutating inputs", function()
        local left = TreeSet.from_values({1, 2, 3, 5})
        local right = TreeSet.from_values({3, 4, 5, 6})

        assert.are.same({1, 2, 3, 4, 5, 6}, left:union(right):to_list())
        assert.are.same({3, 5}, left:intersection(right):to_list())
        assert.are.same({1, 2}, left:difference(right):to_list())
        assert.are.same({1, 2, 4, 6}, left:symmetric_difference(right):to_list())
        assert.are.same({1, 2, 3, 5}, left:to_list())
        assert.are.same({3, 4, 5, 6}, right:to_list())
    end)

    it("supports subset, superset, disjointness, and equality", function()
        local small = TreeSet.from_values({2, 3})
        local large = TreeSet.from_values({1, 2, 3, 4})
        local disjoint = TreeSet.from_values({8, 9})

        assert.is_true(small:is_subset(large))
        assert.is_true(large:is_superset(small))
        assert.is_true(small:is_disjoint(disjoint))
        assert.is_false(small:is_disjoint(large))
        assert.is_true(small:equals(TreeSet.from_values({3, 2})))
        assert.is_false(small:equals(large))
        assert.is_false(small:equals({2, 3}))
    end)
end)

describe("TreeSet comparison and validation", function()
    it("supports custom comparators", function()
        local function by_length(left, right)
            if #left ~= #right then
                return #left - #right
            end
            return module.default_compare(left, right)
        end
        local set = TreeSet.new({}, by_length):add("banana"):add("fig"):add("apple")

        assert.are.same({"fig", "apple", "banana"}, set:to_list())
        assert.is_true(set:backend():is_valid_avl())
        assert.equals("apple", set:by_rank(1))
    end)

    it("validates public inputs", function()
        assert.has_error(function() TreeSet.new("values") end, "values must be a table")
        assert.has_error(function() TreeSet.new({}, "compare") end, "compare must be a function")
        assert.has_error(function() TreeSet.empty():add(nil) end, "value must not be nil")
        assert.has_error(function() TreeSet.empty():range(1, 2, "yes") end, "inclusive must be a boolean")
        assert.has_error(function() TreeSet.empty():union({}) end, "other must be a TreeSet")
    end)

    it("keeps AVL invariants through a larger mutation sequence", function()
        local set = TreeSet.empty()
        for index = 1, 100 do
            set:add((index * 37) % 101)
            assert.is_true(set:backend():is_valid_avl())
        end
        for value = 1, 99, 2 do
            assert.is_true(set:delete(value))
            assert.is_true(set:backend():is_valid_avl())
        end

        local expected = {}
        for value = 2, 100, 2 do
            expected[#expected + 1] = value
        end
        assert.are.same(expected, set:to_list())
    end)
end)
