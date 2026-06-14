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
    activation = activation or "sigmoid"
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

local function derivative(raw, activated, activation)
    activation = activation or "sigmoid"
    if activation == "linear" then return 1.0 end
    if activation == "sigmoid" then return activated * (1.0 - activated) end
    error("unsupported activation: " .. tostring(activation))
end

local function dot(left, right)
    local rows, width = validate_matrix("left", left)
    local right_rows, cols = validate_matrix("right", right)
    if width ~= right_rows then error("matrix shapes do not align") end
    local result = {}
    for row = 1, rows do
        result[row] = {}
        for col = 1, cols do
            local sum = 0.0
            for k = 1, width do
                sum = sum + left[row][k] * right[k][col]
            end
            result[row][col] = sum
        end
    end
    return result
end

local function transpose(matrix)
    local rows, cols = validate_matrix("matrix", matrix)
    local result = {}
    for col = 1, cols do
        result[col] = {}
        for row = 1, rows do
            result[col][row] = matrix[row][col]
        end
    end
    return result
end

local function add_biases(matrix, biases)
    local result = {}
    for row = 1, #matrix do
        result[row] = {}
        for col = 1, #matrix[row] do
            result[row][col] = matrix[row][col] + biases[col]
        end
    end
    return result
end

local function apply_activation(matrix, activation)
    local result = {}
    for row = 1, #matrix do
        result[row] = {}
        for col = 1, #matrix[row] do
            result[row][col] = activate(matrix[row][col], activation)
        end
    end
    return result
end

local function column_sums(matrix)
    local _, cols = validate_matrix("matrix", matrix)
    local sums = {}
    for col = 1, cols do sums[col] = 0.0 end
    for row = 1, #matrix do
        for col = 1, cols do sums[col] = sums[col] + matrix[row][col] end
    end
    return sums
end

local function mse(errors)
    local total, count = 0.0, 0
    for row = 1, #errors do
        for col = 1, #errors[row] do
            total = total + errors[row][col] * errors[row][col]
            count = count + 1
        end
    end
    return total / count
end

local function subtract_scaled(matrix, gradients, learning_rate)
    local result = {}
    for row = 1, #matrix do
        result[row] = {}
        for col = 1, #matrix[row] do
            result[row][col] = matrix[row][col] - learning_rate * gradients[row][col]
        end
    end
    return result
end

function M.xor_warm_start_parameters()
    return {
        input_to_hidden_weights = {{4.0, -4.0}, {4.0, -4.0}},
        hidden_biases = {-2.0, 6.0},
        hidden_to_output_weights = {{4.0}, {4.0}},
        output_biases = {-6.0},
    }
end

function M.forward(inputs, parameters, hidden_activation, output_activation)
    hidden_activation = hidden_activation or "sigmoid"
    output_activation = output_activation or "sigmoid"
    local hidden_raw = add_biases(dot(inputs, parameters.input_to_hidden_weights), parameters.hidden_biases)
    local hidden_activations = apply_activation(hidden_raw, hidden_activation)
    local output_raw = add_biases(dot(hidden_activations, parameters.hidden_to_output_weights), parameters.output_biases)
    local predictions = apply_activation(output_raw, output_activation)
    return {
        hidden_raw = hidden_raw,
        hidden_activations = hidden_activations,
        output_raw = output_raw,
        predictions = predictions,
    }
end

function M.train_one_epoch(inputs, targets, parameters, learning_rate, hidden_activation, output_activation)
    hidden_activation = hidden_activation or "sigmoid"
    output_activation = output_activation or "sigmoid"
    local sample_count = validate_matrix("inputs", inputs)
    local _, output_count = validate_matrix("targets", targets)
    local pass = M.forward(inputs, parameters, hidden_activation, output_activation)
    local scale = 2.0 / (sample_count * output_count)
    local errors, output_deltas = {}, {}
    for row = 1, sample_count do
        errors[row], output_deltas[row] = {}, {}
        for output = 1, output_count do
            local err = pass.predictions[row][output] - targets[row][output]
            errors[row][output] = err
            output_deltas[row][output] = scale * err * derivative(pass.output_raw[row][output], pass.predictions[row][output], output_activation)
        end
    end
    local h2o_gradients = dot(transpose(pass.hidden_activations), output_deltas)
    local output_bias_gradients = column_sums(output_deltas)
    local hidden_errors = dot(output_deltas, transpose(parameters.hidden_to_output_weights))
    local hidden_width = #parameters.hidden_biases
    local hidden_deltas = {}
    for row = 1, sample_count do
        hidden_deltas[row] = {}
        for hidden = 1, hidden_width do
            hidden_deltas[row][hidden] = hidden_errors[row][hidden] * derivative(pass.hidden_raw[row][hidden], pass.hidden_activations[row][hidden], hidden_activation)
        end
    end
    local i2h_gradients = dot(transpose(inputs), hidden_deltas)
    local hidden_bias_gradients = column_sums(hidden_deltas)
    local next_hidden_biases, next_output_biases = {}, {}
    for hidden = 1, hidden_width do next_hidden_biases[hidden] = parameters.hidden_biases[hidden] - learning_rate * hidden_bias_gradients[hidden] end
    for output = 1, output_count do next_output_biases[output] = parameters.output_biases[output] - learning_rate * output_bias_gradients[output] end
    return {
        predictions = pass.predictions,
        errors = errors,
        output_deltas = output_deltas,
        hidden_deltas = hidden_deltas,
        hidden_to_output_weight_gradients = h2o_gradients,
        output_bias_gradients = output_bias_gradients,
        input_to_hidden_weight_gradients = i2h_gradients,
        hidden_bias_gradients = hidden_bias_gradients,
        next_parameters = {
            input_to_hidden_weights = subtract_scaled(parameters.input_to_hidden_weights, i2h_gradients, learning_rate),
            hidden_biases = next_hidden_biases,
            hidden_to_output_weights = subtract_scaled(parameters.hidden_to_output_weights, h2o_gradients, learning_rate),
            output_biases = next_output_biases,
        },
        loss = mse(errors),
    }
end

return M
