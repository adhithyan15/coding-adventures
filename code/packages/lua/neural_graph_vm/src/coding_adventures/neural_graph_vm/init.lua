local M = {}
M.VERSION = "0.1.0"

function M.compile_neural_network_to_bytecode(network)
    return M.compile_neural_graph_to_bytecode(network.graph)
end

function M.compile_neural_graph_to_bytecode(graph)
    local values = {}
    local next_value_id = 0
    local function alloc()
        local id = "v" .. tostring(next_value_id)
        next_value_id = next_value_id + 1
        return id
    end
    local instructions = {}
    for _, node in ipairs(graph:topological_sort()) do
        local props = graph:node_properties(node)
        local op = props["nn.op"] or "weighted_sum"
        if op == "input" then
            local dst = alloc(); values[node] = dst
            table.insert(instructions, { op = "LOAD_INPUT", dst = dst, input_name = props["nn.input"] or node, source_node = node })
        elseif op == "constant" then
            local dst = alloc(); values[node] = dst
            table.insert(instructions, { op = "LOAD_CONST", dst = dst, value = props["nn.value"], source_node = node })
        elseif op == "weighted_sum" then
            local incoming = graph:incoming_edges(node)
            table.sort(incoming, function(a, b) return a.id < b.id end)
            local terms = {}
            for _, edge in ipairs(incoming) do
                local weight_value = alloc(); local term_value = alloc()
                table.insert(instructions, { op = "LOAD_EDGE_WEIGHT", dst = weight_value, edge_id = edge.id, source_edge = edge.id })
                table.insert(instructions, { op = "MUL", dst = term_value, left = values[edge.from], right = weight_value, source_edge = edge.id })
                table.insert(terms, term_value)
            end
            local dst = alloc(); values[node] = dst
            if #terms == 0 then table.insert(instructions, { op = "LOAD_CONST", dst = dst, value = 0.0, source_node = node })
            else table.insert(instructions, { op = "ADD", dst = dst, inputs = terms, source_node = node }) end
        elseif op == "activation" then
            local dst = alloc(); values[node] = dst
            table.insert(instructions, { op = "ACTIVATE", dst = dst, input = M.single_input_value(graph, values, node), activation = props["nn.activation"] or "relu", source_node = node })
        elseif op == "output" then
            local input = M.single_input_value(graph, values, node); values[node] = input
            table.insert(instructions, { op = "STORE_OUTPUT", output_name = props["nn.output"] or node, input = input, source_node = node })
        else error("unsupported neural graph op: " .. tostring(op)) end
    end
    local edges = {}
    for _, edge in ipairs(graph:edges()) do table.insert(edges, { id = edge.id, from = edge.from, to = edge.to, weight = edge.weight }) end
    return { magic = "CANN", version = 0, nodes = graph:nodes(), edges = edges, functions = { { id = "forward", kind = "forward", instructions = instructions } } }
end

function M.run_neural_bytecode_forward(module, inputs)
    local values = {}
    local edge_weights = {}
    for _, edge in ipairs(module.edges) do edge_weights[edge.id] = edge.weight end
    local outputs = {}
    local forward = module.functions[1]
    for _, inst in ipairs(forward.instructions) do
        if inst.op == "LOAD_INPUT" then values[inst.dst] = inputs[inst.input_name]
        elseif inst.op == "LOAD_CONST" then values[inst.dst] = inst.value or 0.0
        elseif inst.op == "LOAD_EDGE_WEIGHT" then values[inst.dst] = edge_weights[inst.edge_id] or 1.0
        elseif inst.op == "MUL" then values[inst.dst] = values[inst.left] * values[inst.right]
        elseif inst.op == "ADD" then local sum = 0.0; for _, id in ipairs(inst.inputs or {}) do sum = sum + values[id] end; values[inst.dst] = sum
        elseif inst.op == "ACTIVATE" then values[inst.dst] = M.apply_neural_activation(values[inst.input], inst.activation or "relu")
        elseif inst.op == "STORE_OUTPUT" then outputs[inst.output_name or "output"] = values[inst.input]
        else error("unsupported opcode: " .. tostring(inst.op)) end
    end
    return outputs
end

function M.apply_neural_activation(value, activation)
    if activation == "relu" then return value > 0 and value or 0.0 end
    if activation == "sigmoid" then return 1.0 / (1.0 + math.exp(-value)) end
    if activation == "tanh" then return math.tanh(value) end
    return value
end

function M.single_input_value(graph, values, node)
    local incoming = graph:incoming_edges(node)
    if #incoming ~= 1 then error("node " .. node .. " expects exactly one input") end
    return values[incoming[1].from]
end

return M
