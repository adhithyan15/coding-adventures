local M = {}

M.VERSION = "0.1.0"

local function validate_matrix(name, matrix)
    if #matrix == 0 then error(name .. " must contain at least one row") end
    local width = #matrix[1]
    if width == 0 then error(name .. " must contain at least one column") end
    for i = 1, #matrix do
        if #matrix[i] ~= width then error(name .. " must be rectangular") end
    end
    return #matrix, width
end

local function activate(value, activation)
    activation = activation or "linear"
    if activation == "linear" then return value end
    if activation == "sigmoid" then
        if value >= 0 then
            local z = math.exp(-value)
            return 1.0 / (1.0 + z)
        end
        local z = math.exp(value)
        return z / (1.0 + z)
    end
    error("unsupported activation: " .. tostring(activation))
end

local function derivative_from_output(output, activation)
    activation = activation or "linear"
    if activation == "linear" then return 1.0 end
    if activation == "sigmoid" then return output * (1.0 - output) end
    error("unsupported activation: " .. tostring(activation))
end

function M.predict_with_parameters(inputs, weights, biases, activation)
    local sample_count, input_count = validate_matrix("inputs", inputs)
    local weight_rows, output_count = validate_matrix("weights", weights)
    if input_count ~= weight_rows then error("input column count must match weight row count") end
    if #biases ~= output_count then error("bias count must match output count") end

    local predictions = {}
    for row = 1, sample_count do
        predictions[row] = {}
        for output = 1, output_count do
            local total = biases[output]
            for input = 1, input_count do
                total = total + inputs[row][input] * weights[input][output]
            end
            predictions[row][output] = activate(total, activation)
        end
    end
    return predictions
end

function M.train_one_epoch_with_matrices(inputs, targets, weights, biases, learning_rate, activation)
    local sample_count, input_count = validate_matrix("inputs", inputs)
    local target_rows, output_count = validate_matrix("targets", targets)
    local weight_rows, weight_cols = validate_matrix("weights", weights)
    if target_rows ~= sample_count then error("inputs and targets must have the same row count") end
    if weight_rows ~= input_count or weight_cols ~= output_count then error("weights must be shaped input_count x output_count") end
    if #biases ~= output_count then error("bias count must match output count") end

    local predictions = M.predict_with_parameters(inputs, weights, biases, activation)
    local scale = 2.0 / (sample_count * output_count)
    local errors, deltas, loss_total = {}, {}, 0.0
    for row = 1, sample_count do
        errors[row], deltas[row] = {}, {}
        for output = 1, output_count do
            local err = predictions[row][output] - targets[row][output]
            errors[row][output] = err
            deltas[row][output] = scale * err * derivative_from_output(predictions[row][output], activation)
            loss_total = loss_total + err * err
        end
    end

    local weight_gradients, next_weights = {}, {}
    for input = 1, input_count do
        weight_gradients[input], next_weights[input] = {}, {}
        for output = 1, output_count do
            local gradient = 0.0
            for row = 1, sample_count do
                gradient = gradient + inputs[row][input] * deltas[row][output]
            end
            weight_gradients[input][output] = gradient
            next_weights[input][output] = weights[input][output] - learning_rate * gradient
        end
    end

    local bias_gradients, next_biases = {}, {}
    for output = 1, output_count do
        local gradient = 0.0
        for row = 1, sample_count do
            gradient = gradient + deltas[row][output]
        end
        bias_gradients[output] = gradient
        next_biases[output] = biases[output] - learning_rate * gradient
    end

    return {
        predictions = predictions,
        errors = errors,
        weight_gradients = weight_gradients,
        bias_gradients = bias_gradients,
        next_weights = next_weights,
        next_biases = next_biases,
        loss = loss_total / (sample_count * output_count),
    }
end

function M.new(input_count, output_count, activation)
    local weights = {}
    for input = 1, input_count do
        weights[input] = {}
        for output = 1, output_count do
            weights[input][output] = 0.0
        end
    end
    local biases = {}
    for output = 1, output_count do biases[output] = 0.0 end
    return { weights = weights, biases = biases, activation = activation or "linear" }
end

local function zero_biases(output_count)
    local biases = {}
    for output = 1, output_count do biases[output] = 0.0 end
    return biases
end

function M.with_shape(input_count, output_count, activation)
    local model = M.new(input_count, output_count, activation)
    model.biases = zero_biases(output_count)
    return model
end

function M.predict(model, inputs)
    return M.predict_with_parameters(inputs, model.weights, model.biases, model.activation)
end

function M.fit(model, inputs, targets, learning_rate, epochs)
    local history = {}
    for epoch = 1, epochs do
        local step = M.train_one_epoch_with_matrices(inputs, targets, model.weights, model.biases, learning_rate, model.activation)
        model.weights = step.next_weights
        model.biases = step.next_biases
        history[epoch] = step
    end
    return history
end

return M
