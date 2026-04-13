-- ============================================================================
-- CodingAdventures.HuffmanTree
-- ============================================================================
--
-- DT27: Huffman Tree — Optimal prefix-free entropy coding
-- Part of the DT (Data Structures and Trees) series in coding-adventures.
--
-- What Is a Huffman Tree?
-- -----------------------
--
-- A Huffman tree is a full binary tree (every internal node has exactly two
-- children) built so that each symbol gets a unique variable-length bit code.
-- Symbols that appear often get short codes; symbols that appear rarely get
-- long codes. The total bits needed to encode a message is minimised — it is
-- the theoretically optimal prefix-free code for a given symbol frequency
-- distribution.
--
-- Think of it like Morse code. In Morse, "E" is "." (one dot) and "Z" is
-- "--.." (four symbols). The designers knew "E" is the most common letter in
-- English so they gave it the shortest code. Huffman's algorithm does this
-- automatically and optimally for any alphabet with any frequency distribution.
--
-- Algorithm: Greedy construction via min-heap
-- -------------------------------------------
--
-- 1. Create one leaf node per distinct symbol, each with its frequency as its
--    weight. Push all leaves onto a min-heap keyed by priority tuple.
--
-- 2. While the heap has more than one node:
--      a. Pop the two nodes with the smallest weight.
--      b. Create a new internal node whose weight = sum of the two children.
--      c. Set left = the first popped node, right = the second popped node.
--      d. Push the new internal node back onto the heap.
--
-- 3. The one remaining node is the root of the Huffman tree.
--
-- Tie-breaking rules (for deterministic output across implementations):
--   1. Lowest weight pops first.
--   2. Leaf nodes have higher priority than internal nodes at equal weight
--      ("leaf-before-internal" rule).
--   3. Among leaves of equal weight, lower symbol value wins.
--   4. Among internal nodes of equal weight, earlier-created node wins
--      (insertion-order FIFO).
--
-- Prefix-Free Property: Why It Works
-- ------------------------------------
--
-- Symbols live ONLY at the leaves, never at internal nodes. The code for a
-- symbol is the path from root to its leaf (left edge = '0', right edge = '1').
--
-- Since one leaf is never an ancestor of another leaf, no code can be a prefix
-- of another code. This is the prefix-free property, and it means the bit
-- stream can be decoded unambiguously without separator characters: just walk
-- the tree bit by bit until you hit a leaf.
--
-- Canonical Codes (DEFLATE / zlib style)
-- ----------------------------------------
--
-- The standard tree-walk produces valid codes, but different tree shapes can
-- produce different codes for the same symbol lengths. Canonical codes normalise
-- this: given only the code *lengths*, you can reconstruct the exact canonical
-- code table without transmitting the tree structure.
--
-- Algorithm:
--   1. Collect (symbol, code_length) pairs from the tree.
--   2. Sort by (code_length, symbol_value).
--   3. Assign codes numerically:
--        code[0] = 0 (left-padded to length[0] bits)
--        code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
--
-- This is exactly what DEFLATE uses: the compressed stream contains only the
-- length table, not the tree, saving space.
--
-- Example with AAABBC:
--   A: weight=3, B: weight=2, C: weight=1
--   Tree:      [6]
--              / \
--             A   [3]
--            (3)  / \
--                B   C
--               (2) (1)
--   Lengths: A=1, B=2, C=2
--   Sorted by (length, symbol): A(1), B(2), C(2)
--   Canonical codes:
--     A → 0        (length 1,  code = 0)
--     B → 10       (length 2,  code = 0+1=1, shifted 1 bit → 10)
--     C → 11       (length 2,  code = 10+1 = 11)
--
-- Heap Package
-- ------------
--
-- This package depends on coding_adventures.heap for the array-backed binary
-- min-heap used during greedy construction. Heap items are stored as
-- {priority, node} pairs, where priority is the 4-tuple used for
-- deterministic tie-breaking.
--
-- A binary heap uses a parent/child indexing trick:
--   - Root is at index 1.
--   - Parent of node at index i is at math.floor(i / 2).
--   - Left child of node at index i is at 2*i.
--   - Right child of node at index i is at 2*i + 1.
--
-- Push: append at the end, then "sift up" by swapping with parent while smaller.
-- Pop:  replace root with last element, shrink by one, then "sift down" by
--       swapping with the smaller child while larger than either child.
--
-- Time complexity: push O(log n), pop O(log n).
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

local heap_module = require("coding_adventures.heap")
local MinHeap = heap_module.MinHeap

