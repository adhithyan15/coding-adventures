-- directed_graph — Directed graph with topological sort, cycle detection, and reachability
-- =======================================================================================
--
-- This package provides a directed graph data structure with algorithms for
-- topological sorting, cycle detection, transitive closure, independent group
-- computation, and affected-node analysis.
--
-- # What is a directed graph?
--
-- A directed graph (or "digraph") is a set of nodes connected by edges,
-- where each edge has a direction — it goes FROM one node TO another.
-- Think of it like a one-way street map: you can travel from A to B,
-- but that doesn't mean you can travel from B to A.
--
-- In a build system, nodes are packages and edges are dependencies:
-- if package A depends on package B, there's an edge from B to A
-- (B must be built before A).
--
-- # Why a directed graph?
--
-- The dependency relationships between packages form a DAG (Directed
-- Acyclic Graph). A DAG has no cycles — you can't have A depend on B
-- depend on C depend on A. The key algorithms on a DAG are:
--
--   - Topological sort: order nodes so every dependency comes before
--     the things that depend on it. This gives you a valid build order.
--
--   - Independent groups: partition nodes into "levels" where everything
--     at the same level can run in parallel. Level 0 has no dependencies.
--     Level 1 depends only on level 0. And so on.
--
--   - Affected nodes: given a set of changed nodes, find everything that
--     transitively depends on them. These are the packages that need
--     rebuilding when something changes.
--
-- # Architecture
--
-- The graph stores both forward edges (node -> its successors) and reverse
-- edges (node -> its predecessors) for efficient lookups in both directions.
-- This doubles memory usage but makes transitive-dependent queries O(V+E)
-- instead of requiring a full graph reversal.
--
-- # Self-loops
--
-- By default, self-loops (edges from a node to itself, like A->A) are
-- prohibited because they create trivial cycles, which makes topological
-- sorting impossible. However, some use cases genuinely need self-loops —
-- for example, modeling state machines where a state can transition to
-- itself, or representing "retry" semantics in a workflow graph.
--
-- Use DirectedGraph.new_allow_self_loops() to create a graph that permits
-- self-loops. The allow_self_loops flag is checked only in add_edge; all
-- other methods work correctly regardless of the flag's value.
--
-- # OOP pattern
--
-- We use the standard Lua metatable OOP pattern:
--
--     local DirectedGraph = {}
--     DirectedGraph.__index = DirectedGraph
--     function DirectedGraph.new() ... end
--
-- This gives us method dispatch via the : operator:
--
--     local g = DirectedGraph.new()
--     g:add_node("A")
--     g:add_edge("A", "B")
--
-- This is the coding-adventures monorepo's standard Lua style. Every
-- method is documented with its purpose, parameters, return values,
-- and error behavior.

-- =========================================================================
-- Error types
-- =========================================================================
--
-- We define structured error objects so callers can distinguish between
-- different failure modes. Each error type has a `type` field for
-- programmatic checking and a `message` field for human-readable output.
--
-- In Go, these are separate struct types with Error() methods. In Lua,
-- we use tables with a `type` discriminator field. Callers check errors
-- like this:
--
--     local ok, err = g:remove_node("X")
--     if not ok and err.type == "node_not_found" then
--         print("Node " .. err.node .. " does not exist")
--     end

--- Create a CycleError.
-- Returned when a cycle is detected in the graph. A cycle means there's
-- a circular dependency: A depends on B depends on C depends on A. This
-- makes it impossible to determine a valid build order.
local function CycleError()
    return {
        type = "cycle",
        message = "graph contains a cycle",
    }
end

--- Create a NodeNotFoundError.
-- Returned when operating on a node that doesn't exist in the graph.
-- @param node string The node that was not found.
local function NodeNotFoundError(node)
    return {
        type = "node_not_found",
        node = node,
        message = string.format("node not found: %q", node),
    }
end

