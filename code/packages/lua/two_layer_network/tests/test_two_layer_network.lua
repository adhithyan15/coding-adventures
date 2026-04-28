package.path = "src/?.lua;src/?/init.lua;" .. package.path

local tln = require("coding_adventures.two_layer_network")

local inputs = {{0.0, 0.0}, {0.0, 1.0}, {1.0, 0.0}, {1.0, 1.0}}
local targets = {{0.0}, {1.0}, {1.0}, {0.0}}
local pass = tln.forward(inputs, tln.xor_warm_start_parameters())

assert(#pass.hidden_activations == 4)
assert(#pass.hidden_activations[1] == 2)
assert(pass.predictions[2][1] > 0.7)
assert(pass.predictions[1][1] < 0.3)

local step = tln.train_one_epoch(inputs, targets, tln.xor_warm_start_parameters(), 0.5)
assert(#step.input_to_hidden_weight_gradients == 2)
assert(#step.input_to_hidden_weight_gradients[1] == 2)
assert(#step.hidden_to_output_weight_gradients == 2)
assert(#step.hidden_to_output_weight_gradients[1] == 1)
