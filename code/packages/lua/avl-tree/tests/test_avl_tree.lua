package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local module = require("coding_adventures.avl_tree")
local AVLNode = module.AVLNode
local AVLTree = module.AVLTree

describe("AVLTree balancing", function()
    it("performs left and right rotations", function()
        local right_heavy = AVLTree.from_values({10, 20, 30})
        local left_heavy = AVLTree.from_values({30, 20, 10})

        assert.equals(20, right_heavy.root.value)
        assert.equals(20, left_heavy.root.value)
        assert.are.same({10, 20, 30}, right_heavy:to_sorted_array())
        assert.is_true(right_heavy:is_valid_bst())
        assert.is_true(right_heavy:is_valid_avl())
        assert.equals(1, right_heavy:height())
        assert.equals(3, right_heavy:size())
        assert.equals(0, right_heavy:balance_factor(right_heavy.root))
    end)

    it("performs both double rotations", function()
        local left_right = AVLTree.from_values({30, 10, 20})
        local right_left = AVLTree.from_values({10, 30, 20})

        assert.equals(20, left_right.root.value)
        assert.equals(20, right_left.root.value)
        assert.is_true(left_right:is_valid_avl())
        assert.is_true(right_left:is_valid_avl())
    end)
end)

describe("AVLTree persistence and queries", function()
    it("searches and computes order statistics", function()
        local tree = AVLTree.from_values({40, 20, 60, 10, 30, 50, 70})

        assert.equals(20, tree:search(20).value)
        assert.is_true(tree:contains(50))
        assert.equals(10, tree:min_value())
        assert.equals(70, tree:max_value())
        assert.equals(30, tree:predecessor(40))
        assert.equals(50, tree:successor(40))
        assert.equals(40, tree:kth_smallest(4))
        assert.equals(3, tree:rank(35))

        local deleted = tree:delete(20)
        assert.is_false(deleted:contains(20))
        assert.is_true(deleted:is_valid_avl())
        assert.is_true(tree:contains(20))
    end)

    it("handles empty trees, duplicates, and absent values", function()
        local empty = AVLTree.empty()
        assert.is_nil(empty:search(1))
        assert.is_nil(empty:min_value())
        assert.is_nil(empty:max_value())
        assert.is_nil(empty:predecessor(1))
        assert.is_nil(empty:successor(1))
        assert.is_nil(empty:kth_smallest(0))
        assert.equals(0, empty:rank(1))
        assert.equals(0, empty:balance_factor(nil))
        assert.equals(-1, empty:height())
        assert.equals(0, empty:size())
        assert.is_true(empty:is_valid_avl())
        assert.equals("AVLTree(root=nil, height=-1, size=0)", tostring(empty))

        local tree = AVLTree.from_values({30, 20, 40, 10, 25, 35, 50})
        assert.are.same(tree:to_sorted_array(), tree:insert(25):to_sorted_array())
        assert.are.same(tree:to_sorted_array(), tree:delete(999):to_sorted_array())
    end)

    it("deletes a root with a nested successor", function()
        local tree = AVLTree.from_values({5, 3, 8, 7, 9, 6})
        local deleted = tree:delete(5)

        assert.are.same({3, 6, 7, 8, 9}, deleted:to_sorted_array())
        assert.is_true(deleted:is_valid_avl())
        assert.equals(3, tree:kth_smallest(1))
        assert.equals(9, tree:kth_smallest(6))
        assert.equals(1, tree:rank(5))
        assert.are.same({2}, AVLTree.from_values({1, 2}):delete(1):to_sorted_array())
        assert.are.same({1}, AVLTree.from_values({2, 1}):delete(2):to_sorted_array())
    end)

    it("maintains invariants across a larger update sequence", function()
        local tree = AVLTree.empty()
        for index = 1, 100 do
            tree = tree:insert((index * 37) % 101)
            assert.is_true(tree:is_valid_avl())
        end
        assert.equals(100, tree:size())

        local updated = tree
        for value = 1, 99, 2 do
            updated = updated:delete(value)
            assert.is_true(updated:is_valid_avl())
        end

        local expected = {}
        for value = 2, 100, 2 do
            expected[#expected + 1] = value
        end
        assert.are.same(expected, updated:to_sorted_array())
        assert.equals(100, tree:size())
    end)
end)

describe("AVLTree validation and comparison", function()
    it("detects ordering, metadata, and balance corruption", function()
        local bad_order = AVLTree.new(AVLNode.new(5, AVLNode.new(6), nil, 1, 2))
        local bad_right_order = AVLTree.new(AVLNode.new(5, nil, AVLNode.new(4), 1, 2))
        local bad_height = AVLTree.new(AVLNode.new(5, AVLNode.new(3), nil, 99, 2))
        local bad_size = AVLTree.new(AVLNode.new(5, AVLNode.new(3), nil, 1, 99))
        local unbalanced_left = AVLNode.new(2, AVLNode.new(1), nil)
        local unbalanced = AVLTree.new(AVLNode.new(3, unbalanced_left, nil))

        assert.is_false(bad_order:is_valid_bst())
        assert.is_false(bad_order:is_valid_avl())
        assert.is_false(bad_right_order:is_valid_bst())
        assert.is_false(bad_right_order:is_valid_avl())
        assert.is_false(bad_height:is_valid_avl())
        assert.is_false(bad_size:is_valid_avl())
        assert.is_false(unbalanced:is_valid_avl())
    end)

    it("supports custom comparators", function()
        local function by_length(left, right)
            return #left - #right
        end
        local tree = AVLTree.from_values({"bbb", "a", "cc"}, by_length)

        assert.are.same({"a", "cc", "bbb"}, tree:to_sorted_array())
        assert.is_true(tree:contains("zz"))
        assert.equals("a", tree:predecessor("zz"))
        assert.equals("bbb", tree:successor("zz"))
        assert.is_true(tree:is_valid_avl())
    end)

    it("validates construction inputs", function()
        assert.has_error(function() AVLNode.new(nil) end, "node value must not be nil")
        assert.has_error(function() AVLNode.new(1, "left") end, "left must be an AVLNode or nil")
        assert.has_error(function() AVLNode.new(1, nil, nil, -1) end, "height must be a non-negative integer")
        assert.has_error(function() AVLNode.new(1, nil, nil, nil, -1) end, "size must be a non-negative integer")
        assert.has_error(function() AVLTree.new("root") end, "root must be an AVLNode or nil")
        assert.has_error(function() AVLTree.new(nil, "compare") end, "compare must be a function")
        assert.has_error(function() AVLTree.from_values("values") end, "values must be a table")
        assert.has_error(function() AVLTree.empty():insert(nil) end, "value must not be nil")
        assert.has_error(function() AVLTree.empty():delete(nil) end, "value must not be nil")
        assert.has_error(function() AVLTree.empty():balance_factor("node") end, "node must be an AVLNode or nil")
    end)
end)