--- Create an EdgeNotFoundError.
-- Returned when removing an edge that doesn't exist in the graph.
-- @param from string The source node of the missing edge.
-- @param to string The destination node of the missing edge.
local function EdgeNotFoundError(from, to)
    return {
        type = "edge_not_found",
        from = from,
        to = to,
        message = string.format("edge not found: %q -> %q", from, to),
    }
end

--- Create a LabelNotFoundError.
-- Returned when removing a label that doesn't exist on an edge.
-- Used by the LabeledGraph module.
-- @param from string The source node.
-- @param to string The destination node.
-- @param label string The label that was not found.
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
-- DirectedGraph
-- =========================================================================
--
-- The core directed graph data structure. Internally maintains two
-- adjacency maps:
--
--   forward[node] = { successor1 = true, successor2 = true, ... }
--   reverse[node] = { predecessor1 = true, predecessor2 = true, ... }
--
-- Both maps always have the same set of keys (all nodes in the graph).
-- When a node is added, it gets empty tables in both maps. When an edge
-- from->to is added, `to` is inserted into forward[from] and `from` is
-- inserted into reverse[to].

local DirectedGraph = {}
DirectedGraph.__index = DirectedGraph

--- Create an empty directed graph that prohibits self-loops.
--
-- This is the default constructor. If you try to add an edge from a node
-- to itself (e.g., g:add_edge("A", "A")), it will raise an error.
--
-- @return DirectedGraph A new empty graph.
function DirectedGraph.new()
    local self = setmetatable({}, DirectedGraph)
    self._forward = {}         -- node -> set of successors
    self._reverse = {}         -- node -> set of predecessors
    self._allow_self_loops = false
    return self
end

--- Create an empty directed graph that permits self-loops.
--
-- A self-loop is an edge from a node to itself, like A->A. This is useful
-- for modeling state machines, retry loops, or any domain where a node
-- can reference itself.
--
-- Note: a graph with self-loops will have cycles (a self-loop IS a cycle
-- of length 1), so topological_sort will return a CycleError and has_cycle
-- will return true.
--
-- @return DirectedGraph A new empty graph that allows self-loops.
function DirectedGraph.new_allow_self_loops()
    local self = setmetatable({}, DirectedGraph)
    self._forward = {}
    self._reverse = {}
    self._allow_self_loops = true
    return self
end

-- =========================================================================
-- Node operations
-- =========================================================================

--- Add a node to the graph. No-op if the node already exists.
--
-- Both the forward and reverse adjacency maps are initialized with empty
-- tables for the new node. This ensures that every node always has entries
-- in both maps, simplifying the logic in all other methods.
--
-- @param node string The node identifier to add.
function DirectedGraph:add_node(node)
    if not self._forward[node] then
        self._forward[node] = {}
        self._reverse[node] = {}
    end
end

--- Remove a node and all its incident edges.
--
-- "Incident edges" means all edges that touch this node — both edges
-- going TO this node (from predecessors) and edges going FROM this node
-- (to successors). We must clean up both directions to keep the forward
-- and reverse maps consistent.
--
-- @param node string The node to remove.
-- @return true on success, or nil + error table on failure.
function DirectedGraph:remove_node(node)
    if not self:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    -- Remove all edges TO this node: for each predecessor, delete
    -- this node from their forward (successor) set.
    for pred, _ in pairs(self._reverse[node]) do
        self._forward[pred][node] = nil
    end

    -- Remove all edges FROM this node: for each successor, delete
    -- this node from their reverse (predecessor) set.
    for succ, _ in pairs(self._forward[node]) do
        self._reverse[succ][node] = nil
    end

    -- Remove the node itself from both maps.
    self._forward[node] = nil
    self._reverse[node] = nil

    return true
end

--- Check whether a node exists in the graph.
-- @param node string The node to check.
-- @return boolean True if the node exists, false otherwise.
function DirectedGraph:has_node(node)
    return self._forward[node] ~= nil
end