local function compare_entries(left, right)
    local a = left[1]
    local b = right[1]

    if a[1] ~= b[1] then return a[1] < b[1] and -1 or 1 end
    if a[2] ~= b[2] then return a[2] < b[2] and -1 or 1 end
    if a[3] ~= b[3] then return a[3] < b[3] and -1 or 1 end
    if a[4] ~= b[4] then return a[4] < b[4] and -1 or 1 end
    return 0
end

-- ── Node Constructors ──────────────────────────────────────────────────────────

-- new_leaf creates a leaf node representing a single symbol.
--
-- A leaf has no children — it represents a concrete symbol in the alphabet.
-- The weight is the symbol's frequency.
--
-- @param symbol  integer  Non-negative integer symbol identifier.
-- @param weight  integer  Positive frequency count.
-- @return table  Leaf node.
local function new_leaf(symbol, weight)
    return {kind = "leaf", symbol = symbol, weight = weight}
end

-- new_internal creates an internal node combining two sub-trees.
--
-- An internal node is not a symbol — it is a routing node that directs
-- decoding left (bit '0') or right (bit '1'). Its weight is the sum of the
-- weights of its two children.
--
-- @param left   table    Left child node.
-- @param right  table    Right child node.
-- @param order  integer  Monotonic insertion counter for tie-breaking.
-- @return table  Internal node.
local function new_internal(left, right, order)
    return {
        kind   = "internal",
        weight = left.weight + right.weight,
        left   = left,
        right  = right,
        order  = order,
    }
end

-- node_priority computes the 4-element heap priority tuple for a node.
--
-- The tuple encodes all four tie-breaking levels in a single comparable key:
--   [1] weight          — lower weight wins (pop smaller first)
--   [2] leaf_flag       — 0=leaf, 1=internal (leaves have higher priority)
--   [3] symbol_or_huge  — for leaves: symbol value; for internal: math.huge
--   [4] order_or_huge   — for internal: insertion order; for leaf: math.huge
--
-- By using math.huge for unused fields, a leaf with any symbol always sorts
-- before an internal node when weight and leaf_flag are equal.
-- Similarly, an internal with any order always sorts before math.huge.
--
-- @param node table  A leaf or internal node.
-- @return table  4-element priority tuple.
local function node_priority(node)
    if node.kind == "leaf" then
        return {node.weight, 0, node.symbol, math.huge}
    else
        return {node.weight, 1, math.huge, node.order}
    end
end

-- ── HuffmanTree API ───────────────────────────────────────────────────────────

-- The public module is a table of functions. Unlike Python where we create
-- instances with __init__, in Lua we use a table to hold the tree root plus
-- the symbol count, and all functions receive the tree table as the first arg.
-- This is idiomatic Lua OOP: the tree object is a plain table with a metatable
-- allowing method-call syntax (tree:method()).

local HuffmanTree = {}
HuffmanTree.__index = HuffmanTree

-- HuffmanTree.build constructs a Huffman tree from a list of (symbol, freq) pairs.
--
-- Algorithm:
--   1. Validate inputs — no empty list, all frequencies positive.
--   2. Push one leaf per symbol onto the min-heap.
--   3. Repeat: pop two nodes, merge into internal, push result.
--   4. When one node remains it is the root.
--
-- The tie-breaking rules are encoded in node_priority():
--   1. Lower weight first.
--   2. Leaf before internal at equal weight.
--   3. Lower symbol value among leaves of equal weight.
--   4. Insertion order (FIFO) among internal nodes of equal weight.
--
-- @param weights  table  Array of {symbol, frequency} pairs (1-indexed).
-- @return HuffmanTree  An object ready for encode/decode.
-- @error  If weights is empty or any frequency is <= 0.
function HuffmanTree.build(weights)
    if not weights or #weights == 0 then
        error("weights must not be empty")
    end
    for _, pair in ipairs(weights) do
        if pair[2] <= 0 then
            error(string.format(
                "frequency must be positive; got symbol=%d, freq=%d",
                pair[1], pair[2]))
        end
    end

    local heap = MinHeap.new(compare_entries)

    -- Seed the heap with one leaf per symbol.
    for _, pair in ipairs(weights) do
        local leaf = new_leaf(pair[1], pair[2])
        heap:push({node_priority(leaf), leaf})
    end

    local order_counter = 0

    -- Merge phase: repeatedly combine the two smallest nodes.
    while heap:len() > 1 do
        local left_entry = heap:pop()
        local right_entry = heap:pop()
        local left = left_entry[2]
        local right = right_entry[2]
        local internal = new_internal(left, right, order_counter)
        order_counter = order_counter + 1
        heap:push({node_priority(internal), internal})
    end

    local root = heap:pop()[2]

    local tree = {
        _root         = root,
        _symbol_count = #weights,
    }
    setmetatable(tree, HuffmanTree)
    return tree
