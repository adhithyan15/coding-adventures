-- b_plus_tree/init.lua — B+ Tree (DT12) implementation in Lua
-- ============================================================
--
-- A B+ Tree is a variant of the B-Tree with two structural differences:
--
-- 1. Internal nodes hold ONLY separator keys (no values).
--    All (key, value) pairs live exclusively in leaf nodes.
--    Internal nodes are a pure routing index.
--
-- 2. Leaf nodes form a singly-linked list.
--    Each leaf has a `next` pointer to the next leaf in key order.
--    Range scans walk this linked list without touching internal nodes.
--
-- ASCII diagram — B+ Tree with t=2, 5 entries:
--
--   Internal:        [3]
--                   /    \
--   Leaves:  [1,2] ──▶ [3,4,5]
--             ↑↑         ↑↑↑
--           values       values
--
-- Key 3 appears in BOTH the internal separator AND the right leaf.
-- This is the crucial difference from a B-Tree:
--   B-Tree  — median key MOVES up, disappears from children.
--   B+ Tree — first key of right leaf is COPIED up, stays in leaf.
--
-- Leaf node:
--   keys[]   — data keys in sorted order
--   values[] — values for each key (parallel array)
--   next     — reference to next leaf, or nil
--   leaf     — true
--
-- Internal node:
--   keys[]    — separator keys (routing only, no values)
--   children  — array of child node refs (length = #keys + 1)
--   leaf      — false
--
-- Child-index rule for internal nodes (B+ Tree):
--   children[i] where i = count of separators <= key + 1.
--   Equivalently: descend into children[i] where i is the first position
--   such that key < separator[i] (or i = n+1 if key >= all separators).

local BPlusTree = {}
BPlusTree.__index = BPlusTree

-- ---------------------------------------------------------------------------
-- Node constructor helpers
-- ---------------------------------------------------------------------------

local function new_leaf()
    return { keys = {}, values = {}, next = nil, leaf = true, n = 0 }
end

local function new_internal()
    return { keys = {}, children = {}, leaf = false, n = 0 }
end

-- ---------------------------------------------------------------------------
-- Private helper functions (forward declarations for cross-references)
-- ---------------------------------------------------------------------------
-- We declare all private functions here so methods can reference them freely.

local height_of
local find_index
local child_index
local find_leaf
local leftmost_leaf
local split_child
local split_leaf
local split_internal
local insert_non_full
local prepare_child
local borrow_from_left
local borrow_from_right
local merge_children_bplus
local delete_bplus
local is_valid_node
local is_linked_list_valid

-- ---------------------------------------------------------------------------
-- BPlusTree constructor
-- ---------------------------------------------------------------------------

--- Create a new B+ Tree.
--- @param opts table  Optional: { t = minimum_degree }  Default: t=2.
function BPlusTree.new(opts)
    opts = opts or {}
    local t = opts.t or 2
    assert(t >= 2, "minimum degree t must be >= 2")
    local self = setmetatable({}, BPlusTree)
    self.t          = t
    local root      = new_leaf()
    self.root       = root
    self.first_leaf = root
    self.count      = 0
    return self
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

--- Return the number of key-value pairs stored in the tree.
function BPlusTree:size()
    return self.count
end

--- Return the height of the tree.
--- An empty tree (single empty leaf) has height 0.
function BPlusTree:height()
    return height_of(self.root)
end

--- Search for key.  Returns the associated value, or nil if not found.
function BPlusTree:search(key)
    local leaf = find_leaf(self.root, key)
    local i    = find_index(leaf, key)
    if i <= leaf.n and leaf.keys[i] == key then
        return leaf.values[i]
    end
    return nil
end

--- Insert (key, value).  Overwrites if key already exists (upsert).
function BPlusTree:insert(key, value)
    local root = self.root
    if root.n == 2 * self.t - 1 then
        -- Root is full — grow the tree by one level.
        local new_root = new_internal()
        new_root.children[1] = root
        split_child(new_root, 1, self.t)
        self.root = new_root
        root = new_root
    end
    local inserted = insert_non_full(root, key, value, self.t)
    if inserted then self.count = self.count + 1 end
    -- Update first_leaf in case a split created a new leftmost leaf.
    self.first_leaf = leftmost_leaf(self.root)
end

--- Remove key from the tree.
--- Returns true if found, false otherwise.
function BPlusTree:delete(key)
    if self.count == 0 then return false end
    local found = delete_bplus(self.root, key, self.t)
    if found then
        self.count = self.count - 1
        -- Collapse root if it lost all its separator keys.
        if self.root.n == 0 and not self.root.leaf then
            self.root = self.root.children[1]
        end
    end
    self.first_leaf = leftmost_leaf(self.root)
    return found
end

--- Return the minimum key in the tree (or nil if empty).
function BPlusTree:min_key()
    if self.count == 0 then return nil end
    return self.first_leaf.keys[1]
end

--- Return the maximum key in the tree (or nil if empty).
function BPlusTree:max_key()
    if self.count == 0 then return nil end
    local leaf = self.first_leaf
    while leaf.next ~= nil do leaf = leaf.next end
    return leaf.keys[leaf.n]
end

--- range_scan(low, high)
--- Return all {key, value} pairs where low <= key <= high.
--- Uses the leaf linked list for O(log n + k) performance.
function BPlusTree:range_scan(low, high)
    local result = {}
    local leaf   = find_leaf(self.root, low)
    while leaf ~= nil do
        for i = 1, leaf.n do
            if leaf.keys[i] > high then
                return result
            end
            if leaf.keys[i] >= low then
                result[#result + 1] = { leaf.keys[i], leaf.values[i] }
            end
        end
        leaf = leaf.next
    end
    return result
end

--- full_scan() — walk the entire leaf linked list.  O(n).
function BPlusTree:full_scan()
    local result = {}
    local leaf   = self.first_leaf
    while leaf ~= nil do
        for i = 1, leaf.n do
            result[#result + 1] = { leaf.keys[i], leaf.values[i] }
        end
        leaf = leaf.next
    end
    return result
end

--- inorder() — alias for full_scan.  All data is in leaves.
function BPlusTree:inorder()
    return self:full_scan()
end

--- Validate all B+ Tree invariants including linked-list integrity.
function BPlusTree:is_valid()
    local h = height_of(self.root)
    if not is_valid_node(self.root, nil, nil, h, 0, true, self.t) then
        return false
    end
    return is_linked_list_valid(self.first_leaf, self.count)
end

-- ---------------------------------------------------------------------------
-- Private — height
-- ---------------------------------------------------------------------------

height_of = function(node)
    if node.leaf then return 0 end
    return 1 + height_of(node.children[1])
end

-- ---------------------------------------------------------------------------
-- Private — helpers
-- ---------------------------------------------------------------------------

--- First index i (1-based) where keys[i] >= key.
find_index = function(node, key)
    local i = 1
    while i <= node.n and node.keys[i] < key do
        i = i + 1
    end
    return i
end

--- For a B+ Tree internal node: count of separators that are <= key, plus 1.
--- This gives the 1-based child index to follow.
child_index = function(node, key)
    local i = 1
    while i <= node.n and key >= node.keys[i] do
        i = i + 1
    end
    return i
end

--- Walk down from node to the leaf that should contain key.
find_leaf = function(node, key)
    while not node.leaf do
        local i = child_index(node, key)
        node = node.children[i]
    end
    return node
end

--- Walk down to the leftmost leaf.
leftmost_leaf = function(node)
    while not node.leaf do node = node.children[1] end
    return node
end

-- ---------------------------------------------------------------------------
-- Private — split
-- ---------------------------------------------------------------------------

--- Split the i-th child of parent (1-based).
--- Dispatches to the correct split based on whether the child is a leaf.
split_child = function(parent, i, t)
    local child = parent.children[i]
    if child.leaf then
        split_leaf(parent, i)
    else
        split_internal(parent, i, t)
    end
end

--- Leaf split.
--- Left leaf keeps the first half.  Right leaf keeps the second half.
--- The first key of the right leaf is COPIED up as a separator (stays in leaf).
split_leaf = function(parent, i)
    local left  = parent.children[i]
    local right = new_leaf()
    local total = left.n
    local mid   = math.floor(total / 2) + 1   -- first index of right half

    -- Copy upper half to right.
    for j = mid, total do
        local ri = j - mid + 1
        right.keys[ri]   = left.keys[j]
        right.values[ri] = left.values[j]
    end
    right.n = total - mid + 1

    -- Shrink left to lower half.
    for j = mid, total do
        left.keys[j]   = nil
        left.values[j] = nil
    end
    left.n = mid - 1

    -- Fix linked list: left → right → old left.next.
    right.next = left.next
    left.next  = right

    -- Insert separator into parent (first key of right, stays in right leaf).
    local sep = right.keys[1]
    for j = parent.n + 1, i + 1, -1 do
        parent.keys[j] = parent.keys[j - 1]
    end
    parent.keys[i] = sep
    parent.n = parent.n + 1

    -- Insert right child after position i.
    for j = parent.n + 1, i + 2, -1 do
        parent.children[j] = parent.children[j - 1]
    end
    parent.children[i + 1] = right
end

--- Internal node split (same as B-Tree).
--- Median key MOVES up into parent and does NOT stay in either child.
split_internal = function(parent, i, t)
    local full  = parent.children[i]
    local right = new_internal()
    local mk    = full.keys[t]   -- median key

    -- Copy upper half of keys to right (exclude median).
    for j = t + 1, full.n do
        local ri = j - t
        right.keys[ri] = full.keys[j]
        full.keys[j]   = nil
    end
    -- Copy upper half of children to right.
    for j = t + 1, full.n + 1 do
        local ri = j - t
        right.children[ri] = full.children[j]
        full.children[j]   = nil
    end
    right.n = full.n - t
    full.keys[t]  = nil
    full.n        = t - 1

    -- Insert median into parent.
    for j = parent.n + 1, i + 1, -1 do
        parent.keys[j] = parent.keys[j - 1]
    end
    parent.keys[i] = mk
    parent.n = parent.n + 1

    -- Insert right child after position i.
    for j = parent.n + 1, i + 2, -1 do
        parent.children[j] = parent.children[j - 1]
    end
    parent.children[i + 1] = right
end

-- ---------------------------------------------------------------------------
-- Private — insert
-- ---------------------------------------------------------------------------

--- Insert (key, value) into a subtree rooted at node.
--- Precondition: node is NOT full.
--- Returns true if a new key was inserted, false if an existing key was updated.
insert_non_full = function(node, key, value, t)
    if node.leaf then
        local i = find_index(node, key)
        if i <= node.n and node.keys[i] == key then
            node.values[i] = value   -- upsert
            return false
        end
        -- Shift right to make room.
        for j = node.n, i, -1 do
            node.keys[j + 1]   = node.keys[j]
            node.values[j + 1] = node.values[j]
        end
        node.keys[i]   = key
        node.values[i] = value
        node.n = node.n + 1
        return true
    end

    -- Internal node: find child index using B+ child_index rule.
    local ci = child_index(node, key)
    -- Clamp to valid range.
    if ci > node.n + 1 then ci = node.n + 1 end

    -- Split proactively if the child is full.
    if node.children[ci].n == 2 * t - 1 then
        split_child(node, ci, t)
        -- Re-compute child index after split.
        if key >= node.keys[ci] then
            ci = ci + 1
        end
        if ci > node.n + 1 then ci = node.n + 1 end
    end
    return insert_non_full(node.children[ci], key, value, t)
end

-- ---------------------------------------------------------------------------
-- Private — delete
-- ---------------------------------------------------------------------------

--- Ensure that children[i] has at least t keys before descending.
--- Returns the (possibly updated) index of the child to use.
prepare_child = function(node, i, t)
    local child = node.children[i]
    if child.n >= t then return i end

    local has_left  = i > 1
    local has_right = i <= node.n

    if has_left and node.children[i - 1].n >= t then
        borrow_from_left(node, i)
        return i
    elseif has_right and node.children[i + 1].n >= t then
        borrow_from_right(node, i)
        return i
    elseif has_left then
        merge_children_bplus(node, i - 1, t)
        return i - 1
    else
        merge_children_bplus(node, i, t)
        return i
    end
end

--- Borrow the largest key from left sibling into children[i].
borrow_from_left = function(parent, i)
    local child = parent.children[i]
    local left  = parent.children[i - 1]

    if child.leaf then
        -- Shift child right.
        for j = child.n, 1, -1 do
            child.keys[j + 1]   = child.keys[j]
            child.values[j + 1] = child.values[j]
        end
        child.keys[1]   = left.keys[left.n]
        child.values[1] = left.values[left.n]
        left.keys[left.n]   = nil
        left.values[left.n] = nil
        left.n  = left.n  - 1
        child.n = child.n + 1
        -- Update separator in parent.
        parent.keys[i - 1] = child.keys[1]
    else
        -- Internal node borrow.
        for j = child.n, 1, -1 do
            child.keys[j + 1]      = child.keys[j]
            child.children[j + 2]  = child.children[j + 1]
        end
        child.children[2] = child.children[1]
        child.keys[1]     = parent.keys[i - 1]
        parent.keys[i - 1]       = left.keys[left.n]
        child.children[1]         = left.children[left.n + 1]
        left.keys[left.n]         = nil
        left.children[left.n + 1] = nil
        left.n  = left.n  - 1
        child.n = child.n + 1
    end
end

--- Borrow the smallest key from right sibling into children[i].
borrow_from_right = function(parent, i)
    local child = parent.children[i]
    local right = parent.children[i + 1]

    if child.leaf then
        child.n = child.n + 1
        child.keys[child.n]   = right.keys[1]
        child.values[child.n] = right.values[1]
        for j = 1, right.n - 1 do
            right.keys[j]   = right.keys[j + 1]
            right.values[j] = right.values[j + 1]
        end
        right.keys[right.n]   = nil
        right.values[right.n] = nil
        right.n = right.n - 1
        -- Update separator in parent.
        parent.keys[i] = right.keys[1]
    else
        child.n = child.n + 1
        child.keys[child.n]         = parent.keys[i]
        child.children[child.n + 1] = right.children[1]
        parent.keys[i] = right.keys[1]
        for j = 1, right.n - 1 do
            right.keys[j]      = right.keys[j + 1]
            right.children[j]  = right.children[j + 1]
        end
        right.children[right.n] = right.children[right.n + 1]
        right.keys[right.n]           = nil
        right.children[right.n + 1]   = nil
        right.n = right.n - 1
    end
end

--- Merge children[i] and children[i+1].
--- For leaves: concatenate and fix linked list; drop separator.
--- For internals: pull down separator from parent; concatenate.
merge_children_bplus = function(parent, i, t)
    local left  = parent.children[i]
    local right = parent.children[i + 1]

    if left.leaf then
        -- Merge leaves: append right's keys/values into left.
        for j = 1, right.n do
            local li = left.n + j
            left.keys[li]   = right.keys[j]
            left.values[li] = right.values[j]
        end
        left.n    = left.n + right.n
        left.next = right.next
    else
        -- Merge internals: pull down separator key, then append right.
        left.n = left.n + 1
        left.keys[left.n] = parent.keys[i]
        for j = 1, right.n do
            left.n = left.n + 1
            left.keys[left.n]       = right.keys[j]
            left.children[left.n]   = right.children[j]
        end
        left.children[left.n + 1] = right.children[right.n + 1]
    end

    -- Remove separator from parent and right child pointer.
    for j = i, parent.n - 1 do
        parent.keys[j]          = parent.keys[j + 1]
        parent.children[j + 1] = parent.children[j + 2]
    end
    parent.keys[parent.n]         = nil
    parent.children[parent.n + 1] = nil
    parent.n = parent.n - 1
end

--- Delete key from the subtree rooted at node.
--- Returns true if found, false otherwise.
delete_bplus = function(node, key, t)
    if node.leaf then
        local i = find_index(node, key)
        if i > node.n or node.keys[i] ~= key then return false end
        -- Remove from leaf.
        for j = i, node.n - 1 do
            node.keys[j]   = node.keys[j + 1]
            node.values[j] = node.values[j + 1]
        end
        node.keys[node.n]   = nil
        node.values[node.n] = nil
        node.n = node.n - 1
        return true
    end

    -- Internal node: find child to descend into.
    local ci = child_index(node, key)
    if ci > node.n + 1 then ci = node.n + 1 end

    local new_ci = prepare_child(node, ci, t)
    -- Clamp after possible merge.
    if new_ci > #node.children then new_ci = #node.children end
    return delete_bplus(node.children[new_ci], key, t)
end

-- ---------------------------------------------------------------------------
-- Private — validation
-- ---------------------------------------------------------------------------

--- Recursively validate B+ Tree structural invariants.
is_valid_node = function(node, min_key, max_key, expected_depth, depth, is_root, t)
    local min_keys = is_root and 0 or (t - 1)
    if node.n < min_keys or node.n > 2 * t - 1 then return false end

    if node.leaf then
        -- Leaves must all be at the same depth.
        if depth ~= expected_depth then return false end
        -- Keys must be strictly sorted.
        for i = 2, node.n do
            if node.keys[i] <= node.keys[i - 1] then return false end
        end
        -- Check bounds.
        if min_key ~= nil and node.n > 0 and node.keys[1] < min_key then
            return false
        end
        if max_key ~= nil and node.n > 0 and node.keys[node.n] > max_key then
            return false
        end
        return true
    end

    -- Internal node.
    if #node.children ~= node.n + 1 then return false end
    -- Separator keys must be strictly sorted.
    for i = 2, node.n do
        if node.keys[i] <= node.keys[i - 1] then return false end
    end

    -- Recurse into children with updated bounds.
    for i = 1, node.n + 1 do
        local cmin = i == 1           and min_key  or node.keys[i - 1]
        local cmax = i == node.n + 1  and max_key  or node.keys[i]
        if not is_valid_node(node.children[i], cmin, cmax,
                             expected_depth, depth + 1, false, t) then
            return false
        end
    end
    return true
end

--- Validate the leaf linked list: count and sort order.
is_linked_list_valid = function(first_leaf, expected_count)
    local total    = 0
    local prev_key = nil
    local leaf     = first_leaf
    while leaf ~= nil do
        for i = 1, leaf.n do
            if i > 1 and leaf.keys[i] <= leaf.keys[i - 1] then return false end
            if prev_key ~= nil and leaf.keys[i] <= prev_key then return false end
            prev_key = leaf.keys[i]
            total    = total + 1
        end
        leaf = leaf.next
    end
    return total == expected_count
end

return BPlusTree
