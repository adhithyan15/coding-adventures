-- tests/test_b_tree.lua — Comprehensive tests for the B-Tree (DT11)
-- =================================================================
--
-- Uses the Busted test framework.  Run from the tests/ directory:
--   LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local BTree = require("coding_adventures.b_tree")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Assert that tree.is_valid() returns true.
local function check(tree, label)
    assert.is_true(tree:is_valid(), "is_valid: " .. (label or ""))
end

--- Assert that inorder returns a sorted array of keys.
local function assert_sorted(pairs_list, label)
    for i = 2, #pairs_list do
        assert.is_true(
            pairs_list[i][1] > pairs_list[i - 1][1],
            "out of order at index " .. i .. ": " .. (label or "")
        )
    end
end

-- ============================================================================
-- 1. Empty tree
-- ============================================================================
describe("empty tree", function()
    it("has size 0", function()
        local t = BTree.new()
        assert.equals(0, t:size())
    end)
    it("has height 0", function()
        local t = BTree.new()
        assert.equals(0, t:height())
    end)
    it("search returns nil", function()
        local t = BTree.new()
        assert.is_nil(t:search("x"))
    end)
    it("min_key returns nil", function()
        local t = BTree.new()
        assert.is_nil(t:min_key())
    end)
    it("max_key returns nil", function()
        local t = BTree.new()
        assert.is_nil(t:max_key())
    end)
    it("inorder returns empty list", function()
        local t = BTree.new()
        assert.same({}, t:inorder())
    end)
    it("delete returns false", function()
        local t = BTree.new()
        assert.is_false(t:delete(42))
    end)
    it("is_valid", function()
        local t = BTree.new()
        check(t, "empty")
    end)
end)

-- ============================================================================
-- 2. Basic insert / search (t=2)
-- ============================================================================
describe("basic insert and search t=2", function()
    local t
    before_each(function()
        t = BTree.new({ t = 2 })
        local pairs = {{10,"ten"},{20,"twenty"},{5,"five"},{15,"fifteen"},{25,"twenty-five"}}
        for _, p in ipairs(pairs) do t:insert(p[1], p[2]) end
    end)
    it("size = 5", function()
        assert.equals(5, t:size())
    end)
    it("finds each inserted key", function()
        assert.equals("ten",         t:search(10))
        assert.equals("twenty",      t:search(20))
        assert.equals("five",        t:search(5))
        assert.equals("fifteen",     t:search(15))
        assert.equals("twenty-five", t:search(25))
    end)
    it("absent key returns nil", function()
        assert.is_nil(t:search(99))
    end)
    it("min_key = 5", function()
        assert.equals(5, t:min_key())
    end)
    it("max_key = 25", function()
        assert.equals(25, t:max_key())
    end)
    it("is_valid", function()
        check(t, "after inserts")
    end)
end)

-- ============================================================================
-- 3. Upsert
-- ============================================================================
describe("upsert", function()
    it("updates value for existing key", function()
        local t = BTree.new()
        t:insert(1, "one")
        t:insert(1, "ONE")
        assert.equals(1, t:size())
        assert.equals("ONE", t:search(1))
        check(t, "upsert")
    end)
end)

-- ============================================================================
-- 4. Inorder traversal is sorted
-- ============================================================================
describe("inorder sorted t=3", function()
    it("returns pairs in ascending order", function()
        local t = BTree.new({ t = 3 })
        for _, k in ipairs({50, 10, 80, 30, 60, 5, 25, 55, 75, 90}) do
            t:insert(k, k * 2)
        end
        local pairs = t:inorder()
        assert_sorted(pairs, "inorder")
        for _, p in ipairs(pairs) do
            assert.equals(p[1] * 2, p[2])
        end
        check(t, "inorder")
    end)
end)

