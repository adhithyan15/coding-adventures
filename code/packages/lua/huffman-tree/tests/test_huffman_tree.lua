-- ============================================================================
-- Tests for the HuffmanTree implementation (DT27).
-- ============================================================================
--
-- Covers: build validation, code_table, canonical_code_table, decode_all,
-- weight, depth, symbol_count, leaves, is_valid, edge cases (single symbol,
-- two symbols, all equal weights), determinism.
--
-- Uses Busted test framework: https://olivinelabs.com/busted/

package.path = "../src/?.lua;../src/?/init.lua;../../heap/src/?.lua;../../heap/src/?/init.lua;" .. package.path

local HuffmanTree = require("coding_adventures.huffman_tree")

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- sorted_pairs returns an array of {key, value} sorted by key.
local function sorted_pairs(t)
    local keys = {}
    for k in pairs(t) do keys[#keys + 1] = k end
    table.sort(keys)
    local result = {}
    for _, k in ipairs(keys) do result[#result + 1] = {k, t[k]} end
    return result
end

-- ── Version ───────────────────────────────────────────────────────────────────

describe("huffman_tree module", function()
    it("has VERSION '0.1.0'", function()
        assert.equals("0.1.0", HuffmanTree.VERSION)
    end)

    it("exposes build function", function()
        assert.is_function(HuffmanTree.build)
    end)
end)

-- ── Build validation ──────────────────────────────────────────────────────────

describe("HuffmanTree.build validation", function()
    it("errors on empty weights", function()
        assert.has_error(function()
            HuffmanTree.build({})
        end)
    end)

    it("errors on nil weights", function()
        assert.has_error(function()
            HuffmanTree.build(nil)
        end)
    end)

    it("errors on zero frequency", function()
        assert.has_error(function()
            HuffmanTree.build({{65, 0}})
        end)
    end)

    it("errors on negative frequency", function()
        assert.has_error(function()
            HuffmanTree.build({{65, -1}})
        end)
    end)

    it("succeeds with one symbol", function()
        local tree = HuffmanTree.build({{65, 5}})
        assert.is_not_nil(tree)
    end)

    it("succeeds with many symbols", function()
        local weights = {}
        for i = 1, 20 do weights[i] = {i, i} end
        local tree = HuffmanTree.build(weights)
        assert.is_not_nil(tree)
    end)
end)

-- ── code_table ────────────────────────────────────────────────────────────────

describe("code_table", function()
    it("basic AAABBC example: A=3, B=2, C=1", function()
        -- Heap construction trace:
        --   Initial heap: C(67,w=1), B(66,w=2), A(65,w=3)
        --   Pop C(priority {1,0,67,huge}) and B(priority {2,0,66,huge}).
        --   Create Internal(w=3, left=C, right=B, order=0).
        --   Heap now: A(65,{3,0,65,huge}), Internal({3,1,huge,0}).
        --   Pop A (leaf wins tie at same weight), then Internal.
        --   Create root(w=6, left=A, right=Internal(C,B)).
        -- Tree shape:   [6]
        --              /   \
        --            A(3)  [3]
        --                 /   \
        --               C(1)  B(2)
        -- Codes: A → "0", C → "10", B → "11"
        local tree = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local tbl  = tree:code_table()
        assert.equals("0",  tbl[65])
        assert.equals("11", tbl[66])
        assert.equals("10", tbl[67])
    end)

    it("single symbol gets code '0'", function()
        local tree = HuffmanTree.build({{42, 7}})
        local tbl  = tree:code_table()
        assert.equals("0", tbl[42])
    end)

    it("two symbols: lower-frequency gets longer code", function()
        -- A(10) and B(1): A=0, B=1
        local tree = HuffmanTree.build({{65, 10}, {66, 1}})
        local tbl  = tree:code_table()
        -- One gets "0", the other gets "1" — 1-bit codes for 2 symbols
        assert.equals(1, #tbl[65])
        assert.equals(1, #tbl[66])
    end)

    it("codes are prefix-free (no code is a prefix of another)", function()
        local weights = {{1,5},{2,3},{3,2},{4,1},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local tbl     = tree:code_table()
        local codes   = {}
        for _, v in pairs(tbl) do codes[#codes+1] = v end
        for i = 1, #codes do
            for j = 1, #codes do
                if i ~= j then
                    local a, b = codes[i], codes[j]
                    -- a should not be a prefix of b
                    assert.is_false(b:sub(1, #a) == a,
                        string.format("'%s' is a prefix of '%s'", a, b))
                end
            end
        end
    end)

    it("all codes are distinct", function()
        local weights = {{10,5},{20,3},{30,2},{40,1}}
        local tree    = HuffmanTree.build(weights)
        local tbl     = tree:code_table()
        local seen    = {}
        for _, code in pairs(tbl) do
            assert.is_nil(seen[code], "duplicate code: " .. code)
            seen[code] = true
        end
    end)

    it("all symbols in code_table match input symbols", function()
        local inputs = {{10,5},{20,3},{30,2},{40,1}}
        local tree   = HuffmanTree.build(inputs)
        local tbl    = tree:code_table()
        for _, pair in ipairs(inputs) do
            assert.is_not_nil(tbl[pair[1]],
                "symbol " .. pair[1] .. " missing from code table")
        end
    end)
end)

-- ── code_for ──────────────────────────────────────────────────────────────────

describe("code_for", function()
    it("returns same code as code_table for known symbols", function()
        local tree = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local tbl  = tree:code_table()
        assert.equals(tbl[65], tree:code_for(65))
        assert.equals(tbl[66], tree:code_for(66))
        assert.equals(tbl[67], tree:code_for(67))
    end)

    it("returns nil for unknown symbol", function()
        local tree = HuffmanTree.build({{65, 3}, {66, 2}})
        assert.is_nil(tree:code_for(99))
    end)

    it("single-symbol tree returns '0'", function()
        local tree = HuffmanTree.build({{1, 1}})
        assert.equals("0", tree:code_for(1))
    end)
end)

-- ── canonical_code_table ──────────────────────────────────────────────────────

describe("canonical_code_table", function()
    it("AAABBC: same lengths as regular, canonical form", function()
        local tree     = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local canon    = tree:canonical_code_table()
        -- Tree gives: A=length 1, B=length 2, C=length 2.
        -- Sorted by (length, symbol): A(1), B(2), C(2).
        -- Canonical assignment: A→"0", B→"10", C→"11"
        assert.equals("0",  canon[65])
        assert.equals("10", canon[66])
        assert.equals("11", canon[67])
    end)

    it("single symbol → '0'", function()
        local tree  = HuffmanTree.build({{5, 10}})
        local canon = tree:canonical_code_table()
        assert.equals("0", canon[5])
    end)

    it("canonical codes preserve same lengths as tree codes", function()
        local weights = {{1,5},{2,3},{3,2},{4,1},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local regular = tree:code_table()
        local canon   = tree:canonical_code_table()
        for sym, code in pairs(regular) do
            assert.equals(#code, #canon[sym],
                "length mismatch for symbol " .. sym)
        end
    end)

    it("canonical codes are prefix-free", function()
        local weights = {{1,5},{2,3},{3,2},{4,1},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local canon   = tree:canonical_code_table()
        local codes   = {}
        for _, v in pairs(canon) do codes[#codes+1] = v end
        for i = 1, #codes do
            for j = 1, #codes do
                if i ~= j then
                    local a, b = codes[i], codes[j]
                    assert.is_false(b:sub(1, #a) == a)
                end
            end
        end
    end)

    it("canonical codes are all-zeros-indexed: shortest code has longest prefix of zeros", function()
        -- By canonical convention, the first code at any length is the smallest
        -- numeric value with that many bits.
        local tree  = HuffmanTree.build({{65, 10}, {66, 5}, {67, 3}, {68, 1}})
        local canon = tree:canonical_code_table()
        -- All code values should be valid binary strings.
        for _, code in pairs(canon) do
            assert.is_truthy(code:match("^[01]+$"), "invalid code: " .. code)
        end
    end)
end)

-- ── decode_all ────────────────────────────────────────────────────────────────

describe("decode_all", function()
    it("decodes A using its code '0'", function()
        -- A has code '0' (left child of root)
        local tree   = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local result = tree:decode_all("0", 1)
        assert.same({65}, result)
    end)

    it("decodes AABC from bit string", function()
        -- A='0', C='10', B='11'
        -- AABC = '0' + '0' + '11' + '10' = "001110"
        local tree   = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local result = tree:decode_all("001110", 4)
        assert.same({65, 65, 66, 67}, result)
    end)

    it("single-leaf tree: each '0' decodes to that symbol", function()
        local tree   = HuffmanTree.build({{42, 5}})
        local result = tree:decode_all("000", 3)
        assert.same({42, 42, 42}, result)
    end)

    it("encode then decode round-trip: AAABBC", function()
        local tree   = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        local tbl    = tree:code_table()
        -- Encode AABBC manually
        local message = {65, 65, 66, 66, 67}
        local bits    = ""
        for _, sym in ipairs(message) do bits = bits .. tbl[sym] end
        local decoded = tree:decode_all(bits, #message)
        assert.same(message, decoded)
    end)

    it("errors on exhausted bit stream", function()
        local tree = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
        -- B='10', C='11' — try to decode 5 but stream only has 2 symbols
        assert.has_error(function()
            tree:decode_all("1011", 5)
        end)
    end)

    it("decode 0 symbols from empty string", function()
        local tree   = HuffmanTree.build({{65, 1}})
        local result = tree:decode_all("", 0)
        assert.same({}, result)
    end)

    it("decode large sequence round-trip", function()
        local weights = {{1,10},{2,5},{3,3},{4,2},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local tbl     = tree:code_table()
        local message = {1,2,3,1,2,3,4,5,1,1,2,3}
        local bits    = ""
        for _, sym in ipairs(message) do bits = bits .. tbl[sym] end
        local decoded = tree:decode_all(bits, #message)
        assert.same(message, decoded)
    end)
end)

-- ── weight / depth / symbol_count ─────────────────────────────────────────────

describe("weight", function()
    it("equals sum of all frequencies", function()
        local tree = HuffmanTree.build({{65,3},{66,2},{67,1}})
        assert.equals(6, tree:weight())
    end)

    it("single symbol", function()
        local tree = HuffmanTree.build({{0, 100}})
        assert.equals(100, tree:weight())
    end)
end)

describe("depth", function()
    it("AAABBC: max depth = 2", function()
        local tree = HuffmanTree.build({{65,3},{66,2},{67,1}})
        assert.equals(2, tree:depth())
    end)

    it("single symbol: depth = 0", function()
        local tree = HuffmanTree.build({{1, 5}})
        assert.equals(0, tree:depth())
    end)

    it("two equal symbols: depth = 1", function()
        local tree = HuffmanTree.build({{1,1},{2,1}})
        assert.equals(1, tree:depth())
    end)
end)

describe("symbol_count", function()
    it("returns number of distinct symbols", function()
        local tree = HuffmanTree.build({{65,3},{66,2},{67,1}})
        assert.equals(3, tree:symbol_count())
    end)

    it("single symbol → 1", function()
        local tree = HuffmanTree.build({{7, 99}})
        assert.equals(1, tree:symbol_count())
    end)

    it("ten symbols", function()
        local weights = {}
        for i = 1, 10 do weights[i] = {i, i} end
        local tree = HuffmanTree.build(weights)
        assert.equals(10, tree:symbol_count())
    end)
end)

-- ── leaves ────────────────────────────────────────────────────────────────────

describe("leaves", function()
    it("returns {symbol, code} pairs in left-to-right order", function()
        local tree   = HuffmanTree.build({{65,3},{66,2},{67,1}})
        local lvs    = tree:leaves()
        -- In-order: left subtree first (A='0'), then right subtree (C='10', B='11').
        assert.equals(3,   #lvs)
        assert.equals(65,  lvs[1][1])
        assert.equals("0", lvs[1][2])
        -- C(67) is the left child of the internal node, B(66) is the right child.
        assert.equals(67,  lvs[2][1])
        assert.equals("10", lvs[2][2])
        assert.equals(66,  lvs[3][1])
        assert.equals("11", lvs[3][2])
    end)

    it("all symbols appear exactly once", function()
        local weights = {{1,5},{2,3},{3,2},{4,1},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local lvs     = tree:leaves()
        assert.equals(5, #lvs)
        local seen = {}
        for _, pair in ipairs(lvs) do
            assert.is_nil(seen[pair[1]])
            seen[pair[1]] = true
        end
    end)

    it("single symbol leaf", function()
        local tree = HuffmanTree.build({{99, 7}})
        local lvs  = tree:leaves()
        assert.equals(1,   #lvs)
        assert.equals(99,  lvs[1][1])
        assert.equals("0", lvs[1][2])
    end)
end)

-- ── is_valid ──────────────────────────────────────────────────────────────────

describe("is_valid", function()
    it("valid tree is_valid → true", function()
        local tree = HuffmanTree.build({{65,3},{66,2},{67,1}})
        assert.is_true(tree:is_valid())
    end)

    it("single symbol tree is valid", function()
        local tree = HuffmanTree.build({{1, 10}})
        assert.is_true(tree:is_valid())
    end)

    it("large tree is valid", function()
        local weights = {}
        for i = 1, 15 do weights[i] = {i, i * 2} end
        local tree = HuffmanTree.build(weights)
        assert.is_true(tree:is_valid())
    end)
end)

-- ── Determinism ───────────────────────────────────────────────────────────────

describe("determinism", function()
    it("same input always produces same codes", function()
        local weights = {{1,5},{2,3},{3,2},{4,1},{5,1}}
        local tree1   = HuffmanTree.build(weights)
        local tree2   = HuffmanTree.build(weights)
        local tbl1    = tree1:code_table()
        local tbl2    = tree2:code_table()
        for sym, code in pairs(tbl1) do
            assert.equals(code, tbl2[sym])
        end
    end)

    it("tie-breaking: equal weights — leaves before internal, lower symbol first", function()
        -- A(1), B(1), C(1), D(1): all equal.
        -- Heap pops A first (lowest symbol), then B → internal AB(2).
        -- Next: C(1), D(1) < AB(2), so pop C, D → internal CD(2).
        -- Then AB(2), CD(2) → root ABCD(4).
        -- A and B end up at depth 2; C and D at depth 2.
        local tree = HuffmanTree.build({{1,1},{2,1},{3,1},{4,1}})
        assert.equals(2, tree:depth())
        assert.is_true(tree:is_valid())
    end)

    it("canonical codes are deterministic across builds", function()
        local weights = {{1,5},{2,3},{3,2},{4,1}}
        local c1 = HuffmanTree.build(weights):canonical_code_table()
        local c2 = HuffmanTree.build(weights):canonical_code_table()
        for sym, code in pairs(c1) do
            assert.equals(code, c2[sym])
        end
    end)
end)

-- ── All-equal weights ─────────────────────────────────────────────────────────

describe("all equal weights", function()
    it("two equal symbols: depth 1", function()
        local tree = HuffmanTree.build({{1,1},{2,1}})
        assert.equals(1, tree:depth())
        assert.is_true(tree:is_valid())
    end)

    it("four equal symbols: depth 2", function()
        local tree = HuffmanTree.build({{1,1},{2,1},{3,1},{4,1}})
        assert.equals(2, tree:depth())
        assert.is_true(tree:is_valid())
    end)

    it("eight equal symbols: depth 3", function()
        local tree = HuffmanTree.build({{1,1},{2,1},{3,1},{4,1},{5,1},{6,1},{7,1},{8,1}})
        assert.equals(3, tree:depth())
        assert.is_true(tree:is_valid())
    end)
end)

-- ── Full round-trip encode/decode ─────────────────────────────────────────────

describe("full round-trip", function()
    it("AAABBCC encode/decode round-trip", function()
        local tree    = HuffmanTree.build({{65,3},{66,2},{67,1}})
        local tbl     = tree:code_table()
        local message = {65, 65, 65, 66, 66, 67}
        local bits    = ""
        for _, sym in ipairs(message) do bits = bits .. tbl[sym] end
        local decoded = tree:decode_all(bits, #message)
        assert.same(message, decoded)
    end)

    it("five symbols round-trip", function()
        local weights = {{1,10},{2,5},{3,3},{4,2},{5,1}}
        local tree    = HuffmanTree.build(weights)
        local tbl     = tree:code_table()
        local message = {1,2,3,4,5,1,1,3,2}
        local bits    = ""
        for _, sym in ipairs(message) do bits = bits .. tbl[sym] end
        local decoded = tree:decode_all(bits, #message)
        assert.same(message, decoded)
    end)

    it("byte-range symbols (0-255) round-trip", function()
        local weights = {}
        for i = 0, 15 do
            weights[i + 1] = {i, i + 1}
        end
        local tree    = HuffmanTree.build(weights)
        local tbl     = tree:code_table()
        local message = {0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15}
        local bits    = ""
        for _, sym in ipairs(message) do bits = bits .. tbl[sym] end
        local decoded = tree:decode_all(bits, #message)
        assert.same(message, decoded)
    end)
end)