end

-- ── Code Table ────────────────────────────────────────────────────────────────

-- code_table returns a hash of {[symbol] = bit_string} for all symbols.
--
-- Traversal: left child = '0', right child = '1'.
-- Single-symbol edge case: the root is a leaf, so there are no edges to
-- traverse. By convention, the one symbol gets code '0'.
--
-- Time: O(n) where n = number of symbols.
--
-- @return table  Hash mapping integer symbols to bit strings.
function HuffmanTree:code_table()
    local table = {}
    local function walk(node, prefix)
        if node.kind == "leaf" then
            table[node.symbol] = (prefix ~= "" and prefix or "0")
            return
        end
        walk(node.left,  prefix .. "0")
        walk(node.right, prefix .. "1")
    end
    walk(self._root, "")
    return table
end

-- code_for returns the bit string for a specific symbol, or nil if not in tree.
--
-- Searches the tree for the leaf with the given symbol. Unlike code_table(),
-- this does not build the entire table — useful for single-symbol lookup.
--
-- Time: O(n) worst case (full tree traversal).
--
-- @param symbol  integer  The symbol to look up.
-- @return string|nil  The bit string, or nil if not found.
function HuffmanTree:code_for(symbol)
    local function find(node, prefix)
        if node.kind == "leaf" then
            if node.symbol == symbol then
                return (prefix ~= "" and prefix or "0")
            end
            return nil
        end
        local left_result = find(node.left, prefix .. "0")
        if left_result then return left_result end
        return find(node.right, prefix .. "1")
    end
    return find(self._root, "")
end

-- ── Canonical Code Table ──────────────────────────────────────────────────────

