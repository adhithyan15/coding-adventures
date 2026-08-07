package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local module = require("coding_adventures.skip_list")
local SkipList = module.SkipList

describe("SkipList basics", function()
    it("inserts, updates, searches, and tracks membership", function()
        local list = SkipList.new()
        assert.is_true(list:insert(5, "five"))
        assert.is_true(list:insert(2, "two"))
        assert.is_true(list:insert(8, "eight"))
        assert.is_false(list:insert(5, "FIVE"))

        assert.equals("FIVE", list:search(5))
        assert.equals("two", list:get(2))
        assert.is_nil(list:search(99))
        assert.is_true(list:contains(8))
        assert.is_true(list:has(2))
        assert.is_false(list:contains(7))
        assert.equals(3, list:size())
        assert.equals(3, list:length())
        assert.equals(3, #list)
        assert.is_true(list:is_valid_skip_list())
    end)

    it("distinguishes nil values from missing keys", function()
        local list = SkipList.new()
        list:insert(7, nil)
        assert.is_true(list:contains(7))
        assert.is_nil(list:search(7))
        assert.is_false(list:contains(8))
        assert.is_true(list:is_valid_skip_list())
    end)

    it("deletes existing keys and preserves absent keys", function()
        local list = module.from_entries({{1, "one"}, {2, "two"}, {3, "three"}})
        assert.is_true(list:delete(2))
        assert.is_false(list:remove(2))
        assert.is_false(list:delete(nil))
        assert.are.same({1, 3}, list:to_list())
        assert.equals(2, list:size())
        assert.is_true(list:is_valid_skip_list())
    end)
end)

describe("SkipList order queries", function()
    it("iterates in sorted order and returns defensive snapshots", function()
        local list = SkipList.from_entries({{5, "e"}, {1, "a"}, {3, "c"}, {9, "i"}})
        assert.are.same({1, 3, 5, 9}, list:to_sorted_array())
        assert.are.same({{1, "a"}, {3, "c"}, {5, "e"}, {9, "i"}}, list:entries())

        local iterated = {}
        for key, value in list:iterator() do
            iterated[#iterated + 1] = key .. value
        end
        assert.are.same({"1a", "3c", "5e", "9i"}, iterated)

        local snapshot = list:to_list()
        snapshot[1] = 999
        assert.are.same({1, 3, 5, 9}, list:to_list())
        assert.equals("SkipList([1, 3, 5, 9])", tostring(list))
    end)

    it("supports boundaries, zero-based rank, and selection", function()
        local list = SkipList.from_entries({{10, 1}, {20, 2}, {30, 3}, {40, 4}})
        assert.equals(10, list:min())
        assert.equals(40, list:max())
        assert.equals(0, list:rank(10))
        assert.equals(2, list:rank(30))
        assert.is_nil(list:rank(25))
        assert.equals(10, list:by_rank(0))
        assert.equals(40, list:by_rank(3))
        assert.is_nil(list:by_rank(-1))
        assert.is_nil(list:by_rank(4))
        assert.equals(30, list:kth_smallest(3))
        assert.is_nil(list:kth_smallest(0))
    end)

    it("returns inclusive and exclusive entry ranges", function()
        local list = SkipList.from_entries({{5, 50}, {12, 120}, {20, 200}, {37, 370}, {42, 420}})
        assert.are.same({{12, 120}, {20, 200}, {37, 370}}, list:range_query(12, 37))
        assert.are.same({{20, 200}}, list:range(12, 37, false))
        assert.are.same({}, list:range(50, 10))
        assert.are.same({}, list:range(100, 200))
    end)

    it("handles the empty state", function()
        local list = SkipList.new()
        assert.is_true(list:is_empty())
        assert.is_nil(list:min())
        assert.is_nil(list:max())
        assert.is_nil(list:by_rank(0))
        assert.are.same({}, list:entries())
        assert.equals("SkipList([])", tostring(list))
        assert.is_true(list:is_valid_skip_list())
    end)
end)

describe("SkipList configuration and invariants", function()
    it("supports custom comparators", function()
        local function by_length(left, right)
            if #left ~= #right then
                return #left - #right
            end
            return module.default_compare(left, right)
        end
        local list = SkipList.new(8, 0.5, by_length, 9)
        list:insert("banana", 6)
        list:insert("fig", 3)
        list:insert("apple", 5)

        assert.are.same({"fig", "apple", "banana"}, list:to_list())
        assert.equals("apple", list:by_rank(1))
        assert.is_true(list:is_valid_skip_list())
    end)

    it("stores deterministic skip-list parameters", function()
        local list = SkipList.new(8, 0.75, nil, 42)
        assert.equals(8, list:max_level())
        assert.equals(0.75, list:probability())
        for value = 1, 50 do
            list:insert(value, value * 2)
        end
        assert.is_true(list:current_level() >= 1)
        assert.is_true(list:current_level() <= 8)
        assert.is_true(list:is_valid_skip_list())
    end)

    it("preserves spans through a larger mutation sequence", function()
        local list = SkipList.new(16, 0.5, nil, 7)
        local expected = {}
        for index = 1, 200 do
            local key = (index * 73) % 211
            list:insert(key, index)
            expected[key] = true
            assert.is_true(list:is_valid_skip_list())
        end
        for key = 1, 209, 3 do
            list:delete(key)
            expected[key] = nil
            assert.is_true(list:is_valid_skip_list())
        end

        local sorted = {}
        for key in pairs(expected) do
            sorted[#sorted + 1] = key
        end
        table.sort(sorted)
        assert.are.same(sorted, list:to_list())
        for rank, key in ipairs(sorted) do
            assert.equals(rank - 1, list:rank(key))
            assert.equals(key, list:by_rank(rank - 1))
        end
    end)

    it("validates public inputs", function()
        assert.has_error(function() SkipList.new(0) end, "max_level must be a positive integer")
        assert.has_error(function() SkipList.new(4, 1) end, "probability must be between 0 and 1")
        assert.has_error(function() SkipList.new(4, 0.5, "compare") end, "compare must be a function")
        assert.has_error(function() SkipList.new(4, 0.5, nil, 1.5) end, "seed must be an integer")
        assert.has_error(function() SkipList.new():insert(nil) end, "key must not be nil")
        assert.has_error(function() SkipList.from_entries("entries") end, "entries must be a table")
        assert.has_error(function() SkipList.new():range(nil, 1) end, "minimum must not be nil")
        assert.has_error(function() SkipList.new():range(1, nil) end, "maximum must not be nil")
        assert.has_error(function() SkipList.new():range(1, 2, "yes") end, "inclusive must be a boolean")
    end)
end)
