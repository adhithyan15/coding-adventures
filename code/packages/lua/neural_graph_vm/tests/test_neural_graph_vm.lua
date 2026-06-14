package.path = "../src/?.lua;../src/?/init.lua;../../neural_network/src/?.lua;../../neural_network/src/?/init.lua;" .. package.path
local NN = require("coding_adventures.neural_network")
local VM = require("coding_adventures.neural_graph_vm")

local function tiny_graph()
    local graph = NN.create_neural_graph("tiny")
    NN.add_input(graph, "x0")
    NN.add_input(graph, "x1")
    NN.add_constant(graph, "bias", 1.0)
    NN.add_weighted_sum(graph, "sum", { NN.wi("x0", 0.25, "x0_to_sum"), NN.wi("x1", 0.75, "x1_to_sum"), NN.wi("bias", -1.0, "bias_to_sum") })
    NN.add_activation(graph, "relu", "sum", "relu", {}, "sum_to_relu")
    NN.add_output(graph, "out", "relu", "prediction", {}, "relu_to_out")
    return graph
end

describe("neural_graph_vm", function()
    it("runs the tiny weighted sum", function()
        local bytecode = VM.compile_neural_graph_to_bytecode(tiny_graph())
        assert.is_true(math.abs(VM.run_neural_bytecode_forward(bytecode, { x0 = 4.0, x1 = 8.0 }).prediction - 6.0) < 1e-9)
    end)
    it("runs xor", function()
        local bytecode = VM.compile_neural_network_to_bytecode(NN.create_xor_network())
        local cases = { {0, 0, 0}, {0, 1, 1}, {1, 0, 1}, {1, 1, 0} }
        for _, case in ipairs(cases) do
            local prediction = VM.run_neural_bytecode_forward(bytecode, { x0 = case[1], x1 = case[2] }).prediction
            if case[3] == 1 then assert.is_true(prediction > 0.99) else assert.is_true(prediction < 0.01) end
        end
    end)
end)
