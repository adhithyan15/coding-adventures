-- tests/test_b_plus_tree.lua — Comprehensive tests for the B+ Tree (DT12)
-- =========================================================================
--
-- Uses the Busted test framework.  Run from the tests/ directory:
--   LUA_PATH="../src/?.lua;../src/?/init.lua;;" busted . --verbose --pattern=test_

package.path = "../src/?.lua;../src/?/init.lua;" .. package.path

local BPlusTree = require("coding_adventures.b_plus_tree")

-- ============================================================================
-- Helpers
-- ============================================================================

--- Assert that tree:is_valid() returns true.
local function check(tree, label)
    assert.is_true(tree:is_valid(), "is_valid: " .. (label or ""))
end

--- Assert that full_scan() returns a sorted list with the right count.
local function check_list(tree, label)
    local all  = tree:full_scan()
    local size = tree:size()
    assert.equals(size, #all, "list count == size: " .. (label or ""))
    for i = 2, #all do
        assert.is_true(
            all[i][1] > all[i - 1][1],
            "linked list out of order at index " .. i .. ": " .. (label or "")
        )
    end
end

-- ============================================================================
-- 1. Empty tree
-- ============================================================================
describe("empty tree", function()
    it("has size 0", function()
        local t = BPlusTree.new()
        assert.equals(0, t:size())
    end)
    it("has height 0", function()
        local t = BPlusTree.new()
        assert.equals(0, t:height())
    end)
    it("search returns nil", function()
        local t = BPlusTree.new()
        assert.is_nil(t:search(42))
    end)
    it("min_key returns nil", function()
        local t = BPlusTree.new()
        assert.is_nil(t:min_key())
    end)
    it("max_key returns nil", function()
        local t = BPlusTree.new()
        assert.is_nil(t:max_key())
    end)
    it("full_scan returns empty list", function()
        local t = BPlusTree.new()
        assert.same({}, t:full_scan())
    end)
    it("range_scan returns empty list", function()
        local t = BPlusTree.new()
        assert.same({}, t:range_scan(1, 10))
    end)
    it("delete returns false", function()
        local t = BPlusTree.new()
        assert.is_false(t:delete(1))
    end)
    it("is_valid", function()
        local t = BPlusTree.new()
        check(t, "empty")
    end)
end)

-- ============================================================================
-- 2. Basic insert / search (t=2)
-- ============================================================================
describe("basic insert and search t=2", function()
    local t
    before_each(function()
        t = BPlusTree.new({ t = 2 })
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
    it("linked list intact", function()
        check_list(t, "after inserts")
    end)
end)

-- ============================================================================
-- 3. Upsert — inserting an existing key updates the value
-- ============================================================================
describe("upsert", function()
    it("updates value without changing size", function()
        local t = BPlusTree.new()
        t:insert(1, "one")
        t:insert(1, "ONE")
        assert.equals(1,     t:size())
        assert.equals("ONE", t:search(1))
        check(t, "upsert")
    end)
end)

-- ============================================================================
-- 4. Full scan is sorted
-- ============================================================================
describe("full_scan sorted t=3", function()
    it("returns all pairs in ascending order", function()
        local t = BPlusTree.new({ t = 3 })
        for _, k in ipairs({50, 10, 80, 30, 60, 5, 25, 55, 75, 90}) do
            t:insert(k, k * 2)
        end
        local pairs = t:full_scan()
        assert.equals(10, #pairs)
        for i = 2, #pairs do
            assert.is_true(pairs[i][1] > pairs[i - 1][1])
        end
        for _, p in ipairs(pairs) do
            assert.equals(p[1] * 2, p[2])
        end
        check(t,      "full_scan")
        check_list(t, "full_scan")
    end)
end)

-- ============================================================================
-- 5. Range scan — uses the leaf linked list
-- ============================================================================
describe("range_scan", function()
    local t
    before_each(function()
        t = BPlusTree.new()
        for i = 1, 20 do t:insert(i, "v" .. i) end
    end)
    it("range 5..10", function()
        local r = t:range_scan(5, 10)
        assert.equals(6, #r)
        for j, p in ipairs(r) do
            assert.equals(4 + j, p[1])
            assert.equals("v" .. p[1], p[2])
        end
    end)
    it("empty range returns empty list", function()
        assert.same({}, t:range_scan(100, 200))
    end)
    it("single-element range", function()
        local r = t:range_scan(7, 7)
        assert.equals(1, #r)
        assert.equals(7, r[1][1])
    end)
    it("is_valid", function()
        check(t, "range_scan")
    end)
    it("linked list intact", function()
        check_list(t, "range_scan")
    end)
end)

-- ============================================================================
-- 6. Linked list integrity after inserts
-- ============================================================================
describe("linked list integrity after inserts", function()
    it("maintains list at every insert step", function()
        local t = BPlusTree.new({ t = 2 })
        for _, k in ipairs({30, 10, 50, 5, 20, 40, 60, 15, 25, 35, 45, 55}) do
            t:insert(k, k)
            check(t,      "insert " .. k)
            check_list(t, "insert " .. k)
        end
    end)
end)

-- ============================================================================
-- 7. Delete — leaf deletion
-- ============================================================================
describe("delete leaf", function()
    it("removes key and updates size", function()
        local t = BPlusTree.new()
        for _, k in ipairs({10, 20, 5, 15, 25}) do t:insert(k, k) end
        assert.is_true(t:delete(5))
        assert.equals(4, t:size())
        assert.is_nil(t:search(5))
        assert.is_false(t:delete(99))
        check(t,      "delete leaf")
        check_list(t, "delete leaf")
    end)
end)

-- ============================================================================
-- 8. Delete until empty
-- ============================================================================
describe("delete until empty", function()
    it("maintains validity at every step", function()
        local t = BPlusTree.new()
        for k = 1, 10 do t:insert(k, k) end
        for k = 1, 10 do
            t:delete(k)
            check(t,      "after delete " .. k)
            check_list(t, "after delete " .. k)
        end
        assert.equals(0, t:size())
        assert.is_nil(t:min_key())
        assert.is_nil(t:max_key())
    end)
end)

-- ============================================================================
-- 9. Delete all cases (1..30 then specific order)
-- ============================================================================
describe("delete all cases", function()
    it("handles all deletion sub-cases", function()
        local t = BPlusTree.new({ t = 2 })
        for k = 1, 30 do t:insert(k, k) end
        check(t, "after 30 inserts")

        local order = {15, 1, 30, 10, 20, 5, 25, 8, 22, 3, 17, 28}
        for _, k in ipairs(order) do
            assert.is_true(t:delete(k), "delete " .. k)
            assert.is_nil(t:search(k),  "key " .. k .. " gone")
            check(t,      "after delete " .. k)
            check_list(t, "after delete " .. k)
        end
        assert.equals(30 - 12, t:size())
    end)
end)

-- ============================================================================
-- 10. Large-scale — 500 keys, t=2
-- ============================================================================
describe("large scale t=2 (500 keys)", function()
    it("insert, search, and delete are correct", function()
        local t = BPlusTree.new({ t = 2 })
        local keys = {}
        for i = 1, 500 do keys[i] = i end
        for i = 500, 2, -1 do
            local j = math.random(i)
            keys[i], keys[j] = keys[j], keys[i]
        end
        for _, k in ipairs(keys) do t:insert(k, k * 3) end
        assert.equals(500, t:size())
        check(t,      "500 inserts")
        check_list(t, "500 inserts")
        assert.equals(1,   t:min_key())
        assert.equals(500, t:max_key())
        for k = 1, 500 do
            assert.equals(k * 3, t:search(k))
        end
        for k = 1, 500, 2 do t:delete(k) end
        assert.equals(250, t:size())
        check(t,      "after deleting 250 keys")
        check_list(t, "after deleting 250 keys")
    end)
end)

-- ============================================================================
-- 11. Large-scale — 600 keys, t=3
-- ============================================================================
describe("large scale t=3 (600 keys)", function()
    it("full_scan and range_scan are correct", function()
        local t = BPlusTree.new({ t = 3 })
        for k = 1, 600 do t:insert(k, k) end
        assert.equals(600, t:size())
        check(t, "600 inserts")

        local all = t:full_scan()
        assert.equals(600, #all)
        for i, p in ipairs(all) do assert.equals(i, p[1]) end

        local r = t:range_scan(200, 400)
        assert.equals(201, #r)
        assert.equals(200, r[1][1])
        assert.equals(400, r[#r][1])

        for k = 2, 600, 2 do t:delete(k) end
        assert.equals(300, t:size())
        check(t,      "after halving")
        check_list(t, "after halving")
    end)
end)

-- ============================================================================
-- 12. Large-scale — 750 keys, t=5
-- ============================================================================
describe("large scale t=5 (750 keys)", function()
    it("range_scan is correct", function()
        local t = BPlusTree.new({ t = 5 })
        for k = 1, 750 do t:insert(k, "key" .. k) end
        assert.equals(750, t:size())
        check(t,      "750 inserts")
        check_list(t, "750 inserts")

        local r = t:range_scan(100, 200)
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
        local t = BPlusTree.new()
        for _, k in ipairs({7, 3, 11, 1, 5, 9, 13}) do t:insert(k, k) end
        assert.equals(1,  t:min_key())
        assert.equals(13, t:max_key())
        t:delete(1)
        assert.equals(3,  t:min_key())
        t:delete(13)
        assert.equals(11, t:max_key())
        check(t,      "min max")
        check_list(t, "min max")
    end)
end)

-- ============================================================================
-- 14. Reverse-order insert
-- ============================================================================
describe("reverse order insert t=3", function()
    it("produces sorted full_scan", function()
        local t = BPlusTree.new({ t = 3 })
        for k = 300, 1, -1 do t:insert(k, k) end
        assert.equals(300, t:size())
        check(t,      "reverse inserts")
        check_list(t, "reverse inserts")
        local pairs = t:full_scan()
        assert.equals(300, #pairs)
        for i, p in ipairs(pairs) do assert.equals(i, p[1]) end
    end)
end)

-- ============================================================================
-- 15. Insert → delete all → re-insert
-- ============================================================================
describe("insert delete reinsert", function()
    it("recovers correctly after full deletion", function()
        local t = BPlusTree.new()
        for k = 1, 50 do t:insert(k, k) end
        for k = 1, 50 do t:delete(k) end
        assert.equals(0, t:size())
        check(t,      "after all deletions")
        check_list(t, "after all deletions")

        for k = 51, 100 do t:insert(k, k) end
        assert.equals(50,  t:size())
        check(t,      "after reinsert")
        check_list(t, "after reinsert")
        assert.equals(51,  t:min_key())
        assert.equals(100, t:max_key())
    end)
end)

-- ============================================================================
-- 16. Cross-leaf range scan
-- ============================================================================
describe("cross-leaf range scan", function()
    it("correctly spans multiple leaf nodes", function()
        local t = BPlusTree.new({ t = 2 })
        for i = 1, 20 do t:insert(i * 10, i * 10) end
        local r = t:range_scan(30, 150)
        assert.equals(13, #r)   -- 30,40,50,60,70,80,90,100,110,120,130,140,150
        assert.equals(30,  r[1][1])
        assert.equals(150, r[#r][1])
        check(t, "cross-leaf range")
    end)
end)
