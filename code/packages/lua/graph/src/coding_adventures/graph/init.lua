local function node_sort_key(node)
    local value_type = type(node)
    if value_type == "string" or value_type == "number" or value_type == "boolean" then
        return value_type .. ":" .. tostring(node)
    end
    return value_type .. ":" .. tostring(node)
end

local function compare_nodes(left, right)
    local left_key = node_sort_key(left)
    local right_key = node_sort_key(right)
    if left_key < right_key then
        return -1
    elseif left_key > right_key then
        return 1
    end
    return 0
end

local function compare_nodes_less(left, right)
    return compare_nodes(left, right) < 0
end

local function canonical_endpoints(left, right)
    if compare_nodes(left, right) <= 0 then
        return left, right
    end
    return right, left
end

local function edge_key(left, right)
    local first, second = canonical_endpoints(left, right)
    return node_sort_key(first) .. "\0" .. node_sort_key(second)
end

local function copy_properties(properties)
    local copy = {}
    for key, value in pairs(properties or {}) do
        copy[key] = value
    end
    return copy
end

local function merge_properties(target, properties)
    for key, value in pairs(properties or {}) do
        target[key] = value
    end
end

local function NodeNotFoundError(node)
    return {
        type = "node_not_found",
        node = node,
        message = string.format("node not found: %q", tostring(node)),
    }
end

local function EdgeNotFoundError(left, right)
    return {
        type = "edge_not_found",
        left = left,
        right = right,
        message = string.format("edge not found: %q -- %q", tostring(left), tostring(right)),
    }
end

local function DisconnectedGraphError()
    return {
        type = "disconnected_graph",
        message = "graph is not connected and has no spanning tree",
    }
end

local GraphRepr = {
    ADJACENCY_LIST = "adjacency_list",
    ADJACENCY_MATRIX = "adjacency_matrix",
}

local Graph = {}
Graph.__index = Graph

function Graph.new(opts)
    opts = opts or {}
    local repr = opts.repr or GraphRepr.ADJACENCY_LIST
    local self = setmetatable({}, Graph)
    self._repr = repr
    self._adj = {}
    self._node_list = {}
    self._node_index = {}
    self._matrix = {}
    self._graph_properties = {}
    self._node_properties = {}
    self._edge_properties = {}
    return self
end

function Graph:repr()
    return self._repr
end

function Graph:add_node(node, properties)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        if self._adj[node] == nil then
            self._adj[node] = {}
        end
        self._node_properties[node] = self._node_properties[node] or {}
        merge_properties(self._node_properties[node], properties)
        return
    end

    if self._node_index[node] ~= nil then
        self._node_properties[node] = self._node_properties[node] or {}
        merge_properties(self._node_properties[node], properties)
        return
    end

    local index = #self._node_list + 1
    self._node_list[index] = node
    self._node_index[node] = index

    for _, row in ipairs(self._matrix) do
        row[index] = nil
    end

    local new_row = {}
    for i = 1, index do
        new_row[i] = nil
    end
    self._matrix[index] = new_row
    self._node_properties[node] = self._node_properties[node] or {}
    merge_properties(self._node_properties[node], properties)
end

function Graph:remove_node(node)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local neighbors = self._adj[node]
        if neighbors == nil then
            return nil, NodeNotFoundError(node)
        end

        for neighbor, _ in pairs(neighbors) do
            self._adj[neighbor][node] = nil
            self._edge_properties[edge_key(node, neighbor)] = nil
        end
        self._adj[node] = nil
        self._node_properties[node] = nil
        return true
    end

    local index = self._node_index[node]
    if index == nil then
        return nil, NodeNotFoundError(node)
    end

    for _, other in ipairs(self._node_list) do
        self._edge_properties[edge_key(node, other)] = nil
    end
    self._node_properties[node] = nil
    table.remove(self._node_list, index)
    table.remove(self._matrix, index)
    for _, row in ipairs(self._matrix) do
        table.remove(row, index)
    end

    self._node_index = {}
    for i, current in ipairs(self._node_list) do
        self._node_index[current] = i
    end
    return true
end

function Graph:has_node(node)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        return self._adj[node] ~= nil
    end
    return self._node_index[node] ~= nil
end

