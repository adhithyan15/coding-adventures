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

local function sample_parameters(input_count, hidden_count)
    local input_to_hidden = {}
    for feature = 1, input_count do
        input_to_hidden[feature] = {}
        for hidden = 1, hidden_count do
            input_to_hidden[feature][hidden] = 0.17 * feature - 0.11 * hidden
        end
    end
    local hidden_biases = {}
    local hidden_to_output = {}
    for hidden = 1, hidden_count do
        hidden_biases[hidden] = 0.05 * (hidden - 2)
        hidden_to_output[hidden] = {0.13 * hidden - 0.25}
    end
    return {
        input_to_hidden_weights = input_to_hidden,
        hidden_biases = hidden_biases,
        hidden_to_output_weights = hidden_to_output,
        output_biases = {0.02},
    }
end

local cases = {
    {"XNOR", inputs, {{1.0}, {0.0}, {0.0}, {1.0}}, 3},
    {"absolute value", {{-1.0}, {-0.5}, {0.0}, {0.5}, {1.0}}, {{1.0}, {0.5}, {0.0}, {0.5}, {1.0}}, 4},
    {"piecewise pricing", {{0.1}, {0.3}, {0.5}, {0.7}, {0.9}}, {{0.12}, {0.25}, {0.55}, {0.88}, {0.88}}, 4},
    {"circle classifier", {{0.0, 0.0}, {0.5, 0.0}, {1.0, 1.0}, {-0.5, 0.5}, {-1.0, 0.0}}, {{1.0}, {1.0}, {0.0}, {1.0}, {0.0}}, 5},
    {"two moons", {{1.0, 0.0}, {0.0, 0.5}, {0.5, 0.85}, {0.5, -0.35}, {-1.0, 0.0}, {2.0, 0.5}}, {{0.0}, {1.0}, {0.0}, {1.0}, {0.0}, {1.0}}, 5},
    {"interaction features", {{0.2, 0.25, 0.0}, {0.6, 0.5, 1.0}, {1.0, 0.75, 1.0}, {1.0, 1.0, 0.0}}, {{0.08}, {0.72}, {0.96}, {0.76}}, 5},
}

for _, case in ipairs(cases) do
    local name, example_inputs, example_targets, hidden_count = case[1], case[2], case[3], case[4]
    local example_step = tln.train_one_epoch(
        example_inputs,
        example_targets,
        sample_parameters(#example_inputs[1], hidden_count),
        0.4
    )
    assert(example_step.loss >= 0.0, name)
    assert(#example_step.input_to_hidden_weight_gradients == #example_inputs[1], name)
    assert(#example_step.hidden_to_output_weight_gradients == hidden_count, name)
end
