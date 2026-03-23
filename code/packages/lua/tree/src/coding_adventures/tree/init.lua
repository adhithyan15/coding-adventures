-- tree -- Rooted tree data structure backed by a directed graph
-- ================================================================
--
-- # What is a Tree?
--
-- A tree is one of the most fundamental data structures in computer science.
-- You encounter trees everywhere:
--
--   - File systems: directories contain files and subdirectories
--   - HTML/XML: elements contain child elements
--   - Programming languages: Abstract Syntax Trees (ASTs) represent code
--   - Organization charts: managers have direct reports
--
-- Formally, a tree is a connected, acyclic graph where:
--
--  1. There is exactly one root node (a node with no parent).
--  2. Every other node has exactly one parent.
--  3. There are no cycles.
--
-- These constraints mean a tree with N nodes always has exactly N-1 edges.
--
-- # Tree vs. Graph
--
-- A tree IS a graph (specifically, a directed acyclic graph with the
-- single-parent constraint). We leverage this by building our Tree on top
-- of the DirectedGraph type from the directed-graph package. The graph
-- handles all the low-level node/edge storage, while this Tree type
-- enforces the tree invariants and provides tree-specific operations
-- like traversals, depth calculation, and lowest common ancestor.
--
-- Edges point from parent to child:
--
--     Program
--     +-- Assignment    (edge: Program -> Assignment)
--     |   +-- Name      (edge: Assignment -> Name)
--     |   +-- BinaryOp  (edge: Assignment -> BinaryOp)
--     +-- Print         (edge: Program -> Print)
--
-- # Implementation Strategy
--
-- We store the tree as a DirectedGraph with edges pointing parent -> child.
-- This means:
--
--   - graph:successors(node) returns the children
--   - graph:predecessors(node) returns a list with 0 or 1 element
--     (the parent, or empty for the root)
--
-- We maintain the tree invariants by checking them in add_child:
--
--   - The parent must already exist in the tree
--   - The child must NOT already exist (no duplicate nodes)
--   - Since we only add one parent edge per child, cycles are impossible
--
-- # OOP Pattern
--
-- We use the standard Lua metatable OOP pattern, consistent with the
-- directed-graph package:
--
--     local Tree = {}
--     Tree.__index = Tree
--     function Tree.new(root) ... end
--
-- This gives us method dispatch via the : operator:
--
--     local t = Tree.new("Program")
--     t:add_child("Program", "Assignment")
--     t:add_child("Program", "Print")

local dg = require("coding_adventures.directed_graph")
local DirectedGraph = dg.DirectedGraph

-- =========================================================================
-- Error types
-- =========================================================================
--
-- Trees impose strict structural constraints. When those constraints are
-- violated, we return specific error tables so callers can handle them
-- programmatically.
--
-- Each error has a `type` field for programmatic checking and a `message`
-- field for human-readable output, following the same convention as the
-- directed-graph package.
--
-- Error checking example:
--
--     local ok, err = t:add_child("nonexistent", "child")
--     if not ok and err.type == "node_not_found" then
--         print("Node " .. err.node .. " does not exist")
--     end

--- Create a NodeNotFoundError.
-- Returned when an operation references a node that doesn't exist in
-- the tree.
-- @param node string The node that was not found.
local function NodeNotFoundError(node)
    return {
        type = "node_not_found",
        node = node,
        message = string.format("node not found in tree: %q", node),
    }
end

--- Create a DuplicateNodeError.
-- Returned when trying to add a node that already exists. In a tree,
-- every node has exactly one parent, so duplicates would violate the
-- tree invariant.
-- @param node string The duplicate node.
local function DuplicateNodeError(node)
    return {
        type = "duplicate_node",
        node = node,
        message = string.format("node already exists in tree: %q", node),
    }
end

--- Create a RootRemovalError.
-- Returned when trying to remove the root node. The root is the anchor
-- of the entire tree; removing it would leave a disconnected collection
-- of subtrees.
local function RootRemovalError()
    return {
        type = "root_removal",
        message = "cannot remove the root node",
    }
end

-- =========================================================================
-- The Tree class
-- =========================================================================

local Tree = {}
Tree.__index = Tree

--- Create a new tree with the given root node.
--
-- The root will be the ancestor of every other node in the tree.
-- A tree always starts with a root -- you can't have an empty tree.
--
-- @param root string The root node identifier.
-- @return Tree A new tree containing only the root.
--
-- Example:
--
--     local t = Tree.new("Program")
--     t:add_child("Program", "Assignment")
--     t:add_child("Program", "Print")
--     print(t:to_ascii())
function Tree.new(root)
    local self = setmetatable({}, Tree)
    self._graph = DirectedGraph.new()
    self._graph:add_node(root)
    self._root = root
    return self
end

-- =========================================================================
-- Mutation
-- =========================================================================

