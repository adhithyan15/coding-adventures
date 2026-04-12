-- b_tree/init.lua — B-Tree (DT11) implementation in Lua
-- =======================================================
--
-- A B-Tree is a self-balancing search tree introduced by Bayer and McCreight
-- in 1970.  It is the backbone of virtually every database and filesystem
-- because it minimises disk seeks: nodes are large, so the tree is shallow.
--
-- Definitions
-- -----------
-- Minimum degree t (always >= 2):
--   • Every non-root node has at least  t-1 keys.
--   • Every node has at most           2t-1 keys.
--   • Every internal node with n keys has n+1 children.
--
-- ASCII picture — B-Tree with t=2, keys 1..7:
--
--            [4]
--           /    \
--        [2]     [6]
--       /   \   /   \
--     [1]  [3] [5]  [7]
--
-- Insertion strategy: proactive top-down splitting.
--   We split full nodes BEFORE descending into them (never backtrack).
--
-- Deletion strategy: CLRS-style top-down preparation.
--   Before entering a child, ensure it has >= t keys (never backtrack).
--   Cases:
--     A)  Key in leaf → remove directly.
--     B1) Key in internal, left child has >= t keys → replace with predecessor.
--     B2) Key in internal, right child has >= t keys → replace with successor.
--     B3) Both children have t-1 keys → merge them, then delete from merged.
--     C)  Key not in current node → prepare child (rotate or merge) first.
--
-- Node representation (Lua table / prototype object):
--   node.keys     — array of keys in sorted order
--   node.values   — parallel array of values (one per key)
--   node.children — array of child nodes (nil for leaves)
--   node.leaf     — boolean: true if leaf node
--   node.n        — number of keys (same as #node.keys)

local BTree = {}
BTree.__index = BTree

-- ---------------------------------------------------------------------------
-- Node constructor helpers
-- ---------------------------------------------------------------------------

--- Create a new leaf node.
local function new_leaf()
    return { keys = {}, values = {}, children = nil, leaf = true, n = 0 }
end

--- Create a new internal node.
local function new_internal()
    return { keys = {}, values = {}, children = {}, leaf = false, n = 0 }
end

-- ---------------------------------------------------------------------------
-- BTree constructor
-- ---------------------------------------------------------------------------

--- Create a new B-Tree.
--- @param opts table  Optional: { t = minimum_degree }  Default: t=2.
function BTree.new(opts)
    opts = opts or {}
    local t = opts.t or 2
    assert(t >= 2, "minimum degree t must be >= 2")
    local self = setmetatable({}, BTree)
    self.t     = t
    self.root  = new_leaf()
    self.count = 0
    return self
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

--- Return the number of key-value pairs stored in the tree.
function BTree:size()
    return self.count
end

--- Return the height of the tree.
--- An empty tree (single empty leaf) has height 0.
function BTree:height()
    return height_of(self.root)
end

--- Search for key.  Returns the associated value, or nil if not found.
function BTree:search(key)
    return search_node(self.root, key)
end

--- Insert (key, value).  Overwrites if key already exists (upsert).
function BTree:insert(key, value)
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
end

--- Remove key from the tree.
--- Returns true if the key was found and removed, false otherwise.
function BTree:delete(key)
    if self.count == 0 then return false end
    local found = delete_node(self.root, key, self.t)
    if found then
        self.count = self.count - 1
        -- If the root has become empty (and is not a leaf), shrink the tree.
        if self.root.n == 0 and not self.root.leaf then
            self.root = self.root.children[1]
        end
    end
    return found
end

--- Return the minimum key in the tree (or nil if empty).
function BTree:min_key()
    if self.count == 0 then return nil end
    local node = self.root
    while not node.leaf do node = node.children[1] end
    return node.keys[1]
end

--- Return the maximum key in the tree (or nil if empty).
function BTree:max_key()
    if self.count == 0 then return nil end
    local node = self.root
    while not node.leaf do node = node.children[node.n + 1] end
    return node.keys[node.n]
end

--- In-order traversal.
--- Returns a list of {key, value} pairs in ascending key order.
function BTree:inorder()
    local result = {}
    inorder_node(self.root, result)
    return result
end

--- Range query: return all {key, value} pairs where low <= key <= high.
function BTree:range_query(low, high)
    local result = {}
    range_node(self.root, low, high, result)
    return result
end

--- Validate the B-Tree invariants.
--- Returns true if everything is correct, false otherwise.
function BTree:is_valid()
    local h = height_of(self.root)
    return is_valid_node(self.root, nil, nil, h, 0, true, self.t)
end

-- ---------------------------------------------------------------------------
-- Private — height
-- ---------------------------------------------------------------------------

--- Compute the height of a subtree rooted at node.
function height_of(node)
    if node.leaf then return 0 end
    return 1 + height_of(node.children[1])
end

-- ---------------------------------------------------------------------------
-- Private — search
-- ---------------------------------------------------------------------------

--- Find the first index i (1-based) where keys[i] >= key.
--- Returns n+1 if key is larger than all keys.
local function find_index(node, key)
    local i = 1
    while i <= node.n and node.keys[i] < key do
        i = i + 1
    end
    return i
end

--- Search for key in subtree rooted at node.  Returns value or nil.
function search_node(node, key)
    local i = find_index(node, key)
    if i <= node.n and node.keys[i] == key then
        return node.values[i]
    end
    if node.leaf then return nil end
    return search_node(node.children[i], key)
end

-- ---------------------------------------------------------------------------
-- Private — split
-- ---------------------------------------------------------------------------

--- Split the i-th child of parent (1-based).
--- The child must be full (2t-1 keys).
--- After the split, parent has one more key and one more child.
function split_child(parent, i, t)
    local full  = parent.children[i]
    local right = full.leaf and new_leaf() or new_internal()

    -- Median key (1-based index t in the full node).
    local mk  = full.keys[t]
    local mv  = full.values[t]

    -- Copy the upper half of full into right.
    for j = 1, t - 1 do
        right.keys[j]   = full.keys[t + j]
        right.values[j] = full.values[t + j]
    end
    right.n = t - 1

    if not full.leaf then
        for j = 1, t do
            right.children[j] = full.children[t + j]
        end
    end

    -- Shrink full to the lower half.
    for j = t, full.n do
        full.keys[j]   = nil
        full.values[j] = nil
    end
    if not full.leaf then
        for j = t + 1, full.n + 1 do
            full.children[j] = nil
        end
    end
    full.n = t - 1

    -- Insert median key into parent at position i.
    for j = parent.n + 1, i + 1, -1 do
        parent.keys[j]   = parent.keys[j - 1]
        parent.values[j] = parent.values[j - 1]
    end
    parent.keys[i]   = mk
    parent.values[i] = mv
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
function insert_non_full(node, key, value, t)
    if node.leaf then
        -- Find insertion position.
        local i = find_index(node, key)
        if i <= node.n and node.keys[i] == key then
            node.values[i] = value   -- upsert
            return false
        end
        -- Shift keys right to make room.
        for j = node.n, i, -1 do
            node.keys[j + 1]   = node.keys[j]
            node.values[j + 1] = node.values[j]
        end
        node.keys[i]   = key
        node.values[i] = value
        node.n = node.n + 1
        return true
    end

    -- Internal node: find child to descend into.
    local i = find_index(node, key)
    -- Exact match in internal node → upsert in place.
    if i <= node.n and node.keys[i] == key then
        node.values[i] = value
        return false
    end

    -- Descend into children[i].  Split proactively if full.
    if node.children[i].n == 2 * t - 1 then
        split_child(node, i, t)
        -- After split, the median is at node.keys[i].
        if key > node.keys[i] then
            i = i + 1
        elseif key == node.keys[i] then
            node.values[i] = value   -- upsert at new median position
            return false
        end
    end
    return insert_non_full(node.children[i], key, value, t)
end

-- ---------------------------------------------------------------------------
-- Private — delete
-- ---------------------------------------------------------------------------

--- Ensure that children[i] has at least t keys before descending.
--- May merge with sibling (reducing parent.n by 1).
--- Returns the (possibly updated) child index to use.
local function prepare_child(node, i, t)
    local child = node.children[i]
    if child.n >= t then return i end   -- already has enough keys

    local has_left  = i > 1
    local has_right = i <= node.n

    if has_left and node.children[i - 1].n >= t then
        -- Rotate right (borrow from left sibling).
        local left = node.children[i - 1]
        -- Shift child keys right.
        for j = child.n + 1, 2, -1 do
            child.keys[j]   = child.keys[j - 1]
            child.values[j] = child.values[j - 1]
        end
        if not child.leaf then
            for j = child.n + 2, 2, -1 do
                child.children[j] = child.children[j - 1]
            end
            child.children[1] = left.children[left.n + 1]
            left.children[left.n + 1] = nil
        end
        child.keys[1]   = node.keys[i - 1]
        child.values[1] = node.values[i - 1]
        node.keys[i - 1]   = left.keys[left.n]
        node.values[i - 1] = left.values[left.n]
        left.keys[left.n]   = nil
        left.values[left.n] = nil
        left.n  = left.n  - 1
        child.n = child.n + 1
        return i

    elseif has_right and node.children[i + 1].n >= t then
        -- Rotate left (borrow from right sibling).
        local right = node.children[i + 1]
        child.n = child.n + 1
        child.keys[child.n]   = node.keys[i]
        child.values[child.n] = node.values[i]
        if not child.leaf then
            child.children[child.n + 1] = right.children[1]
            for j = 1, right.n do
                right.children[j] = right.children[j + 1]
            end
            right.children[right.n + 1] = nil
        end
        node.keys[i]   = right.keys[1]
        node.values[i] = right.values[1]
        for j = 1, right.n - 1 do
            right.keys[j]   = right.keys[j + 1]
            right.values[j] = right.values[j + 1]
        end
        right.keys[right.n]   = nil
        right.values[right.n] = nil
        right.n = right.n - 1
        return i

    elseif has_left then
        -- Merge child with its left sibling (merge at index i-1).
        merge_children(node, i - 1, t)
        return i - 1

    else
        -- Merge child with its right sibling (merge at index i).
        merge_children(node, i, t)
        return i
    end
end

--- Merge children[i] and children[i+1] around the separator key at keys[i].
--- The merged node replaces children[i]; children[i+1] is discarded.
function merge_children(node, i, t)
    local left  = node.children[i]
    local right = node.children[i + 1]

    -- Pull down the separator key.
    left.n = left.n + 1
    left.keys[left.n]   = node.keys[i]
    left.values[left.n] = node.values[i]

    -- Append right's keys and values.
    for j = 1, right.n do
        left.n = left.n + 1
        left.keys[left.n]   = right.keys[j]
        left.values[left.n] = right.values[j]
    end

    -- Append right's children (if internal).
    if not right.leaf then
        for j = 1, right.n + 1 do
            left.children[left.n - right.n + j] = right.children[j]
        end
    end

    -- Remove separator from parent and right child pointer.
    for j = i, node.n - 1 do
        node.keys[j]   = node.keys[j + 1]
        node.values[j] = node.values[j + 1]
    end
    node.keys[node.n]   = nil
    node.values[node.n] = nil
    for j = i + 1, node.n do
        node.children[j] = node.children[j + 1]
    end
    node.children[node.n + 1] = nil
    node.n = node.n - 1
end

--- Find predecessor: largest key in the subtree rooted at node.
local function predecessor(node)
    while not node.leaf do node = node.children[node.n + 1] end
    return node.keys[node.n], node.values[node.n]
end

--- Find successor: smallest key in the subtree rooted at node.
local function successor(node)
    while not node.leaf do node = node.children[1] end
    return node.keys[1], node.values[1]
end

--- Delete key from the subtree rooted at node.
--- Returns true if found, false otherwise.
function delete_node(node, key, t)
    local i = find_index(node, key)
    local found_here = i <= node.n and node.keys[i] == key

    if node.leaf then
        if not found_here then return false end
        -- Case A: key is in a leaf — remove directly.
        for j = i, node.n - 1 do
            node.keys[j]   = node.keys[j + 1]
            node.values[j] = node.values[j + 1]
        end
        node.keys[node.n]   = nil
        node.values[node.n] = nil
        node.n = node.n - 1
        return true
    end

    if found_here then
        local left  = node.children[i]
        local right = node.children[i + 1]

        if left.n >= t then
            -- Case B1: left child has >= t keys → replace with predecessor.
            local pk, pv = predecessor(left)
            node.keys[i]   = pk
            node.values[i] = pv
            return delete_node(left, pk, t)

        elseif right.n >= t then
            -- Case B2: right child has >= t keys → replace with successor.
            local sk, sv = successor(right)
            node.keys[i]   = sk
            node.values[i] = sv
            return delete_node(right, sk, t)

        else
            -- Case B3: both children have t-1 keys → merge and delete.
            merge_children(node, i, t)
            return delete_node(node.children[i], key, t)
        end
    end

    -- Case C: key is not in this node — descend into the appropriate child.
    -- Prepare the child to have >= t keys before descending.
    local ci     = i   -- children[i] is the child to descend into
    local new_ci = prepare_child(node, ci, t)
    return delete_node(node.children[new_ci], key, t)
end

-- ---------------------------------------------------------------------------
-- Private — traversal
-- ---------------------------------------------------------------------------

--- Collect all {key, value} pairs from the subtree into result (in order).
function inorder_node(node, result)
    for i = 1, node.n do
        if not node.leaf then
            inorder_node(node.children[i], result)
        end
        result[#result + 1] = { node.keys[i], node.values[i] }
    end
    if not node.leaf then
        inorder_node(node.children[node.n + 1], result)
    end
end

--- Collect all {key, value} pairs where low <= key <= high.
function range_node(node, low, high, result)
    for i = 1, node.n do
        if not node.leaf and node.keys[i] > low then
            range_node(node.children[i], low, high, result)
        end
        if node.keys[i] >= low and node.keys[i] <= high then
            result[#result + 1] = { node.keys[i], node.values[i] }
        end
        if node.keys[i] > high then return end
    end
    if not node.leaf then
        range_node(node.children[node.n + 1], low, high, result)
    end
end

-- ---------------------------------------------------------------------------
-- Private — validation
-- ---------------------------------------------------------------------------

--- Recursively validate the B-Tree invariants.
--- expected_depth: all leaves must be at this depth (0-based from leaves).
--- depth:          current depth from root.
--- is_root:        true only for the root node.
function is_valid_node(node, min_key, max_key, expected_depth, depth, is_root, t)
    -- Key count invariant.
    local min_keys = is_root and 0 or (t - 1)
    if node.n < min_keys or node.n > 2 * t - 1 then return false end

    -- Keys must be sorted and within [min_key, max_key].
    for i = 1, node.n do
        if min_key ~= nil and node.keys[i] <= min_key then return false end
        if max_key ~= nil and node.keys[i] >= max_key then return false end
        if i > 1 and node.keys[i] <= node.keys[i - 1] then return false end
    end

    if node.leaf then
        -- All leaves must be at the same depth.
        if depth ~= expected_depth then return false end
        return true
    end

    -- Internal node: must have n+1 children.
    if #node.children ~= node.n + 1 then return false end

    -- Recurse into children with updated bounds.
    for i = 1, node.n + 1 do
        local cmin = i == 1        and min_key        or node.keys[i - 1]
        local cmax = i == node.n + 1 and max_key      or node.keys[i]
        if not is_valid_node(node.children[i], cmin, cmax,
                             expected_depth, depth + 1, false, t) then
            return false
        end
    end
    return true
end

return BTree