-- ============================================================================
-- 5. Range query
-- ============================================================================
describe("range_query", function()
    local t
    before_each(function()
        t = BTree.new()
        for i = 1, 20 do t:insert(i, "v" .. i) end
    end)
    it("range 5..10", function()
        local r = t:range_query(5, 10)
        assert.equals(6, #r)
        for i, p in ipairs(r) do
            assert.equals(4 + i, p[1])
        end
    end)
    it("empty range returns empty list", function()
        assert.same({}, t:range_query(100, 200))
    end)
    it("single-element range", function()
        local r = t:range_query(7, 7)
        assert.equals(1, #r)
        assert.equals(7, r[1][1])
    end)
    it("is_valid", function()
        check(t, "range_query")
    end)
end)

-- ============================================================================
-- 6. Delete — leaf (Case A)
-- ============================================================================
describe("delete leaf", function()
    it("removes the key and returns true", function()
        local t = BTree.new()
        for _, k in ipairs({10, 20, 5, 15, 25}) do t:insert(k, k) end
        assert.is_true(t:delete(5))
        assert.equals(4, t:size())
        assert.is_nil(t:search(5))
        assert.is_false(t:delete(99))
        check(t, "delete leaf")
    end)
end)

-- ============================================================================
-- 7. Delete — internal node (Cases B1/B2/B3)
-- ============================================================================
describe("delete internal node", function()
    it("removes internal key correctly", function()
        local t = BTree.new({ t = 2 })
        for k = 1, 15 do t:insert(k, k) end
        check(t, "before internal delete")
        assert.is_true(t:delete(8))
        assert.is_nil(t:search(8))
        assert.equals(14, t:size())
        check(t, "after internal delete")
        t:delete(4); t:delete(12)
        check(t, "after more deletes")
    end)
end)

-- ============================================================================
-- 8. Delete all cases (1..30 then specific order)
-- ============================================================================
describe("delete all cases", function()
    it("handles all CLRS deletion sub-cases", function()
        local t = BTree.new({ t = 2 })
        for k = 1, 30 do t:insert(k, k) end
        check(t, "after 30 inserts")

        local order = {15, 1, 30, 10, 20, 5, 25, 8, 22, 3, 17, 28}
        for _, k in ipairs(order) do
            assert.is_true(t:delete(k), "delete " .. k)
            assert.is_nil(t:search(k),  "key " .. k .. " gone")
            check(t, "after delete " .. k)
        end
        assert.equals(30 - 12, t:size())
    end)
end)

-- ============================================================================
-- 9. Delete until empty
-- ============================================================================
describe("delete until empty", function()
    it("maintains validity at every step", function()
        local t = BTree.new()
        for k = 1, 10 do t:insert(k, k) end
        for k = 1, 10 do
            t:delete(k)
            check(t, "after delete " .. k)
        end
        assert.equals(0, t:size())
        assert.is_nil(t:min_key())
        assert.is_nil(t:max_key())
    end)
end)

-- ============================================================================
-- 10. Large-scale — 500 keys, t=2
-- ============================================================================
describe("large scale t=2 (500 keys)", function()
    it("insert, search, and delete are correct", function()
        local t = BTree.new({ t = 2 })
        -- Fisher-Yates shuffle.
        local keys = {}
        for i = 1, 500 do keys[i] = i end
        for i = 500, 2, -1 do
            local j = math.random(i)
            keys[i], keys[j] = keys[j], keys[i]
        end
        for _, k in ipairs(keys) do t:insert(k, k * 3) end
        assert.equals(500, t:size())
        check(t, "500 inserts")
        assert.equals(1,   t:min_key())
        assert.equals(500, t:max_key())
        for k = 1, 500 do
            assert.equals(k * 3, t:search(k))
        end
        -- Delete odd keys.
        for k = 1, 500, 2 do t:delete(k) end
        assert.equals(250, t:size())
        check(t, "after deleting 250 keys")
    end)
end)

-- ============================================================================
-- 11. Large-scale — 600 keys, t=3
-- ============================================================================
describe("large scale t=3 (600 keys)", function()
    it("inorder is 1..600 and deletion is correct", function()
        local t = BTree.new({ t = 3 })
        for k = 1, 600 do t:insert(k, k) end
        assert.equals(600, t:size())
        check(t, "600 inserts")

        local pairs = t:inorder()
        assert.equals(600, #pairs)
        for i, p in ipairs(pairs) do
            assert.equals(i, p[1])
        end

        -- Delete even keys.
        for k = 2, 600, 2 do t:delete(k) end
        assert.equals(300, t:size())
        check(t, "after halving")
    end)
end)

-- ============================================================================
-- 12. Large-scale — 750 keys, t=5
-- ============================================================================
describe("large scale t=5 (750 keys)", function()
    it("range query is correct", function()
        local t = BTree.new({ t = 5 })
        for k = 1, 750 do t:insert(k, "key" .. k) end
        assert.equals(750, t:size())
        check(t, "750 inserts")

        local r = t:range_query(100, 200)
        assert.equals(101, #r)
        assert.equals(100, r[1][1])
        assert.equals(200, r[#r][1])
    end)
end)

-- ============================================================================
-- 13. Min / max after deletions
-- ============================================================================
describe("min max after deletions", function()
    it("updates correctly", function()
        local t = BTree.new()
        for _, k in ipairs({7, 3, 11, 1, 5, 9, 13}) do t:insert(k, k) end
        assert.equals(1,  t:min_key())
        assert.equals(13, t:max_key())
        t:delete(1)
        assert.equals(3,  t:min_key())
        t:delete(13)
        assert.equals(11, t:max_key())
        check(t, "min max")
    end)
end)

-- ============================================================================
-- 14. Reverse-order insert
-- ============================================================================
describe("reverse order insert t=3", function()
    it("produces sorted inorder traversal", function()
        local t = BTree.new({ t = 3 })
        for k = 300, 1, -1 do t:insert(k, k) end
        assert.equals(300, t:size())
        check(t, "reverse inserts")
        local pairs = t:inorder()
        assert.equals(300, #pairs)
        for i, p in ipairs(pairs) do
            assert.equals(i, p[1])
        end
    end)
end)

-- ============================================================================
-- 15. Insert → delete all → re-insert
-- ============================================================================
describe("insert delete reinsert", function()
    it("recovers correctly after full deletion", function()
        local t = BTree.new()
        for k = 1, 50 do t:insert(k, k) end
        for k = 1, 50 do t:delete(k) end
        assert.equals(0, t:size())
        check(t, "after all deletions")
        for k = 51, 100 do t:insert(k, k) end
        assert.equals(50,  t:size())
        check(t, "after reinsert")
        assert.equals(51,  t:min_key())
        assert.equals(100, t:max_key())
    end)
end)