function Graph:nodes()
    local result = {}
    if self._repr == GraphRepr.ADJACENCY_LIST then
        for node, _ in pairs(self._adj) do
            result[#result + 1] = node
        end
    else
        for i = 1, #self._node_list do
            result[i] = self._node_list[i]
        end
    end
    table.sort(result, compare_nodes_less)
    return result
end

function Graph:size()
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local count = 0
        for _ in pairs(self._adj) do
            count = count + 1
        end
        return count
    end
    return #self._node_list
end

function Graph:add_edge(left, right, weight, properties)
    weight = weight == nil and 1.0 or weight
    self:add_node(left)
    self:add_node(right)

    if self._repr == GraphRepr.ADJACENCY_LIST then
        self._adj[left][right] = weight
        self._adj[right][left] = weight
        self._edge_properties[edge_key(left, right)] = self._edge_properties[edge_key(left, right)] or {}
        merge_properties(self._edge_properties[edge_key(left, right)], properties)
        self._edge_properties[edge_key(left, right)].weight = weight
        return
    end

    local left_index = self._node_index[left]
    local right_index = self._node_index[right]
    self._matrix[left_index][right_index] = weight
    self._matrix[right_index][left_index] = weight
    self._edge_properties[edge_key(left, right)] = self._edge_properties[edge_key(left, right)] or {}
    merge_properties(self._edge_properties[edge_key(left, right)], properties)
    self._edge_properties[edge_key(left, right)].weight = weight
end

function Graph:remove_edge(left, right)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local left_neighbors = self._adj[left]
        local right_neighbors = self._adj[right]
        if left_neighbors == nil or right_neighbors == nil or left_neighbors[right] == nil then
            return nil, EdgeNotFoundError(left, right)
        end

        left_neighbors[right] = nil
        right_neighbors[left] = nil
        self._edge_properties[edge_key(left, right)] = nil
        return true
    end

    local left_index = self._node_index[left]
    local right_index = self._node_index[right]
    if left_index == nil or right_index == nil or self._matrix[left_index][right_index] == nil then
        return nil, EdgeNotFoundError(left, right)
    end

    self._matrix[left_index][right_index] = nil
    self._matrix[right_index][left_index] = nil
    self._edge_properties[edge_key(left, right)] = nil
    return true
end

function Graph:has_edge(left, right)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local neighbors = self._adj[left]
        return neighbors ~= nil and neighbors[right] ~= nil
    end

    local left_index = self._node_index[left]
    local right_index = self._node_index[right]
    return left_index ~= nil and right_index ~= nil and self._matrix[left_index][right_index] ~= nil
end

function Graph:edge_weight(left, right)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local neighbors = self._adj[left]
        if neighbors == nil or neighbors[right] == nil then
            return nil, EdgeNotFoundError(left, right)
        end
        return neighbors[right]
    end

    local left_index = self._node_index[left]
    local right_index = self._node_index[right]
    if left_index == nil or right_index == nil or self._matrix[left_index][right_index] == nil then
        return nil, EdgeNotFoundError(left, right)
    end
    return self._matrix[left_index][right_index]
end

function Graph:graph_properties()
    return copy_properties(self._graph_properties)
end

function Graph:set_graph_property(key, value)
    self._graph_properties[key] = value
    return true
end

function Graph:remove_graph_property(key)
    self._graph_properties[key] = nil
    return true
end

function Graph:node_properties(node)
    if not self:has_node(node) then
        return nil, NodeNotFoundError(node)
    end
    return copy_properties(self._node_properties[node])
end

function Graph:set_node_property(node, key, value)
    if not self:has_node(node) then
        return nil, NodeNotFoundError(node)
    end
    self._node_properties[node] = self._node_properties[node] or {}
    self._node_properties[node][key] = value
    return true
end

function Graph:remove_node_property(node, key)
    if not self:has_node(node) then
        return nil, NodeNotFoundError(node)
    end
    if self._node_properties[node] ~= nil then
        self._node_properties[node][key] = nil
    end
    return true
end

function Graph:edge_properties(left, right)
    local weight, err = self:edge_weight(left, right)
    if weight == nil then
        return nil, err
    end
    local properties = copy_properties(self._edge_properties[edge_key(left, right)])
    properties.weight = weight
    return properties
end

function Graph:set_edge_property(left, right, key, value)
    if not self:has_edge(left, right) then
        return nil, EdgeNotFoundError(left, right)
    end
    if key == "weight" then
        if type(value) ~= "number" then
            return nil, { type = "invalid_property", message = "edge property 'weight' must be numeric" }
        end
        self:_set_edge_weight(left, right, value)
    end
    self._edge_properties[edge_key(left, right)] = self._edge_properties[edge_key(left, right)] or {}
    self._edge_properties[edge_key(left, right)][key] = value
    return true
end

function Graph:remove_edge_property(left, right, key)
    if not self:has_edge(left, right) then
        return nil, EdgeNotFoundError(left, right)
    end
    if key == "weight" then
        self:_set_edge_weight(left, right, 1.0)
        self._edge_properties[edge_key(left, right)] = self._edge_properties[edge_key(left, right)] or {}
        self._edge_properties[edge_key(left, right)].weight = 1.0
        return true
    end
    if self._edge_properties[edge_key(left, right)] ~= nil then
        self._edge_properties[edge_key(left, right)][key] = nil
    end
    return true
end

function Graph:_set_edge_weight(left, right, weight)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        self._adj[left][right] = weight
        self._adj[right][left] = weight
        return
    end

    local left_index = self._node_index[left]
    local right_index = self._node_index[right]
    self._matrix[left_index][right_index] = weight
    self._matrix[right_index][left_index] = weight
end

function Graph:edges()
    local result = {}
    local seen = {}

    if self._repr == GraphRepr.ADJACENCY_LIST then
        for left, neighbors in pairs(self._adj) do
            for right, weight in pairs(neighbors) do
                local first, second = canonical_endpoints(left, right)
                local key = node_sort_key(first) .. "\0" .. node_sort_key(second)
                if not seen[key] then
                    seen[key] = true
                    result[#result + 1] = { first, second, weight }
                end
            end
        end
    else
        for row = 1, #self._node_list do
            for col = row, #self._node_list do
                local weight = self._matrix[row][col]
                if weight ~= nil then
                    result[#result + 1] = { self._node_list[row], self._node_list[col], weight }
                end
            end
        end
    end

    table.sort(result, function(left, right)
        if left[3] ~= right[3] then
            return left[3] < right[3]
        end
        local by_left = compare_nodes(left[1], right[1])
        if by_left ~= 0 then
            return by_left < 0
        end
        return compare_nodes(left[2], right[2]) < 0
    end)
    return result
end

function Graph:neighbors(node)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local neighbors = self._adj[node]
        if neighbors == nil then
            return nil, NodeNotFoundError(node)
        end

        local result = {}
        for neighbor, _ in pairs(neighbors) do
            result[#result + 1] = neighbor
        end
        table.sort(result, compare_nodes_less)
        return result
    end

    local index = self._node_index[node]
    if index == nil then
        return nil, NodeNotFoundError(node)
    end

    local result = {}
    for col = 1, #self._node_list do
        if self._matrix[index][col] ~= nil then
            result[#result + 1] = self._node_list[col]
        end
    end
    table.sort(result, compare_nodes_less)
    return result
end

function Graph:neighbors_weighted(node)
    if self._repr == GraphRepr.ADJACENCY_LIST then
        local neighbors = self._adj[node]
        if neighbors == nil then
            return nil, NodeNotFoundError(node)
        end

        local copy = {}
        for neighbor, weight in pairs(neighbors) do
            copy[neighbor] = weight
        end
        return copy
    end

    local index = self._node_index[node]
    if index == nil then
        return nil, NodeNotFoundError(node)
    end

    local result = {}
    for col = 1, #self._node_list do
        local weight = self._matrix[index][col]
        if weight ~= nil then
            result[self._node_list[col]] = weight
        end
    end
    return result
end

function Graph:degree(node)
    local neighbors, err = self:neighbors(node)
    if not neighbors then
        return nil, err
    end
    return #neighbors
end

function Graph:__tostring()
    return string.format("Graph(nodes=%d, edges=%d, repr=%s)", self:size(), #self:edges(), self._repr)
end

local function bfs(graph, start)
    if not graph:has_node(start) then
        return nil, NodeNotFoundError(start)
    end

    local visited = { [start] = true }
    local queue = { start }
    local head = 1
    local result = {}

    while head <= #queue do
        local node = queue[head]
        head = head + 1
        result[#result + 1] = node

        local neighbors = graph:neighbors(node)
        for _, neighbor in ipairs(neighbors) do
            if not visited[neighbor] then
                visited[neighbor] = true
                queue[#queue + 1] = neighbor
            end
        end
    end

    return result
end

local function dfs(graph, start)
    if not graph:has_node(start) then
        return nil, NodeNotFoundError(start)
    end

    local visited = {}
    local stack = { start }
    local result = {}

    while #stack > 0 do
        local node = table.remove(stack)
        if not visited[node] then
            visited[node] = true
            result[#result + 1] = node

            local neighbors = graph:neighbors(node)
            for i = #neighbors, 1, -1 do
                local neighbor = neighbors[i]
                if not visited[neighbor] then
                    stack[#stack + 1] = neighbor
                end
            end
        end
    end

    return result
end

local function is_connected(graph)
    local nodes = graph:nodes()
    if #nodes == 0 then
        return true
    end
    local visited = bfs(graph, nodes[1])
    return #visited == #nodes
end

local function connected_components(graph)
    local result = {}
    local visited = {}

    for _, node in ipairs(graph:nodes()) do
        if not visited[node] then
            local component = bfs(graph, node)
            for _, member in ipairs(component) do
                visited[member] = true
            end
            result[#result + 1] = component
        end
    end

    return result
end

local function has_cycle(graph)
    local visited = {}

    local function visit(node, parent)
        visited[node] = true
        local neighbors = graph:neighbors(node)
        for _, neighbor in ipairs(neighbors) do
            if not visited[neighbor] then
                if visit(neighbor, node) then
                    return true
                end
            elseif neighbor ~= parent then
                return true
            end
        end
        return false
    end

    for _, node in ipairs(graph:nodes()) do
        if not visited[node] and visit(node, nil) then
            return true
        end
    end

    return false
end

local function shortest_path(graph, start, goal)
    if not graph:has_node(start) then
        return nil, NodeNotFoundError(start)
    end
    if not graph:has_node(goal) then
        return nil, NodeNotFoundError(goal)
    end
    if start == goal then
        return { start }
    end

    local inf = math.huge
    local distances = {}
    local previous = {}
    local frontier = {}
    local sequence = 0

    for _, node in ipairs(graph:nodes()) do
        distances[node] = inf
    end
    distances[start] = 0
    frontier[1] = { priority = 0, seq = 0, node = start }

    local function pop_min()
        local best_index = 1
        for i = 2, #frontier do
            local left = frontier[i]
            local right = frontier[best_index]
            if left.priority < right.priority or
               (left.priority == right.priority and left.seq < right.seq) then
                best_index = i
            end
        end
        return table.remove(frontier, best_index)
    end

    while #frontier > 0 do
        local current = pop_min()
        if current.priority <= distances[current.node] then
            if current.node == goal then
                break
            end

            local neighbors = graph:neighbors_weighted(current.node)
            local keys = {}
            for neighbor, _ in pairs(neighbors) do
                keys[#keys + 1] = neighbor
            end
            table.sort(keys, compare_nodes_less)

            for _, neighbor in ipairs(keys) do
                local next_distance = distances[current.node] + neighbors[neighbor]
                if next_distance < distances[neighbor] then
                    distances[neighbor] = next_distance
                    previous[neighbor] = current.node
                    sequence = sequence + 1
                    frontier[#frontier + 1] = {
                        priority = next_distance,
                        seq = sequence,
                        node = neighbor,
                    }
                end
            end
        end
    end

    if distances[goal] == inf then
        return {}
    end

    local path = {}
    local current = goal
    while current ~= nil do
        table.insert(path, 1, current)
        current = previous[current]
    end
    return path
end

local function minimum_spanning_tree(graph)
    if graph:size() <= 1 then
        return {}
    end
    if not is_connected(graph) then
        return nil, DisconnectedGraphError()
    end

    local parent = {}
    local rank = {}
    for _, node in ipairs(graph:nodes()) do
        parent[node] = node
        rank[node] = 0
    end

    local function find(node)
        if parent[node] ~= node then
            parent[node] = find(parent[node])
        end
        return parent[node]
    end

    local function union(left, right)
        local left_root = find(left)
        local right_root = find(right)
        if left_root == right_root then
            return
        end

        if rank[left_root] < rank[right_root] then
            parent[left_root] = right_root
        elseif rank[left_root] > rank[right_root] then
            parent[right_root] = left_root
        else
            parent[right_root] = left_root
            rank[left_root] = rank[left_root] + 1
        end
    end

    local result = {}
    for _, edge in ipairs(graph:edges()) do
        if find(edge[1]) ~= find(edge[2]) then
            union(edge[1], edge[2])
            result[#result + 1] = edge
            if #result == graph:size() - 1 then
                break
            end
        end
    end

    return result
end

local graph = {
    VERSION = "0.1.0",
    Graph = Graph,
    GraphRepr = GraphRepr,
    NodeNotFoundError = NodeNotFoundError,
    EdgeNotFoundError = EdgeNotFoundError,
    DisconnectedGraphError = DisconnectedGraphError,
    bfs = bfs,
    dfs = dfs,
    is_connected = is_connected,
    connected_components = connected_components,
    has_cycle = has_cycle,
    shortest_path = shortest_path,
    minimum_spanning_tree = minimum_spanning_tree,
}

return graph