--- Return all nodes in sorted order (deterministic).
--
-- Sorting ensures that the output is deterministic regardless of Lua's
-- table iteration order. This is important for tests and for algorithms
-- that need reproducible results (like topological sort).
--
-- @return table A sorted array of node strings.
function DirectedGraph:nodes()
    local result = {}
    for node, _ in pairs(self._forward) do
        result[#result + 1] = node
    end
    table.sort(result)
    return result
end

--- Return the number of nodes in the graph.
-- @return integer The node count.
function DirectedGraph:size()
    local count = 0
    for _ in pairs(self._forward) do
        count = count + 1
    end
    return count
end

-- =========================================================================
-- Edge operations
-- =========================================================================

--- Add a directed edge from `from` to `to`.
--
-- Both nodes are implicitly added if they don't exist. This is a
-- convenience so callers don't need to add_node before add_edge.
--
-- Self-loop behavior depends on how the graph was created:
--   - DirectedGraph.new()                   -> self-loops PROHIBITED (error)
--   - DirectedGraph.new_allow_self_loops()  -> self-loops ALLOWED
--
-- When self-loops are allowed, add_edge("A", "A") inserts A into both
-- the forward and reverse adjacency sets for A. This means:
--   - has_edge("A", "A") returns true
--   - successors("A") includes "A"
--   - predecessors("A") includes "A"
--   - has_cycle() returns true (a self-loop is a cycle of length 1)
--
-- @param from string The source node.
-- @param to string The destination node.
function DirectedGraph:add_edge(from, to)
    if from == to and not self._allow_self_loops then
        error(string.format("self-loop not allowed: %q", from))
    end
    self:add_node(from)
    self:add_node(to)
    self._forward[from][to] = true
    self._reverse[to][from] = true
end

--- Remove the edge from `from` to `to`.
--
-- The nodes themselves are NOT removed — only the edge between them.
-- After removal, both nodes still exist in the graph but are no longer
-- directly connected.
--
-- @param from string The source node.
-- @param to string The destination node.
-- @return true on success, or nil + error table on failure.
function DirectedGraph:remove_edge(from, to)
    if not self:has_edge(from, to) then
        return nil, EdgeNotFoundError(from, to)
    end
    self._forward[from][to] = nil
    self._reverse[to][from] = nil
    return true
end

--- Check whether an edge from `from` to `to` exists.
-- @param from string The source node.
-- @param to string The destination node.
-- @return boolean True if the edge exists, false otherwise.
function DirectedGraph:has_edge(from, to)
    local succs = self._forward[from]
    if succs then
        return succs[to] == true
    end
    return false
end

--- Return all edges as {from, to} pairs, sorted deterministically.
--
-- Edges are sorted first by `from`, then by `to`. This matches the Go
-- implementation's behavior and ensures reproducible output.
--
-- @return table An array of {from, to} arrays.
function DirectedGraph:edges()
    local result = {}
    for from, succs in pairs(self._forward) do
        for to, _ in pairs(succs) do
            result[#result + 1] = {from, to}
        end
    end
    table.sort(result, function(a, b)
        if a[1] ~= b[1] then
            return a[1] < b[1]
        end
        return a[2] < b[2]
    end)
    return result
end

-- =========================================================================
-- Neighbor queries
-- =========================================================================

--- Return the direct predecessors of a node (nodes with edges TO this node).
--
-- Predecessors are returned in sorted order for determinism.
--
-- @param node string The node to query.
-- @return table A sorted array of predecessor strings, or nil + error.
function DirectedGraph:predecessors(node)
    local preds = self._reverse[node]
    if not preds then
        return nil, NodeNotFoundError(node)
    end
    local result = {}
    for p, _ in pairs(preds) do
        result[#result + 1] = p
    end
    table.sort(result)
    return result
end

--- Return the direct successors of a node (nodes this node has edges TO).
--
-- Successors are returned in sorted order for determinism.
--
-- @param node string The node to query.
-- @return table A sorted array of successor strings, or nil + error.
function DirectedGraph:successors(node)
    local succs = self._forward[node]
    if not succs then
        return nil, NodeNotFoundError(node)
    end
    local result = {}
    for s, _ in pairs(succs) do
        result[#result + 1] = s
    end
    table.sort(result)
    return result
end

-- =========================================================================
-- Topological sort — Kahn's algorithm
-- =========================================================================
--
-- Kahn's algorithm works by repeatedly removing nodes with no incoming edges:
--
--  1. Find all nodes with in-degree 0 (no predecessors)
--  2. Remove them from the graph (conceptually), add to result
--  3. Their successors may now have in-degree 0 — repeat
--  4. If all nodes are removed, we have a valid ordering
--  5. If some nodes remain, there's a cycle
--
-- Why Kahn's instead of DFS-based topological sort?
--
-- Both are O(V+E), but Kahn's has a nice property: at each step, we can
-- sort the queue of in-degree-0 nodes to get a deterministic ordering.
-- DFS-based approaches produce valid topological orderings but the exact
-- order depends on iteration order of adjacency sets, which in Lua (like
-- Go maps) is non-deterministic.

--- Return nodes in topological order using Kahn's algorithm.
--
-- @return table A sorted array of node strings, or nil + CycleError.
function DirectedGraph:topological_sort()
    -- Step 1: Compute in-degrees.
    -- The in-degree of a node is the number of edges pointing TO it.
    -- We compute this from the reverse adjacency map.
    local in_degree = {}
    local total_nodes = 0
    for node, preds in pairs(self._reverse) do
        local count = 0
        for _ in pairs(preds) do
            count = count + 1
        end
        in_degree[node] = count
        total_nodes = total_nodes + 1
    end

    -- Step 2: Collect nodes with in-degree 0 (no predecessors).
    -- These are the "roots" — they can be processed first.
    local queue = {}
    for node, deg in pairs(in_degree) do
        if deg == 0 then
            queue[#queue + 1] = node
        end
    end
    table.sort(queue) -- deterministic processing order

    -- Step 3: Process the queue, decrementing in-degrees.
    local result = {}
    while #queue > 0 do
        -- Take the first node from the queue (FIFO).
        local node = table.remove(queue, 1)
        result[#result + 1] = node

        -- For each successor, decrement its in-degree.
        -- If in-degree hits 0, add to queue.
        local succs = {}
        for s, _ in pairs(self._forward[node]) do
            succs[#succs + 1] = s
        end
        table.sort(succs) -- deterministic order

        for _, succ in ipairs(succs) do
            in_degree[succ] = in_degree[succ] - 1
            if in_degree[succ] == 0 then
                queue[#queue + 1] = succ
                table.sort(queue) -- keep queue sorted for determinism
            end
        end
    end

    -- Step 4: Check if all nodes were processed.
    -- If not, the remaining nodes form one or more cycles.
    if #result ~= total_nodes then
        return nil, CycleError()
    end

    return result
end

-- =========================================================================
-- Cycle detection — DFS with three-color marking
-- =========================================================================
--
-- The three-color algorithm uses:
--   white (0) = unvisited
--   gray  (1) = in current DFS path (on the recursion stack)
--   black (2) = fully processed (all descendants explored)
--
-- If we encounter a gray node during DFS, we've found a back edge,
-- which means there's a cycle. This is the standard textbook algorithm
-- (see CLRS "Introduction to Algorithms", chapter 22).
--
-- Truth table for what happens when we visit a successor:
--
--   Successor color | Meaning                    | Action
--   ---------------+----------------------------+------------------
--   white (0)      | Not yet visited             | Recurse into it
--   gray  (1)      | On current path = CYCLE!    | Return true
--   black (2)      | Already fully explored      | Skip it

--- Return true if the graph contains a cycle.
--
-- @return boolean True if a cycle exists, false otherwise.
function DirectedGraph:has_cycle()
    local WHITE, GRAY, BLACK = 0, 1, 2
    local color = {}

    -- Recursive DFS function.
    -- Returns true if a cycle is found starting from `node`.
    local function dfs(node)
        color[node] = GRAY
        for succ, _ in pairs(self._forward[node]) do
            if color[succ] == GRAY then
                return true -- back edge = cycle!
            end
            if (color[succ] or WHITE) == WHITE then
                if dfs(succ) then
                    return true
                end
            end
        end
        color[node] = BLACK
        return false
    end

    -- Run DFS from every unvisited node.
    -- We sort nodes for deterministic traversal order.
    local sorted_nodes = self:nodes()
    for _, node in ipairs(sorted_nodes) do
        if (color[node] or WHITE) == WHITE then
            if dfs(node) then
                return true
            end
        end
    end

    return false
end

-- =========================================================================
-- Transitive closure — BFS reachability
-- =========================================================================
--
-- The transitive closure of a node is the set of all nodes reachable
-- from it by following edges forward. We use BFS (breadth-first search)
-- because it naturally explores nodes level by level and avoids the
-- stack-depth concerns of DFS on large graphs.
--
-- Example on a chain A -> B -> C -> D:
--   TransitiveClosure("A") = {B, C, D}  (everything reachable from A)
--   TransitiveClosure("C") = {D}        (only D is reachable from C)
--   TransitiveClosure("D") = {}         (D is a leaf, nothing reachable)

--- Return all nodes reachable from the given node by following edges forward.
--
-- The starting node itself is NOT included in the result (unless it's
-- reachable via a cycle or self-loop).
--
-- @param node string The starting node.
-- @return table A set (table with boolean values) of reachable nodes,
--         or nil + NodeNotFoundError.
function DirectedGraph:transitive_closure(node)
    if not self:has_node(node) then
        return nil, NodeNotFoundError(node)
    end

    local visited = {}
    local queue = {node}

    while #queue > 0 do
        local curr = table.remove(queue, 1)
        for succ, _ in pairs(self._forward[curr]) do
            if not visited[succ] then
                visited[succ] = true
                queue[#queue + 1] = succ
            end
        end
    end

    return visited
end

--- Return all nodes that transitively depend on the given node.
--
-- Edge convention: edges go FROM dependency TO dependent.
-- So logic-gates -> arithmetic means "arithmetic depends on logic-gates".
--
-- If "logic-gates" changes, its transitive dependents are everything
-- that directly or indirectly depends on it. These are found by following
-- forward edges.
--
-- Note: transitive_closure and transitive_dependents both follow forward
-- edges. They are the same operation. transitive_dependents exists as a
-- named alias to make build-system code more readable.
--
-- @param node string The node whose dependents to find.
-- @return table A set of dependent nodes, or nil + NodeNotFoundError.
function DirectedGraph:transitive_dependents(node)
    return self:transitive_closure(node)
end

-- =========================================================================
-- Independent groups — parallel execution levels
-- =========================================================================
--
-- This partitions nodes into levels by topological depth. Nodes at the
-- same level have no dependency on each other and can run in parallel.
--
-- This is the key method for a build system's parallel execution.
--
-- Example for a diamond graph (A->B, A->C, B->D, C->D):
--
--   Level 0: {A}      — no dependencies
--   Level 1: {B, C}   — depend only on A, can run in parallel
--   Level 2: {D}      — depends on B and C
--
-- The algorithm is a modified Kahn's: instead of processing one node at
-- a time, we process all in-degree-0 nodes as a batch (one level), then
-- decrement in-degrees and find the next batch.

--- Partition nodes into levels by topological depth.
--
-- @return table An array of arrays, where each inner array is a sorted
--         list of nodes at that level. Returns nil + CycleError if the
--         graph contains a cycle.
function DirectedGraph:independent_groups()
    -- Compute in-degrees.
    local in_degree = {}
    local total_nodes = 0
    for node, preds in pairs(self._reverse) do
        local count = 0
        for _ in pairs(preds) do
            count = count + 1
        end
        in_degree[node] = count
        total_nodes = total_nodes + 1
    end

    -- Collect initial in-degree-0 nodes.
    local queue = {}
    for node, deg in pairs(in_degree) do
        if deg == 0 then
            queue[#queue + 1] = node
        end
    end
    table.sort(queue)

    local levels = {}
    local processed = 0

    -- Process level by level.
    while #queue > 0 do
        -- All nodes in the current queue form one level.
        local level = {}
        for _, node in ipairs(queue) do
            level[#level + 1] = node
        end
        table.sort(level)
        levels[#levels + 1] = level
        processed = processed + #level

        -- Find the next level: decrement in-degrees of successors.
        local next_queue = {}
        for _, node in ipairs(queue) do
            for succ, _ in pairs(self._forward[node]) do
                in_degree[succ] = in_degree[succ] - 1
                if in_degree[succ] == 0 then
                    next_queue[#next_queue + 1] = succ
                end
            end
        end
        table.sort(next_queue)
        queue = next_queue
    end

    -- Check for cycles.
    if processed ~= total_nodes then
        return nil, CycleError()
    end

    return levels
end

-- =========================================================================
-- Affected nodes — change propagation
-- =========================================================================
--
-- Given a set of changed nodes, "affected" means: the changed nodes
-- themselves, plus everything that transitively depends on any of them.
--
-- This is used by the build tool: if you change logic-gates, the affected
-- set includes logic-gates + arithmetic + cpu-simulator + ...
--
-- Nodes in the `changed` set that don't exist in the graph are silently
-- ignored. This is intentional — the build tool may report changes in
-- files that aren't part of any known package.

--- Return the set of nodes affected by changes to the given nodes.
--
-- @param changed table A set (table with boolean values) of changed nodes.
-- @return table A set of affected nodes (including the changed nodes).
function DirectedGraph:affected_nodes(changed)
    local affected = {}
    for node, _ in pairs(changed) do
        if self:has_node(node) then
            affected[node] = true
            local deps, _ = self:transitive_dependents(node)
            if deps then
                for dep, _ in pairs(deps) do
                    affected[dep] = true
                end
            end
        end
    end
    return affected
end

--- Convenience wrapper that returns affected nodes as a sorted list.
--
-- @param changed table A set of changed nodes.
-- @return table A sorted array of affected node strings.
function DirectedGraph:affected_nodes_list(changed)
    local affected = self:affected_nodes(changed)
    local result = {}
    for node, _ in pairs(affected) do
        result[#result + 1] = node
    end
    table.sort(result)
    return result
end

-- =========================================================================
-- Module export
-- =========================================================================
--
-- We export the DirectedGraph class, the error constructors, and the
-- LabeledGraph and visualization submodules. The submodules are loaded
-- lazily via require() to avoid circular dependencies.

local directed_graph = {}

directed_graph.VERSION = "0.1.0"

-- The main graph class.
directed_graph.DirectedGraph = DirectedGraph

-- Error constructors — exported so tests and other modules can create them.
directed_graph.CycleError = CycleError
directed_graph.NodeNotFoundError = NodeNotFoundError
directed_graph.EdgeNotFoundError = EdgeNotFoundError
directed_graph.LabelNotFoundError = LabelNotFoundError

-- Register ourselves in package.loaded BEFORE requiring submodules.
-- This is critical: labeled_graph.lua needs to require us back to get
-- the DirectedGraph class. Without this early registration, Lua's
-- require() would see the module as "still loading" and return true
-- instead of the module table. By setting package.loaded explicitly,
-- the circular require works correctly.
package.loaded["coding_adventures.directed_graph"] = directed_graph

-- Submodules — can now safely require us back.
directed_graph.LabeledGraph = require("coding_adventures.directed_graph.labeled_graph")
directed_graph.visualization = require("coding_adventures.directed_graph.visualization")

return directed_graph
