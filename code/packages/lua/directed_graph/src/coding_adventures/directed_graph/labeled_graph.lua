-- labeled_graph — Labeled Directed Graph
-- =======================================
--
-- A LabeledGraph extends the basic directed graph with edge labels.
-- Each edge can have one or more string labels, turning the graph into
-- a "multigraph-like" structure where the same pair of nodes can be
-- connected by edges with different semantic meanings.
--
-- # Why labeled edges?
--
-- In a build system, you might want to distinguish between different
-- kinds of dependencies:
--
--   - "compile" dependency: package A needs package B at compile time
--   - "test"    dependency: package A needs package B only for testing
--   - "runtime" dependency: package A needs package B at runtime
--
-- With labeled edges, you can query "what are A's compile-time
-- dependencies?" without conflating them with test-only dependencies.
--
-- # Architecture: composition over inheritance
--
-- Rather than duplicating the adjacency-map logic from DirectedGraph,
-- LabeledGraph wraps a DirectedGraph and adds a label map on top:
--
--     +---------------------------------------------------+
--     | LabeledGraph                                      |
--     |                                                   |
--     |  +-------------------------+                      |
--     |  | graph (DirectedGraph)   |  <- handles nodes,   |
--     |  |   forward adjacency     |    edges, algorithms  |
--     |  |   reverse adjacency     |                      |
--     |  +-------------------------+                      |
--     |                                                   |
--     |  labels["from\0to"] = { label1=true, label2=true } |
--     |    (from, to) -> set of label strings             |
--     |                                                   |
--     +---------------------------------------------------+
--
-- The underlying DirectedGraph stores a single edge between any two
-- nodes, regardless of how many labels that edge carries. When all
-- labels for an edge are removed, the underlying edge is also removed.
--
-- All algorithm methods (topological_sort, has_cycle, transitive_closure,
-- etc.) delegate to the underlying DirectedGraph, so they work identically.
--
-- # Label map key encoding
--
-- In Go, the label map key is [2]string{from, to} — a fixed-size array.
-- Lua tables can't use arrays as keys (they'd be compared by reference,
-- not value). Instead, we concatenate from and to with a null byte
-- separator: from .. "\0" .. to. The null byte is safe because node names
-- are strings that shouldn't contain null bytes.

-- Helper: make a label map key from (from, to) pair.
-- We use null-byte separation to avoid collisions.
-- E.g., ("A", "B") -> "A\0B"
local function label_key(from, to)
    return from .. "\0" .. to
end

-- Error constructors — duplicated from init.lua to avoid circular require.
-- These are lightweight table constructors, so duplication is acceptable.

local function NodeNotFoundError(node)
    return {
        type = "node_not_found",
        node = node,
        message = string.format("node not found: %q", node),
    }
end

local function EdgeNotFoundError(from, to)
    return {
        type = "edge_not_found",
        from = from,
        to = to,
        message = string.format("edge not found: %q -> %q", from, to),
    }
end

local function LabelNotFoundError(from, to, label)
    return {
        type = "label_not_found",
        from = from,
        to = to,
        label = label,
        message = string.format("label %q not found on edge %q -> %q", label, from, to),
    }
end

-- =========================================================================
-- LabeledGraph class
-- =========================================================================

local LabeledGraph = {}
LabeledGraph.__index = LabeledGraph

--- Create an empty labeled directed graph.
-- Self-loops are prohibited (inherits from DirectedGraph.new()).
--
-- @return LabeledGraph A new empty labeled graph.
function LabeledGraph.new()
    local dg = require("coding_adventures.directed_graph")
    local self = setmetatable({}, LabeledGraph)
    self._graph = dg.DirectedGraph.new()
    self._labels = {}  -- label_key(from,to) -> {label1=true, label2=true}
    return self
end

--- Create an empty labeled directed graph that permits self-loops.
--
-- @return LabeledGraph A new empty labeled graph allowing self-loops.
function LabeledGraph.new_allow_self_loops()
    local dg = require("coding_adventures.directed_graph")
    local self = setmetatable({}, LabeledGraph)
    self._graph = dg.DirectedGraph.new_allow_self_loops()
    self._labels = {}
    return self
end

-- =========================================================================
-- Node operations — delegate to underlying DirectedGraph
-- =========================================================================

