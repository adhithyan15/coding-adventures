-- ============================================================================
-- init.lua — Undirected Graph
-- ============================================================================
--
-- A complete undirected graph data structure implementation supporting:
--   - Two representations: adjacency list (default) and adjacency matrix
--   - Weighted edges: optional weights on edges (default 1.0)
--   - Core operations: add/remove nodes/edges, neighbor queries, degree
--   - Algorithms: BFS, DFS, shortest path, cycle detection, connected components,
--     minimum spanning tree, graph connectivity
--
-- All algorithms work identically on both representations because they only
-- call the Graph's public API.
--
-- ============================================================================

local M = {}
M.__index = M

-- Representation type constants
M.ADJACENCY_LIST = "adjacency_list"
M.ADJACENCY_MATRIX = "adjacency_matrix"

-- ============================================================================
-- Constructor
-- ============================================================================

--- Create a new empty graph with adjacency list representation (default).
function M.new()
    return M.new_with_repr(M.ADJACENCY_LIST)
end

--- Create a new empty graph with specified representation.
function M.new_with_repr(repr)
    local self = setmetatable({}, M)
    self.repr = repr

    if repr == M.ADJACENCY_LIST then
        -- adjacency[node] = {neighbor = weight, ...}
        self.adjacency = {}
    else
        -- Adjacency matrix representation
        self.node_list = {}   -- [1] = node1, [2] = node2, ...
        self.node_idx = {}    -- {node1 = 1, node2 = 2, ...}
        self.matrix = {}      -- {{0, w12}, {w12, 0}, ...}
    end

    return self
end

-- ============================================================================
-- Node Operations
-- ============================================================================

--- Add a node to the graph. No-op if the node already exists.
function M:add_node(node)
    if self.repr == M.ADJACENCY_LIST then
        if not self.adjacency[node] then
            self.adjacency[node] = {}
        end
    else
        if not self.node_idx[node] then
            local idx = #self.node_list + 1
            table.insert(self.node_list, node)
            self.node_idx[node] = idx

            -- Add new row and column of zeros
            for i = 1, #self.matrix do
                table.insert(self.matrix[i], 0.0)
            end
            local new_row = {}
            for i = 1, idx do
                table.insert(new_row, 0.0)
            end
            table.insert(self.matrix, new_row)
        end
    end
end

--- Remove a node and all its edges from the graph.
function M:remove_node(node)
    if not self:has_node(node) then
        error("Node not found: " .. tostring(node))
    end

    if self.repr == M.ADJACENCY_LIST then
        -- Remove all edges connected to this node
        for neighbor in pairs(self.adjacency[node]) do
            self.adjacency[neighbor][node] = nil
        end
        self.adjacency[node] = nil
    else
        local idx = self.node_idx[node]
        self.node_idx[node] = nil
        table.remove(self.node_list, idx)

        -- Update indices for nodes that shifted down
        for i = idx, #self.node_list do
            self.node_idx[self.node_list[i]] = i
        end

        -- Remove the row
        table.remove(self.matrix, idx)
        -- Remove the column from every remaining row
        for i = 1, #self.matrix do
            table.remove(self.matrix[i], idx)
        end
    end
end

--- Check if a node exists in the graph.
function M:has_node(node)
    if self.repr == M.ADJACENCY_LIST then
        return self.adjacency[node] ~= nil
    else
        return self.node_idx[node] ~= nil
    end
end

--- Get all nodes in the graph as a sorted table.
function M:nodes()
    local result
    if self.repr == M.ADJACENCY_LIST then
        result = {}
        for node in pairs(self.adjacency) do
            table.insert(result, node)
        end
    else
        result = {}
        for _, node in ipairs(self.node_list) do
            table.insert(result, node)
        end
    end

    table.sort(result, function(a, b)
        return tostring(a) < tostring(b)
    end)
    return result
end

--- Get the number of nodes in the graph.
function M:size()
    if self.repr == M.ADJACENCY_LIST then
        return self._count_keys(self.adjacency)
    else
        return #self.node_list
    end
end

-- Helper: count table keys
function M._count_keys(tbl)
    local count = 0
    for _ in pairs(tbl) do
        count = count + 1
    end
    return count
end

-- ============================================================================
-- Edge Operations
-- ============================================================================

--- Add an undirected edge between u and v with optional weight (default 1.0).
function M:add_edge(u, v, weight)
    weight = weight or 1.0

    self:add_node(u)
    self:add_node(v)

    if self.repr == M.ADJACENCY_LIST then
        self.adjacency[u][v] = weight
        self.adjacency[v][u] = weight
    else
        local i, j = self.node_idx[u], self.node_idx[v]
        self.matrix[i][j] = weight
        self.matrix[j][i] = weight
    end
end

--- Remove an edge between u and v.
function M:remove_edge(u, v)
    if not self:has_edge(u, v) then
        error("Edge not found: " .. tostring(u) .. " -- " .. tostring(v))
    end

    if self.repr == M.ADJACENCY_LIST then
        self.adjacency[u][v] = nil
        self.adjacency[v][u] = nil
    else
        local i, j = self.node_idx[u], self.node_idx[v]
        self.matrix[i][j] = 0.0
        self.matrix[j][i] = 0.0
    end
