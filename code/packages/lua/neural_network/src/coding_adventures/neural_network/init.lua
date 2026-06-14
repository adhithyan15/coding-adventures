local M = {}
M.VERSION = "0.1.0"

local Graph = {}
Graph.__index = Graph

function Graph.new(name)
    local self = setmetatable({}, Graph)
    self.graph_properties = { ["nn.version"] = "0" }
    if name ~= nil then self.graph_properties["nn.name"] = name end
    self._nodes = {}
    self._node_properties = {}
    self._edges = {}
    self._next_edge_id = 0
    return self
end

function Graph:add_node(node, properties)
    properties = properties or {}
    if self._node_properties[node] == nil then
        table.insert(self._nodes, node)
        self._node_properties[node] = {}
    end
    for key, value in pairs(properties) do self._node_properties[node][key] = value end
end

function Graph:nodes()
    local out = {}
    for i, node in ipairs(self._nodes) do out[i] = node end
    return out
end

function Graph:edges()
    local out = {}
    for i, edge in ipairs(self._edges) do out[i] = edge end
    return out
end

function Graph:node_properties(node)
    local out = {}
    for key, value in pairs(self._node_properties[node] or {}) do out[key] = value end
    return out
end

function Graph:add_edge(from, to, weight, properties, edge_id)
    properties = properties or {}
    weight = weight or 1.0
    self:add_node(from, {})
    self:add_node(to, {})
    if edge_id == nil then
        edge_id = "e" .. tostring(self._next_edge_id)
        self._next_edge_id = self._next_edge_id + 1
    end
    local props = {}
    for key, value in pairs(properties) do props[key] = value end
    props.weight = weight
    table.insert(self._edges, { id = edge_id, from = from, to = to, weight = weight, properties = props })
    return edge_id
end

function Graph:incoming_edges(node)
    local incoming = {}
    for _, edge in ipairs(self._edges) do
        if edge.to == node then table.insert(incoming, edge) end
    end
    return incoming
end

function Graph:topological_sort()
    local indegree = {}
    for _, node in ipairs(self._nodes) do indegree[node] = 0 end
    for _, edge in ipairs(self._edges) do
        indegree[edge.from] = indegree[edge.from] or 0
        indegree[edge.to] = (indegree[edge.to] or 0) + 1
    end
    local ready = {}
    for node, degree in pairs(indegree) do if degree == 0 then table.insert(ready, node) end end
    table.sort(ready)
    local order = {}
    while #ready > 0 do
        local node = table.remove(ready, 1)
        table.insert(order, node)
        local released = {}
        for _, edge in ipairs(self._edges) do
            if edge.from == node then
                indegree[edge.to] = indegree[edge.to] - 1
                if indegree[edge.to] == 0 then table.insert(released, edge.to) end
            end
        end
        table.sort(released)
        for _, released_node in ipairs(released) do table.insert(ready, released_node) end
    end
    local count = 0
    for _ in pairs(indegree) do count = count + 1 end
    if #order ~= count then error("neural graph contains a cycle") end
    return order
end

M.Graph = Graph

local Network = {}
Network.__index = Network
function Network.new(name) return setmetatable({ graph = M.create_neural_graph(name) }, Network) end
function Network:input(node, input_name, properties) M.add_input(self.graph, node, input_name or node, properties or {}); return self end
function Network:constant(node, value, properties) M.add_constant(self.graph, node, value, properties or {}); return self end
function Network:weighted_sum(node, inputs, properties) M.add_weighted_sum(self.graph, node, inputs, properties or {}); return self end
function Network:activation(node, input, activation, properties, edge_id) M.add_activation(self.graph, node, input, activation, properties or {}, edge_id); return self end
function Network:output(node, input, output_name, properties, edge_id) M.add_output(self.graph, node, input, output_name or node, properties or {}, edge_id); return self end
M.Network = Network

function M.create_neural_graph(name) return Graph.new(name) end
function M.create_neural_network(name) return Network.new(name) end
function M.wi(from, weight, edge_id) return { from = from, weight = weight, edge_id = edge_id, properties = {} } end

local function merge(properties, extra)
    local out = {}
    for key, value in pairs(properties or {}) do out[key] = value end
    for key, value in pairs(extra) do out[key] = value end
    return out
end

function M.add_input(graph, node, input_name, properties)
    graph:add_node(node, merge(properties, { ["nn.op"] = "input", ["nn.input"] = input_name or node }))
end
function M.add_constant(graph, node, value, properties)
    if value ~= value then error("constant value must be finite") end
    graph:add_node(node, merge(properties, { ["nn.op"] = "constant", ["nn.value"] = value }))
end
function M.add_weighted_sum(graph, node, inputs, properties)
    graph:add_node(node, merge(properties, { ["nn.op"] = "weighted_sum" }))
    for _, input in ipairs(inputs) do graph:add_edge(input.from, node, input.weight or 1.0, input.properties or {}, input.edge_id) end
end
function M.add_activation(graph, node, input, activation, properties, edge_id)
    graph:add_node(node, merge(properties, { ["nn.op"] = "activation", ["nn.activation"] = activation }))
    return graph:add_edge(input, node, 1.0, {}, edge_id)
end
function M.add_output(graph, node, input, output_name, properties, edge_id)
    graph:add_node(node, merge(properties, { ["nn.op"] = "output", ["nn.output"] = output_name or node }))
    return graph:add_edge(input, node, 1.0, {}, edge_id)
end

function M.create_xor_network(name)
    return M.create_neural_network(name or "xor")
        :input("x0")
        :input("x1")
        :constant("bias", 1.0, { ["nn.role"] = "bias" })
        :weighted_sum("h_or_sum", { M.wi("x0", 20, "x0_to_h_or"), M.wi("x1", 20, "x1_to_h_or"), M.wi("bias", -10, "bias_to_h_or") }, { ["nn.layer"] = "hidden" })
        :activation("h_or", "h_or_sum", "sigmoid", { ["nn.layer"] = "hidden" }, "h_or_sum_to_h_or")
        :weighted_sum("h_nand_sum", { M.wi("x0", -20, "x0_to_h_nand"), M.wi("x1", -20, "x1_to_h_nand"), M.wi("bias", 30, "bias_to_h_nand") }, { ["nn.layer"] = "hidden" })
        :activation("h_nand", "h_nand_sum", "sigmoid", { ["nn.layer"] = "hidden" }, "h_nand_sum_to_h_nand")
        :weighted_sum("out_sum", { M.wi("h_or", 20, "h_or_to_out"), M.wi("h_nand", 20, "h_nand_to_out"), M.wi("bias", -30, "bias_to_out") }, { ["nn.layer"] = "output" })
        :activation("out_activation", "out_sum", "sigmoid", { ["nn.layer"] = "output" }, "out_sum_to_activation")
        :output("out", "out_activation", "prediction", { ["nn.layer"] = "output" }, "activation_to_out")
end

return M
