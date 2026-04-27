package.path = "src/?.lua;src/?/init.lua;" .. package.path

local sln = require("coding_adventures.single_layer_network")

local function near(actual, expected)
    assert(math.abs(actual - expected) <= 1e-6, tostring(actual) .. " != " .. tostring(expected))
end

local step = sln.train_one_epoch_with_matrices(
    {{1.0, 2.0}},
    {{3.0, 5.0}},
    {{0.0, 0.0}, {0.0, 0.0}},
    {0.0, 0.0},
    0.1
)
near(step.weight_gradients[1][1], -3.0)
near(step.weight_gradients[2][2], -10.0)
near(step.next_weights[1][1], 0.3)
near(step.next_weights[2][2], 1.0)

local model = sln.with_shape(3, 2)
local history = sln.fit(
    model,
    {{0.0, 0.0, 1.0}, {1.0, 2.0, 1.0}, {2.0, 1.0, 1.0}},
    {{1.0, -1.0}, {3.0, 2.0}, {4.0, 1.0}},
    0.05,
    500
)
assert(history[#history].loss < history[1].loss)
assert(#sln.predict(model, {{1.0, 1.0, 1.0}})[1] == 2)