end

--- Check if an edge exists between u and v.
function M:has_edge(u, v)
    if self.repr == M.ADJACENCY_LIST then
        if not self.adjacency[u] then
            return false
        end
        return self.adjacency[u][v] ~= nil
    else
        if not self.node_idx[u] or not self.node_idx[v] then
            return false
        end
        local i, j = self.node_idx[u], self.node_idx[v]
        return self.matrix[i][j] ~= 0.0
    end
end

--- Get all edges as a table of {u, v, weight} tuples.
function M:edges()
    local result = {}

    if self.repr == M.ADJACENCY_LIST then
        local seen = {}
        for u, neighbors in pairs(self.adjacency) do
            for v, w in pairs(neighbors) do
                -- Canonical ordering
                local a, b = u, v
                if tostring(a) > tostring(b) then
                    a, b = b, a
                end
                local key = tostring(a) .. "|" .. tostring(b)
                if not seen[key] then
                    table.insert(result, {a, b, w})
                    seen[key] = true
                end
            end
        end
    else
        local n = #self.node_list
        for i = 1, n do
            for j = i + 1, n do
                local w = self.matrix[i][j]
                if w ~= 0.0 then
                    table.insert(result, {self.node_list[i], self.node_list[j], w})
                end
            end
        end
    end

    table.sort(result, function(a, b)
        if tostring(a[1]) ~= tostring(b[1]) then
            return tostring(a[1]) < tostring(b[1])
        end
        return tostring(a[2]) < tostring(b[2])
    end)

    return result
end

--- Get the weight of an edge, or error if it doesn't exist.
function M:edge_weight(u, v)
    if not self:has_edge(u, v) then
        error("Edge not found: " .. tostring(u) .. " -- " .. tostring(v))
    end

    if self.repr == M.ADJACENCY_LIST then
        return self.adjacency[u][v]
    else
        local i, j = self.node_idx[u], self.node_idx[v]
        return self.matrix[i][j]
    end
end

-- ============================================================================
-- Neighborhood Queries
-- ============================================================================

--- Get all neighbours of node as a sorted table.
function M:neighbors(node)
    if not self:has_node(node) then
        error("Node not found: " .. tostring(node))
    end

    local result = {}

    if self.repr == M.ADJACENCY_LIST then
        for neighbor in pairs(self.adjacency[node]) do
            table.insert(result, neighbor)
        end
    else
        local idx = self.node_idx[node]
        for j, w in ipairs(self.matrix[idx]) do
            if w ~= 0.0 then
                table.insert(result, self.node_list[j])
            end
        end
    end

    table.sort(result, function(a, b)
        return tostring(a) < tostring(b)
    end)

    return result
end

--- Get neighbors with their weights as a table {neighbor = weight, ...}.
function M:neighbors_weighted(node)
    if not self:has_node(node) then
        error("Node not found: " .. tostring(node))
    end

    local result = {}

    if self.repr == M.ADJACENCY_LIST then
        for neighbor, weight in pairs(self.adjacency[node]) do
            result[neighbor] = weight
        end
    else
        local idx = self.node_idx[node]
        for j, w in ipairs(self.matrix[idx]) do
            if w ~= 0.0 then
                result[self.node_list[j]] = w
            end
        end
    end

    return result
end

--- Get the degree (number of neighbors) of a node.
function M:degree(node)
    return #self:neighbors(node)
end

--- Check if every node can reach every other node.
function M:is_connected()
    if self:size() == 0 then
        return true
    end

    local start = self:nodes()[1]
    local reachable = M.bfs(self, start)
    return #reachable == self:size()
end

--- Get all connected components as a table of tables of nodes.
function M:connected_components()
    local unvisited = {}
    for _, node in ipairs(self:nodes()) do
        unvisited[node] = true
    end

    local components = {}

    while self._count_keys(unvisited) > 0 do
        local start
        for node in pairs(unvisited) do
            start = node
            break
        end

        local component = M.bfs(self, start)
        table.insert(components, component)

        for _, node in ipairs(component) do
            unvisited[node] = nil
        end
    end

    return components
end

-- ============================================================================
-- Algorithms (pure functions)
-- ============================================================================

--- BFS traversal: return nodes reachable from start in breadth-first order.
function M.bfs(graph, start)
    local visited = {}
    local queue = {start}
    local result = {}

    visited[start] = true

    local head = 1
    while head <= #queue do
        local node = queue[head]
        head = head + 1
        table.insert(result, node)

        for _, neighbor in ipairs(graph:neighbors(node)) do
            if not visited[neighbor] then
                visited[neighbor] = true
                table.insert(queue, neighbor)
            end
        end
    end

    return result
end

--- DFS traversal: return nodes reachable from start in depth-first order.
function M.dfs(graph, start)
    local visited = {}
    local stack = {start}
    local result = {}

    while #stack > 0 do
        local node = table.remove(stack)

        if not visited[node] then
            visited[node] = true
            table.insert(result, node)

            local neighbors = graph:neighbors(node)
            -- Reverse order for consistent output
            for i = #neighbors, 1, -1 do
                if not visited[neighbors[i]] then
                    table.insert(stack, neighbors[i])
                end
            end
        end
    end

    return result