--- Add a child node under the given parent.
--
-- This is the primary way to build up a tree. Each call adds one new
-- node and one edge (parent -> child).
--
-- @param parent string The parent node (must exist in the tree).
-- @param child string The new child node (must NOT exist in the tree).
-- @return true on success, or nil + error table on failure.
--
-- Error cases:
--   - NodeNotFoundError if parent is not in the tree
--   - DuplicateNodeError if child already exists
function Tree:add_child(parent, child)
    if not self._graph:has_node(parent) then
        return nil, NodeNotFoundError(parent)
    end
    if self._graph:has_node(child) then
        return nil, DuplicateNodeError(child)
    end

    self._graph:add_edge(parent, child)
    return true
end

--- Remove a node and all its descendants from the tree.
--
-- This is a "prune" operation -- it cuts off an entire branch. The parent
-- of the removed node is unaffected.
--
-- We collect all nodes in the subtree via BFS, then remove them in
-- reverse order (children first) to keep the graph consistent at each
-- step.
--
-- @param node string The node to remove (along with all descendants).
-- @return true on success, or nil + error table on failure.
--
-- Error cases:
--   - NodeNotFoundError if node is not in the tree
--   - RootRemovalError if node is the root
function Tree:remove_subtree(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end
    if node == self._root then
        return nil, RootRemovalError()
    end

    -- Collect subtree via BFS, then remove in reverse (children first).
    local to_remove = self:_collect_subtree_nodes(node)

    for i = #to_remove, 1, -1 do
        self._graph:remove_node(to_remove[i])
    end
    return true
end

--- Collect all nodes in the subtree rooted at `node` using BFS.
-- Returns an array starting with `node`, then all descendants.
-- Children are sorted for determinism.
--
-- @param node string The root of the subtree.
-- @return table An array of node strings.
function Tree:_collect_subtree_nodes(node)
    local result = {}
    local queue = {node}
    local head = 1

    while head <= #queue do
        local current = queue[head]
        head = head + 1
        result[#result + 1] = current

        local children = self._graph:successors(current)
        table.sort(children)
        for _, child in ipairs(children) do
            queue[#queue + 1] = child
        end
    end

    return result
end

-- =========================================================================
-- Queries
-- =========================================================================

--- Return the root node of the tree.
-- @return string The root node identifier.
function Tree:root()
    return self._root
end

--- Return the parent of a node, or nil if the node is the root.
--
-- In a tree, every non-root node has exactly one parent. The root has
-- no parent.
--
-- @param node string The node to query.
-- @return string|nil The parent node, or nil if the node is the root.
--         On error, returns nil + error table.
function Tree:parent(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local preds = self._graph:predecessors(node)
    if #preds == 0 then
        return nil
    end
    return preds[1]
end

--- Return the children of a node (sorted alphabetically).
--
-- @param node string The node to query.
-- @return table A sorted array of child node strings.
--         On error, returns nil + error table.
function Tree:children(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local children = self._graph:successors(node)
    table.sort(children)
    return children
end

--- Return the siblings of a node (other children of the same parent).
--
-- The root has no siblings (returns an empty table).
-- Siblings are sorted alphabetically.
--
-- @param node string The node to query.
-- @return table A sorted array of sibling node strings.
--         On error, returns nil + error table.
function Tree:siblings(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local parent_node = self:parent(node)
    if parent_node == nil then
        -- Root has no siblings.
        return {}
    end

    local all_children = self:children(parent_node)
    local sibs = {}
    for _, c in ipairs(all_children) do
        if c ~= node then
            sibs[#sibs + 1] = c
        end
    end
    return sibs
end

--- Return true if the node has no children (is a leaf).
--
-- @param node string The node to query.
-- @return boolean True if the node is a leaf, false otherwise.
--         On error, returns nil + error table.
function Tree:is_leaf(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local children = self._graph:successors(node)
    return #children == 0
end

--- Return true if the node is the root of the tree.
--
-- @param node string The node to query.
-- @return boolean True if the node is the root.
--         On error, returns nil + error table.
function Tree:is_root(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    return node == self._root
end

--- Return the depth of a node (distance from root).
--
-- The depth is the number of edges on the path from the root to this
-- node. By definition:
--
--   - Root has depth 0
--   - Root's children have depth 1
--   - Grandchildren have depth 2
--   - And so on...
--
-- @param node string The node to query.
-- @return integer The depth of the node.
--         On error, returns nil + error table.
function Tree:depth(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local d = 0
    local current = node
    while current ~= self._root do
        local preds = self._graph:predecessors(current)
        current = preds[1]
        d = d + 1
    end

    return d
end

--- Return the height of the tree (maximum depth of any node).
--
-- A single-node tree has height 0. The height equals the length of
-- the longest root-to-leaf path.
--
-- Uses BFS to track depth of each node, keeping the maximum.
--
-- @return integer The height of the tree.
function Tree:height()
    local max_depth = 0

    -- BFS with depth tracking. Each queue item is {node, depth}.
    local queue = {{self._root, 0}}
    local head = 1

    while head <= #queue do
        local item = queue[head]
        head = head + 1
        local current_node = item[1]
        local current_depth = item[2]

        if current_depth > max_depth then
            max_depth = current_depth
        end

        local children = self._graph:successors(current_node)
        for _, child in ipairs(children) do
            queue[#queue + 1] = {child, current_depth + 1}
        end
    end

    return max_depth
end

--- Return the total number of nodes in the tree.
-- @return integer The node count.
function Tree:size()
    return self._graph:size()
end

--- Return all nodes in the tree (sorted alphabetically).
-- @return table A sorted array of node strings.
function Tree:nodes()
    local ns = self._graph:nodes()
    table.sort(ns)
    return ns
end

--- Return all leaf nodes (sorted alphabetically).
--
-- A leaf is a node with no children. In file system terms, leaves
-- are files while non-leaves are directories.
--
-- @return table A sorted array of leaf node strings.
function Tree:leaves()
    local result = {}
    for _, n in ipairs(self._graph:nodes()) do
        local children = self._graph:successors(n)
        if #children == 0 then
            result[#result + 1] = n
        end
    end
    table.sort(result)
    return result
end

--- Return true if the node exists in the tree.
-- @param node string The node to check.
-- @return boolean True if the node exists.
function Tree:has_node(node)
    return self._graph:has_node(node)
end

-- =========================================================================
-- Traversals
-- =========================================================================
--
-- Tree traversals visit every node exactly once, in different orders.
--
-- 1. Preorder (root first): Visit a node, then visit all its children.
--    Top-down. Good for: copying a tree, prefix notation.
--
-- 2. Postorder (root last): Visit all children, then the node.
--    Bottom-up. Good for: computing sizes, deleting trees.
--
-- 3. Level-order (BFS): Visit all nodes at depth 0, then 1, then 2, etc.
--
-- For a tree:
--
--         A
--        / \
--       B   C
--      / \
--     D   E
--
-- Preorder:     A, B, D, E, C
-- Postorder:    D, E, B, C, A
-- Level-order:  A, B, C, D, E

--- Return nodes in preorder (parent before children).
--
-- Uses an explicit stack. Children are pushed in reverse sorted order
-- so that the smallest child is popped (visited) first.
--
-- @return table An array of node strings in preorder.
function Tree:preorder()
    local result = {}
    local stack = {self._root}

    while #stack > 0 do
        -- Pop from the top of the stack.
        local node = stack[#stack]
        stack[#stack] = nil
        result[#result + 1] = node

        -- Get children sorted, then push in reverse order so the
        -- lexicographically smallest child ends up on top of the stack.
        local children = self._graph:successors(node)
        table.sort(children)
        for i = #children, 1, -1 do
            stack[#stack + 1] = children[i]
        end
    end

    return result
end

--- Return nodes in postorder (children before parent).
--
-- Uses a recursive helper. Children are visited in sorted order.
--
-- @return table An array of node strings in postorder.
function Tree:postorder()
    local result = {}
    self:_postorder_recursive(self._root, result)
    return result
end

--- Recursive helper for postorder traversal.
-- @param node string The current node.
-- @param result table The accumulator array.
function Tree:_postorder_recursive(node, result)
    local children = self._graph:successors(node)
    table.sort(children)
    for _, child in ipairs(children) do
        self:_postorder_recursive(child, result)
    end
    result[#result + 1] = node
end

--- Return nodes in level-order (breadth-first).
--
-- Classic BFS using a queue. Children are visited in sorted order.
-- Level-order visits all nodes at depth 0, then depth 1, then depth 2,
-- and so on.
--
-- @return table An array of node strings in level-order.
function Tree:level_order()
    local result = {}
    local queue = {self._root}
    local head = 1

    while head <= #queue do
        local node = queue[head]
        head = head + 1
        result[#result + 1] = node

        local children = self._graph:successors(node)
        table.sort(children)
        for _, child in ipairs(children) do
            queue[#queue + 1] = child
        end
    end

    return result
end

-- =========================================================================
-- Utilities
-- =========================================================================

--- Return the path from the root to the given node.
--
-- The path is an array of nodes starting at the root and ending at the
-- target node. For the root itself, the path is just {root}.
--
-- @param node string The target node.
-- @return table An array of node strings from root to node.
--         On error, returns nil + error table.
function Tree:path_to(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    -- Walk from the node up to the root, collecting ancestors.
    local path = {}
    local current = node

    while current ~= nil do
        path[#path + 1] = current
        current = self:parent(current)
    end

    -- Reverse the path so it goes root -> node.
    local n = #path
    for i = 1, math.floor(n / 2) do
        path[i], path[n - i + 1] = path[n - i + 1], path[i]
    end

    return path
end

--- Return the lowest common ancestor (LCA) of nodes a and b.
--
-- The LCA is the deepest node that is an ancestor of both a and b.
-- Algorithm: compute paths from root to each node, then walk the
-- paths in parallel. The last node where they agree is the LCA.
--
-- Examples using the tree:
--
--         A
--        / \
--       B   C
--      / \   \
--     D   E   F
--     |
--     G
--
--   LCA(D, E) = B   (siblings share their parent)
--   LCA(G, F) = A   (nodes in different subtrees share the root)
--   LCA(B, D) = B   (ancestor-descendant: the ancestor is the LCA)
--   LCA(D, D) = D   (same node is its own LCA)
--
-- @param a string The first node.
-- @param b string The second node.
-- @return string The LCA node.
--         On error, returns nil + error table.
function Tree:lca(a, b)
    if not self._graph:has_node(a) then
        return nil, NodeNotFoundError(a)
    end
    if not self._graph:has_node(b) then
        return nil, NodeNotFoundError(b)
    end

    local path_a = self:path_to(a)
    local path_b = self:path_to(b)

    local lca_node = self._root
    local min_len = math.min(#path_a, #path_b)

    for i = 1, min_len do
        if path_a[i] == path_b[i] then
            lca_node = path_a[i]
        else
            break
        end
    end

    return lca_node
end

--- Extract the subtree rooted at the given node.
--
-- Returns a NEW Tree object. The original tree is not modified.
-- The subtree contains the given node as root, plus all its descendants
-- with the same structure.
--
-- @param node string The root of the subtree to extract.
-- @return Tree A new independent tree.
--         On error, returns nil + error table.
function Tree:subtree(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local new_tree = Tree.new(node)
    local queue = {node}
    local head = 1

    while head <= #queue do
        local current = queue[head]
        head = head + 1

        local children = self._graph:successors(current)
        table.sort(children)
        for _, child in ipairs(children) do
            new_tree:add_child(current, child)
            queue[#queue + 1] = child
        end
    end

    return new_tree
end

-- =========================================================================
-- Visualization
-- =========================================================================

--- Render the tree as an ASCII art diagram.
--
-- Produces output like:
--
--     Program
--     +-- Assignment
--     |   +-- BinaryOp
--     |   +-- Name
--     +-- Print
--
-- Uses Unicode box-drawing characters for a clean appearance:
--   - branch connector (non-last child)
--   - end connector (last child)
--   - vertical line (continuing branch)
--
-- @return string The ASCII art representation.
function Tree:to_ascii()
    local lines = {}
    self:_ascii_recursive(self._root, "", "", lines)
    return table.concat(lines, "\n")
end

--- Recursive helper for to_ascii.
--
-- @param node string The current node.
-- @param prefix string The prefix for this node's line.
-- @param child_prefix string The prefix for this node's children's lines.
-- @param lines table The accumulator array of line strings.
function Tree:_ascii_recursive(node, prefix, child_prefix, lines)
    lines[#lines + 1] = prefix .. node
    local children = self._graph:successors(node)
    table.sort(children)

    for i, child in ipairs(children) do
        if i < #children then
            -- Not the last child: use branch connector and vertical line
            -- for subsequent children.
            self:_ascii_recursive(
                child,
                child_prefix .. "\xe2\x94\x9c\xe2\x94\x80\xe2\x94\x80 ",
                child_prefix .. "\xe2\x94\x82   ",
                lines
            )
        else
            -- Last child: use end connector and spaces for subsequent
            -- children.
            self:_ascii_recursive(
                child,
                child_prefix .. "\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80 ",
                child_prefix .. "    ",
                lines
            )
        end
    end
end

-- =========================================================================
-- Graph access
-- =========================================================================

--- Return the underlying directed graph.
--
-- This provides access to graph-level operations like topological sort,
-- cycle detection, and edge queries. The tree's structural invariants
-- (single parent, no cycles) are guaranteed by the Tree methods, so
-- direct graph manipulation may break those invariants.
--
-- @return DirectedGraph The underlying graph.
function Tree:graph()
    return self._graph
end

--- Return a string representation showing root and size.
-- @return string A human-readable summary.
function Tree:__tostring()
    return string.format('Tree(root=%q, size=%d)', self._root, self:size())
end

-- =========================================================================
-- Module export
-- =========================================================================

local tree = {}

tree.VERSION = "0.1.0"

-- The main Tree class.
tree.Tree = Tree

-- Error constructors -- exported so tests and other modules can use them.
tree.NodeNotFoundError = NodeNotFoundError
tree.DuplicateNodeError = DuplicateNodeError
tree.RootRemovalError = RootRemovalError

return tree