-- canonical_code_table returns DEFLATE-style canonical Huffman codes.
--
-- Canonical codes normalise the code assignment: given only the symbol lengths,
-- the exact codes can always be reproduced. This is how DEFLATE stores Huffman
-- tables — it transmits lengths, not full codes.
--
-- Steps:
--   1. Walk the tree to collect code lengths for all symbols.
--   2. Sort by (length, symbol) — shorter codes get smaller numeric values.
--   3. Assign codes numerically: first code = 0, then increment and shift left
--      when moving to a longer length.
--
-- Time: O(n log n) for the sort step.
--
-- @return table  Hash mapping integer symbols to canonical bit strings.
function HuffmanTree:canonical_code_table()
    -- Step 1: collect lengths.
    local lengths = {}

    local function collect_lengths(node, depth)
        if node.kind == "leaf" then
            -- Single-leaf tree: the leaf is at depth 0, but length is 1 by convention.
            lengths[node.symbol] = (depth > 0 and depth or 1)
            return
        end
        collect_lengths(node.left,  depth + 1)
        collect_lengths(node.right, depth + 1)
    end
    collect_lengths(self._root, 0)

    -- Single-leaf edge case: assign length 1 (code '0') directly.
    if self._symbol_count == 1 then
        local sym = next(lengths)
        return {[sym] = "0"}
    end

    -- Step 2: sort pairs by (length, symbol).
    local sorted = {}
    for sym, len in pairs(lengths) do
        sorted[#sorted + 1] = {sym, len}
    end
    table.sort(sorted, function(a, b)
        if a[2] ~= b[2] then return a[2] < b[2] end
        return a[1] < b[1]
    end)

    -- Step 3: assign codes numerically.
    -- Start at code 0 for the first symbol.
    -- For each subsequent symbol: if length stayed the same, increment.
    -- If length increased, shift left (code <<= length_diff) then increment.
    local code_val  = 0
    local prev_len  = sorted[1][2]
    local result    = {}

    for _, pair in ipairs(sorted) do
        local sym = pair[1]
        local len = pair[2]
        if len > prev_len then
            code_val = code_val << (len - prev_len)
        end
        -- Format code_val as a zero-padded binary string of length `len`.
        -- We build the binary string bit by bit (MSB first).
        local bits = {}
        local v = code_val
        for b = len - 1, 0, -1 do
            bits[#bits + 1] = ((v >> b) & 1 == 1) and "1" or "0"
        end
        result[sym] = table.concat(bits)
        code_val  = code_val + 1
        prev_len  = len
    end

    return result
end

-- ── Decoding ──────────────────────────────────────────────────────────────────

-- decode_all decodes exactly `count` symbols from a bit string.
--
-- Decoding works by walking the tree one bit at a time:
--   '0' → go left, '1' → go right.
-- When a leaf is reached, emit its symbol and return to the root.
--
-- Single-leaf edge case: the root is a leaf, so there are no edges. Each
-- symbol is encoded as a single '0' bit, which we consume without walking.
--
-- Multi-leaf case: after reaching a leaf (which happens after consuming the
-- last edge bit), we do NOT advance the bit index again — the index is already
-- past the last consumed bit when the leaf check fires.
--
-- @param bits   string   A string of '0' and '1' characters.
-- @param count  integer  The exact number of symbols to decode.
-- @return table  Array of decoded symbols (1-indexed).
-- @error  If the bit stream is exhausted before `count` symbols are decoded.
function HuffmanTree:decode_all(bits, count)
    local result      = {}
    local node        = self._root
    local i           = 1  -- 1-indexed position into the bits string
    local single_leaf = (self._root.kind == "leaf")

    while #result < count do
        if node.kind == "leaf" then
            result[#result + 1] = node.symbol
            node = self._root
            if single_leaf then
                -- For a single-leaf tree, consume one '0' bit per symbol.
                if i <= #bits then
                    i = i + 1
                end
            end
            -- For multi-leaf trees: index is already advanced past the leaf edge;
            -- no extra increment needed here.
        else
            -- Not yet at a leaf — consume the next bit.
            if i > #bits then
                error(string.format(
                    "bit stream exhausted after %d symbols; expected %d",
                    #result, count))
            end
            local bit = bits:sub(i, i)
            i = i + 1
            node = (bit == "0") and node.left or node.right
        end
    end

    return result
end

-- ── Inspection ────────────────────────────────────────────────────────────────

-- weight returns the total weight of the tree (= sum of all leaf frequencies).
--
-- This equals the root weight because each internal node's weight is the sum
-- of its children's weights, so the root accumulates all leaf frequencies.
-- O(1) — stored at the root.
--
-- @return integer  Total weight.
function HuffmanTree:weight()
    return self._root.weight
end

-- depth returns the maximum code length (= depth of the deepest leaf).
--
-- Traverses the entire tree to find the leaf at greatest depth.
-- O(n) — must visit every node.
--
-- @return integer  Maximum depth (0 for a single-leaf tree).
function HuffmanTree:depth()
    local function max_depth(node, d)
        if node.kind == "leaf" then return d end
        return math.max(max_depth(node.left, d + 1), max_depth(node.right, d + 1))
    end
    return max_depth(self._root, 0)
end

-- symbol_count returns the number of distinct symbols (= number of leaf nodes).
--
-- Stored at construction time; O(1).
--
-- @return integer  Number of distinct symbols.
function HuffmanTree:symbol_count()
    return self._symbol_count
end

-- leaves returns an in-order traversal of all leaves.
--
-- Returns a list of {symbol, code} pairs in left-to-right (in-order) order.
-- Useful for visualisation and debugging.
--
-- Time: O(n).
--
-- @return table  Array of {symbol, code} pairs (1-indexed).
function HuffmanTree:leaves()
    local code_tbl = self:code_table()
    local result   = {}

    local function walk(node)
        if node.kind == "leaf" then
            result[#result + 1] = {node.symbol, code_tbl[node.symbol]}
            return
        end
        walk(node.left)
        walk(node.right)
    end
    walk(self._root)
    return result
end

-- is_valid checks structural invariants of the tree.
--
-- Invariants checked:
--   1. Every internal node has exactly 2 children (full binary tree).
--   2. weight(internal) == weight(left) + weight(right).
--   3. No symbol appears in more than one leaf (no duplicate symbols).
--
-- Returns true if all invariants hold; false otherwise.
-- For testing and assertions only; not needed for normal encode/decode.
--
-- @return boolean
function HuffmanTree:is_valid()
    local seen = {}

    local function check(node)
        if node.kind == "leaf" then
            if seen[node.symbol] then return false end
            seen[node.symbol] = true
            return true
        end
        -- Internal node: both children must exist.
        if not node.left or not node.right then return false end
        -- Weight invariant: parent = left + right.
        if node.weight ~= node.left.weight + node.right.weight then
            return false
        end
        return check(node.left) and check(node.right)
    end

    return check(self._root)
end

-- Expose the HuffmanTree constructor as the module's main entry point.
-- Usage:
--   local HuffmanTree = require("coding_adventures.huffman_tree")
--   local tree = HuffmanTree.build({{65, 3}, {66, 2}, {67, 1}})
M.build               = HuffmanTree.build
M.HuffmanTree         = HuffmanTree

return M
