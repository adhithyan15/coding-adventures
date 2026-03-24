-- directed_graph.lua -- A Minimal Directed Graph for Dependency Resolution
-- =========================================================================
--
-- This is a self-contained directed graph implementation used by the build
-- tool's dependency resolver. It stores adjacency lists in both directions
-- (forward for successors, reverse for predecessors) so that traversals in
-- either direction are O(1) per edge.
--
-- The key operations are:
--   - add_node / add_edge   -- Build the graph incrementally.
--   - independent_groups    -- Kahn's algorithm for topological levels.
--   - transitive_closure    -- All reachable nodes from a starting node.
--   - transitive_dependents -- All nodes that transitively depend on a node.
--
-- In Lua, we use tables for everything. A set is a table where keys are
-- the elements and values are `true`. An adjacency list is a table mapping
-- node names to their neighbor sets.

local DirectedGraph = {}
DirectedGraph.__index = DirectedGraph

--- Create a new empty directed graph.
--
-- The graph stores two adjacency tables:
--   forward[node] = {neighbor = true, ...}   -- edges FROM node
--   reverse[node] = {neighbor = true, ...}   -- edges TO node
--
-- @return DirectedGraph A new graph instance.
function DirectedGraph.new()
    local self = setmetatable({}, DirectedGraph)
    self._forward = {}
    self._reverse = {}
    return self
end

--- Ensure a node exists in the graph.
--
-- If the node already exists, this is a no-op. We touch both the forward
-- and reverse adjacency tables so the node appears in `nodes()` even if
-- it has no edges.
--
-- @param node string The node identifier.
function DirectedGraph:add_node(node)
    if not self._forward[node] then
        self._forward[node] = {}
    end
    if not self._reverse[node] then
        self._reverse[node] = {}
    end
end

--- Add a directed edge from `from_node` to `to_node`.
--
-- Both nodes are implicitly added if they don't exist yet.
--
-- @param from_node string The source node.
-- @param to_node string The target node.
function DirectedGraph:add_edge(from_node, to_node)
    self:add_node(from_node)
    self:add_node(to_node)
    self._forward[from_node][to_node] = true
    self._reverse[to_node][from_node] = true
end

--- Check whether a node exists in the graph.
--
-- @param node string The node to check.
-- @return boolean
function DirectedGraph:has_node(node)
    return self._forward[node] ~= nil
end

--- Return all node identifiers as a list.
--
-- @return table A list of node name strings.
function DirectedGraph:nodes()
    local result = {}
    for node in pairs(self._forward) do
        result[#result + 1] = node
    end
    table.sort(result)
    return result
end

--- Return nodes that this node has edges TO.
--
-- @param node string
-- @return table A list of successor node names.
function DirectedGraph:successors(node)
    local result = {}
    for successor in pairs(self._forward[node] or {}) do
        result[#result + 1] = successor
    end
    return result
end

--- Return nodes that have edges TO this node.
--
-- @param node string
-- @return table A list of predecessor node names.
function DirectedGraph:predecessors(node)
    local result = {}
    for predecessor in pairs(self._reverse[node] or {}) do
        result[#result + 1] = predecessor
    end
    return result
end

--- All nodes reachable from `node` (excluding itself).
--
-- We do a simple iterative depth-first traversal following forward edges.
--
-- @param node string
-- @return table A set of reachable node names (keys are names, values are true).
function DirectedGraph:transitive_closure(node)
    if not self._forward[node] then
        return {}
    end

    local visited = {}
    local stack = {}
    for successor in pairs(self._forward[node]) do
        stack[#stack + 1] = successor
        visited[successor] = true
    end

    while #stack > 0 do
        local current = table.remove(stack)
        for successor in pairs(self._forward[current] or {}) do
            if not visited[successor] then
                visited[successor] = true
                stack[#stack + 1] = successor
            end
        end
    end

    return visited
end

--- All nodes that transitively depend on `node`.
--
-- This walks REVERSE edges: it finds every node that (directly or
-- indirectly) has `node` as a dependency.
--
-- @param node string
-- @return table A set of dependent node names.
function DirectedGraph:transitive_dependents(node)
    if not self._reverse[node] then
        return {}
    end

    local visited = {}
    local stack = {}
    for predecessor in pairs(self._reverse[node]) do
        stack[#stack + 1] = predecessor
        visited[predecessor] = true
    end

    while #stack > 0 do
        local current = table.remove(stack)
        for predecessor in pairs(self._reverse[current] or {}) do
            if not visited[predecessor] then
                visited[predecessor] = true
                stack[#stack + 1] = predecessor
            end
        end
    end

    return visited
end

--- Count the in-degree (number of predecessors) for a node.
--
-- @param node string
-- @return integer
function DirectedGraph:_in_degree(node)
    local count = 0
    for _ in pairs(self._reverse[node] or {}) do
        count = count + 1
    end
    return count
end

--- Partition nodes into parallel execution levels (Kahn's algorithm).
--
-- Returns a list of lists (levels). Each level contains nodes whose
-- in-degree is zero after removing all nodes in previous levels. Nodes
-- within a level can be built in parallel because they have no
-- dependencies on each other.
--
-- @return table A list of lists of node names.
-- @error If the graph contains a cycle.
function DirectedGraph:independent_groups()
    -- Compute in-degrees for all nodes.
    local in_degree = {}
    for node in pairs(self._forward) do
        in_degree[node] = self:_in_degree(node)
    end

    -- Start with all zero-in-degree nodes, sorted for determinism.
    local current_level = {}
    for node, degree in pairs(in_degree) do
        if degree == 0 then
            current_level[#current_level + 1] = node
        end
    end
    table.sort(current_level)

    local groups = {}
    local processed = 0

    while #current_level > 0 do
        groups[#groups + 1] = current_level
        processed = processed + #current_level

        local next_level_set = {}
        for _, node in ipairs(current_level) do
            for successor in pairs(self._forward[node]) do
                in_degree[successor] = in_degree[successor] - 1
                if in_degree[successor] == 0 then
                    next_level_set[successor] = true
                end
            end
        end

        current_level = {}
        for node in pairs(next_level_set) do
            current_level[#current_level + 1] = node
        end
        table.sort(current_level)
    end

    -- Count total nodes.
    local total = 0
    for _ in pairs(self._forward) do
        total = total + 1
    end

    if processed ~= total then
        error("Dependency graph contains a cycle")
    end

    return groups
end

return DirectedGraph