end

--- Find the shortest (lowest-weight) path from start to end.
function M.shortest_path(graph, start, end_node)
    if start == end_node then
        if graph:has_node(start) then
            return {start}
        else
            return {}
        end
    end

    -- Check if all weights are 1.0
    local all_unit = true
    for _, edge in ipairs(graph:edges()) do
        if edge[3] ~= 1.0 then
            all_unit = false
            break
        end
    end

    if all_unit then
        return M._shortest_path_bfs(graph, start, end_node)
    else
        return M._shortest_path_dijkstra(graph, start, end_node)
    end
end

function M._shortest_path_bfs(graph, start, end_node)
    local parent = {}
    parent[start] = false
    local queue = {start}

    local head = 1
    while head <= #queue do
        local node = queue[head]
        head = head + 1

        if node == end_node then
            break
        end

        for _, neighbor in ipairs(graph:neighbors(node)) do
            if parent[neighbor] == nil then
                parent[neighbor] = node
                table.insert(queue, neighbor)
            end
        end
    end

    if parent[end_node] == nil then
        return {}
    end

    local path = {}
    local cur = end_node
    while cur ~= false do
        table.insert(path, 1, cur)
        cur = parent[cur]
    end

    return path
end

function M._shortest_path_dijkstra(graph, start, end_node)
    local INF = math.huge
    local dist = {}
    local parent = {}

    for _, node in ipairs(graph:nodes()) do
        dist[node] = INF
    end
    dist[start] = 0

    local pq = {{0.0, start}}
    local visited = {}

    while #pq > 0 do
        -- Find minimum in priority queue
        local min_idx = 1
        for i = 2, #pq do
            if pq[i][1] < pq[min_idx][1] then
                min_idx = i
            end
        end

        local d, node = pq[min_idx][1], pq[min_idx][2]
        table.remove(pq, min_idx)

        if d > dist[node] then
            goto continue
        end

        if node == end_node then
            break
        end

        visited[node] = true

        local neighbors_map = graph:neighbors_weighted(node)
        for neighbor, weight in pairs(neighbors_map) do
            if not visited[neighbor] then
                local new_dist = dist[node] + weight
                if new_dist < dist[neighbor] then
                    dist[neighbor] = new_dist
                    parent[neighbor] = node
                    table.insert(pq, {new_dist, neighbor})
                end
            end
        end

        ::continue::
    end

    if dist[end_node] == INF then
        return {}
    end

    local path = {}
    local cur = end_node
    while cur ~= nil do
        table.insert(path, 1, cur)
        cur = parent[cur]
    end

    return path
end

--- Check if the graph contains any cycle.
function M.has_cycle(graph)
    local visited = {}

    for _, start in ipairs(graph:nodes()) do
        if not visited[start] then
            local result, new_visited = M._has_cycle_from(graph, start, nil, visited)
            visited = new_visited

            if result then
                return true
            end
        end
    end

    return false
end

function M._has_cycle_from(graph, node, parent, visited)
    visited[node] = true

    for _, neighbor in ipairs(graph:neighbors(node)) do
        if not visited[neighbor] then
            local has_cycle, new_visited = M._has_cycle_from(graph, neighbor, node, visited)
            visited = new_visited

            if has_cycle then
                return true, visited
            end
        elseif neighbor ~= parent then
            -- Back edge: visited neighbor that isn't our parent → cycle
            return true, visited
        end
    end

    return false, visited
end

--- Get the minimum spanning tree using Kruskal's algorithm.
function M.minimum_spanning_tree(graph)
    local nodes = graph:nodes()

    if #nodes == 0 or #nodes == 1 then
        return {}
    end

    local edges = graph:edges()
    table.sort(edges, function(a, b) return a[3] < b[3] end)

    local uf = M._union_find_new(nodes)
    local mst = {}

    for _, edge in ipairs(edges) do
        local u, v, w = edge[1], edge[2], edge[3]

        if M._union_find_find(uf, u) ~= M._union_find_find(uf, v) then
            M._union_find_union(uf, u, v)
            table.insert(mst, edge)

            if #mst == #nodes - 1 then
                break
            end
        end
    end

    if #mst < #nodes - 1 then
        return nil  -- Not connected
    end

    return mst
end

-- ============================================================================
-- Union-Find (helper for Kruskal's algorithm)
-- ============================================================================

function M._union_find_new(nodes)
    local uf = {}
    for _, n in ipairs(nodes) do
        uf[n] = n
    end
    return uf
end

function M._union_find_find(uf, x)
    if uf[x] ~= x then
        uf[x] = M._union_find_find(uf, uf[x])  -- Path compression
    end
    return uf[x]
end

function M._union_find_union(uf, a, b)
    local ra = M._union_find_find(uf, a)
    local rb = M._union_find_find(uf, b)

    if ra ~= rb then
        uf[rb] = ra
    end
end

return M