--- Add a node to the graph. No-op if the node already exists.
-- @param node string The node identifier.
function LabeledGraph:add_node(node)
    self._graph:add_node(node)
end

--- Remove a node and all its incident edges (including labels).
--
-- When a node is removed, we must also clean up any label entries for
-- edges that touched that node. We iterate over all label keys and
-- remove any that reference the deleted node.
--
-- @param node string The node to remove.
-- @return true on success, or nil + error.
function LabeledGraph:remove_node(node)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    -- Clean up labels for all edges involving this node.
    -- We collect keys first to avoid mutating the table during iteration.
    local keys_to_delete = {}
    for key, _ in pairs(self._labels) do
        -- Key format is "from\0to". Check if either part is our node.
        local sep = key:find("\0", 1, true)
        local from = key:sub(1, sep - 1)
        local to = key:sub(sep + 1)
        if from == node or to == node then
            keys_to_delete[#keys_to_delete + 1] = key
        end
    end
    for _, key in ipairs(keys_to_delete) do
        self._labels[key] = nil
    end

    return self._graph:remove_node(node)
end

--- Check whether a node exists in the graph.
-- @param node string The node to check.
-- @return boolean
function LabeledGraph:has_node(node)
    return self._graph:has_node(node)
end

--- Return all nodes in sorted order (deterministic).
-- @return table A sorted array of node strings.
function LabeledGraph:nodes()
    return self._graph:nodes()
end

--- Return the number of nodes in the graph.
-- @return integer
function LabeledGraph:size()
    return self._graph:size()
end

-- =========================================================================
-- Labeled edge operations
-- =========================================================================
--
-- Each edge in a LabeledGraph carries one or more labels. add_edge
-- requires a label; if you want multiple labels on the same edge,
-- call add_edge multiple times with different labels.
--
-- The underlying DirectedGraph tracks whether an edge exists at all (for
-- algorithm purposes). The label map tracks which labels are on each edge.

--- Add a directed edge from `from` to `to` with the given label.
--
-- If the edge already exists (possibly with different labels), the new
-- label is added to the existing set — the edge is not duplicated in
-- the underlying graph.
--
-- If the edge does not yet exist, it is created in the underlying graph
-- and the label is recorded.
--
-- Raises an error on self-loops if the underlying graph prohibits them.
--
-- @param from string The source node.
-- @param to string The destination node.
-- @param label string The label for this edge.
function LabeledGraph:add_edge(from, to, label)
    -- This may raise an error if self-loops are not allowed.
    self._graph:add_edge(from, to)

    local key = label_key(from, to)
    if not self._labels[key] then
        self._labels[key] = {}
    end
    self._labels[key][label] = true
end

--- Remove a specific label from the edge from->to.
--
-- If this was the last label on the edge, the underlying edge is also
-- removed from the graph. If other labels remain, only the specified
-- label is removed and the edge persists.
--
-- @param from string The source node.
-- @param to string The destination node.
-- @param label string The label to remove.
-- @return true on success, or nil + error.
function LabeledGraph:remove_edge(from, to, label)
    local key = label_key(from, to)
    local label_set = self._labels[key]

    if not label_set or not self._graph:has_edge(from, to) then
        return nil, EdgeNotFoundError(from, to)
    end

    if not label_set[label] then
        return nil, LabelNotFoundError(from, to, label)
    end

    label_set[label] = nil

    -- If no labels remain, remove the underlying edge entirely.
    local remaining = false
    for _ in pairs(label_set) do
        remaining = true
        break
    end

    if not remaining then
        self._labels[key] = nil
        return self._graph:remove_edge(from, to)
    end

    return true
end

--- Check whether there's any edge from `from` to `to` (regardless of label).
-- @param from string The source node.
-- @param to string The destination node.
-- @return boolean
function LabeledGraph:has_edge(from, to)
    return self._graph:has_edge(from, to)
end

--- Check whether there's an edge from `from` to `to` with a specific label.
-- @param from string The source node.
-- @param to string The destination node.
-- @param label string The label to check for.
-- @return boolean
function LabeledGraph:has_edge_with_label(from, to, label)
    local key = label_key(from, to)
    local label_set = self._labels[key]
    if label_set then
        return label_set[label] == true
    end
    return false
end

--- Return all edges as {from, to, label} triples, sorted deterministically.
--
-- If an edge has multiple labels, it appears once per label.
--
-- Example: if edge A->B has labels "compile" and "test", the output
-- includes both {"A", "B", "compile"} and {"A", "B", "test"}.
--
-- @return table An array of {from, to, label} arrays.
function LabeledGraph:edges()
    local result = {}
    for key, label_set in pairs(self._labels) do
        local sep = key:find("\0", 1, true)
        local from = key:sub(1, sep - 1)
        local to = key:sub(sep + 1)
        for label, _ in pairs(label_set) do
            result[#result + 1] = {from, to, label}
        end
    end
    table.sort(result, function(a, b)
        if a[1] ~= b[1] then return a[1] < b[1] end
        if a[2] ~= b[2] then return a[2] < b[2] end
        return a[3] < b[3]
    end)
    return result
end

--- Return the set of labels on the edge from->to.
--
-- Returns an empty table if the edge doesn't exist. Returns a COPY
-- of the internal label set to prevent callers from mutating internal state.
--
-- @param from string The source node.
-- @param to string The destination node.
-- @return table A set (table with boolean values) of labels.
function LabeledGraph:labels(from, to)
    local key = label_key(from, to)
    local label_set = self._labels[key]
    if label_set then
        -- Return a copy to prevent callers from mutating internal state.
        local result = {}
        for l, _ in pairs(label_set) do
            result[l] = true
        end
        return result
    end
    return {}
end

-- =========================================================================
-- Neighbor queries with label filtering
-- =========================================================================

--- Return the direct successors of a node (any label).
-- @param node string The node to query.
-- @return table Sorted array of successor strings, or nil + error.
function LabeledGraph:successors(node)
    return self._graph:successors(node)
end

--- Return successors connected by edges with the given label.
--
-- For example, if A->B has label "compile" and A->C has label "test",
-- then successors_with_label("A", "compile") returns {"B"}.
--
-- @param node string The node to query.
-- @param label string The label to filter by.
-- @return table Sorted array of successor strings, or nil + error.
function LabeledGraph:successors_with_label(node, label)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local succs, err = self._graph:successors(node)
    if not succs then return nil, err end

    local result = {}
    for _, succ in ipairs(succs) do
        local key = label_key(node, succ)
        local label_set = self._labels[key]
        if label_set and label_set[label] then
            result[#result + 1] = succ
        end
    end
    table.sort(result)
    return result
end

--- Return the direct predecessors of a node (any label).
-- @param node string The node to query.
-- @return table Sorted array of predecessor strings, or nil + error.
function LabeledGraph:predecessors(node)
    return self._graph:predecessors(node)
end

--- Return predecessors connected by edges with the given label.
--
-- @param node string The node to query.
-- @param label string The label to filter by.
-- @return table Sorted array of predecessor strings, or nil + error.
function LabeledGraph:predecessors_with_label(node, label)
    if not self._graph:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local preds, err = self._graph:predecessors(node)
    if not preds then return nil, err end

    local result = {}
    for _, pred in ipairs(preds) do
        local key = label_key(pred, node)
        local label_set = self._labels[key]
        if label_set and label_set[label] then
            result[#result + 1] = pred
        end
    end
    table.sort(result)
    return result
end

-- =========================================================================
-- Algorithm delegation
-- =========================================================================
--
-- All graph algorithms delegate to the underlying DirectedGraph. Labels
-- don't affect the structural algorithms — topological sort, cycle
-- detection, and transitive closure only care about whether edges exist,
-- not what they're labeled.

--- Return nodes in topological order.
-- @return table Sorted array of nodes, or nil + CycleError.
function LabeledGraph:topological_sort()
    return self._graph:topological_sort()
end

--- Return true if the graph contains a cycle.
-- @return boolean
function LabeledGraph:has_cycle()
    return self._graph:has_cycle()
end

--- Return all nodes reachable from the given node by following edges forward.
-- @param node string The starting node.
-- @return table A set of reachable nodes, or nil + error.
function LabeledGraph:transitive_closure(node)
    return self._graph:transitive_closure(node)
end

--- Return the underlying DirectedGraph, giving access to all base graph
-- methods (independent_groups, affected_nodes, etc.).
-- @return DirectedGraph The underlying graph.
function LabeledGraph:graph()
    return self._graph
end

return LabeledGraph
