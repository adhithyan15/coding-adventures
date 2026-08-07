package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local module = require("coding_adventures.fenwick_tree")
local FenwickTree = module.FenwickTree

describe("FenwickTree construction", function()
    it("exposes the package version and empty-tree state", function()
        assert.equals("0.1.0", module.VERSION)
        local tree = FenwickTree.new(0)
        assert.equals(0, tree:len())
        assert.equals(0, tree:size())
        assert.is_true(tree:is_empty())
        assert.are.same({}, tree:bit_array())
    end)

    it("rejects invalid sizes and inputs", function()
        assert.has_error(function() FenwickTree.new(-1) end, "size must be non-negative")
        assert.has_error(function() FenwickTree.new(1.5) end, "size must be an integer")
        assert.has_error(function() FenwickTree.from_list("nope") end, "values must be a table")
        assert.has_error(function() FenwickTree.from_list({1, "x"}) end, "value at index 2 must be a number")
    end)
end)

describe("FenwickTree queries and updates", function()
    it("matches the reference prefix, range, and point vectors", function()
        local tree = FenwickTree.from_list({3, 2, 1, 7, 4})
        assert.equals(5, tree:len())
        assert.is_false(tree:is_empty())
        assert.are.same({3, 5, 6, 13, 17}, {
            tree:prefix_sum(1),
            tree:prefix_sum(2),
            tree:prefix_sum(3),
            tree:prefix_sum(4),
            tree:prefix_sum(5),
        })
        assert.equals(0, tree:prefix_sum(0))
        assert.equals(10, tree:range_sum(2, 4))
        assert.equals(17, tree:range_sum(1, 5))
        assert.equals(7, tree:point_query(4))
    end)

    it("supports integer, negative, and floating-point updates", function()
        local tree = FenwickTree.from_list({5, -2, 7, 1.5, 4.5})
        assert.equals(16, tree:prefix_sum(5))
        assert.equals(6.5, tree:range_sum(2, 4))
        assert.equals(tree, tree:update(2, 4.5))
        assert.equals(2.5, tree:point_query(2))
        assert.equals(20.5, tree:prefix_sum(5))
    end)

    it("returns a defensive BIT-array copy and renders itself", function()
        local tree = FenwickTree.from_list({1, 2, 3})
        local bit = tree:bit_array()
        bit[1] = 99
        assert.equals(1, tree:point_query(1))
        assert.is_truthy(tostring(tree):match("FenwickTree"))
    end)
end)

describe("FenwickTree order statistics", function()
    it("finds the first prefix that reaches each target", function()
        local tree = FenwickTree.from_list({1, 2, 3, 4, 5})
        assert.equals(1, tree:find_kth(1))
        assert.equals(2, tree:find_kth(2))
        assert.equals(2, tree:find_kth(3))
        assert.equals(3, tree:find_kth(4))
        assert.equals(4, tree:find_kth(10))
        assert.equals(5, tree:find_kth(15))
    end)

    it("rejects invalid targets", function()
        assert.has_error(function() FenwickTree.new(0):find_kth(1) end, "find_kth called on empty tree")
        local tree = FenwickTree.from_list({1, 2, 3})
        assert.has_error(function() tree:find_kth(0) end, "target must be positive")
        assert.has_error(function() tree:find_kth(7) end, "target exceeds total sum")
    end)
end)

describe("FenwickTree validation", function()
    it("rejects invalid indices and ranges", function()
        local tree = FenwickTree.from_list({1, 2, 3})
        assert.has_error(function() tree:prefix_sum(-1) end, "prefix index -1 out of range [0, 3]")
        assert.has_error(function() tree:prefix_sum(4) end, "prefix index 4 out of range [0, 3]")
        assert.has_error(function() tree:update(0, 1) end, "index 0 out of range [1, 3]")
        assert.has_error(function() tree:update(1, "x") end, "delta must be a number")
        assert.has_error(function() tree:range_sum(3, 1) end, "left must be <= right")
        assert.has_error(function() tree:range_sum(0, 2) end, "index 0 out of range [1, 3]")
        assert.has_error(function() tree:point_query(4) end, "index 4 out of range [1, 3]")
    end)
end)
