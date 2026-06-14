package.path = "../src/?.lua;../src/?/init.lua;" .. package.path
local NN = require("coding_adventures.neural_network")

describe("neural_network", function()
    it("builds a tiny weighted graph", function()
        local graph = NN.create_neural_graph("tiny")
        NN.add_input(graph, "x0")
        NN.add_input(graph, "x1")
        NN.add_constant(graph, "bias", 1.0)
        NN.add_weighted_sum(graph, "sum", { NN.wi("x0", 0.25, "x0_to_sum"), NN.wi("x1", 0.75, "x1_to_sum"), NN.wi("bias", -1.0, "bias_to_sum") })
        NN.add_activation(graph, "relu", "sum", "relu", {}, "sum_to_relu")
        NN.add_output(graph, "out", "relu", "prediction", {}, "relu_to_out")
        assert.are.equal(3, #graph:incoming_edges("sum"))
        assert.are.equal("out", graph:topological_sort()[#graph:topological_sort()])
    end)
    it("builds xor with hidden edges", function()
        local network = NN.create_xor_network()
        local found = false
        for _, edge in ipairs(network.graph:edges()) do if edge.id == "h_or_to_out" then found = true end end
        assert.is_true(found)
    end)
end)
