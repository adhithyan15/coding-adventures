local M = {}

M.VERSION = "0.1.0"

local function validate_matrix(rows)
    if type(rows) ~= "table" or #rows == 0 or type(rows[1]) ~= "table" or #rows[1] == 0 then
        error("matrix must have at least one row and one column")
    end
    local width = #rows[1]
    for _, row in ipairs(rows) do
        if #row ~= width then
            error("all rows must have the same number of columns")
        end
    end
    return width
end

function M.fit_standard_scaler(rows)
    local width = validate_matrix(rows)
    local means = {}
    local standard_deviations = {}
    for col = 1, width do
        local sum = 0.0
        for _, row in ipairs(rows) do
            sum = sum + row[col]
        end
        means[col] = sum / #rows
    end

    for col = 1, width do
        local sum = 0.0
        for _, row in ipairs(rows) do
            local diff = row[col] - means[col]
            sum = sum + diff * diff
        end
        standard_deviations[col] = math.sqrt(sum / #rows)
    end

    return { means = means, standard_deviations = standard_deviations }
end

function M.transform_standard(rows, scaler)
    local width = validate_matrix(rows)
    if width ~= #scaler.means then
        error("matrix width must match scaler width")
    end
    local out = {}
    for row_index, row in ipairs(rows) do
        out[row_index] = {}
        for col = 1, width do
            local std = scaler.standard_deviations[col]
            out[row_index][col] = std == 0.0 and 0.0 or (row[col] - scaler.means[col]) / std
        end
    end
    return out
end

function M.fit_min_max_scaler(rows)
    local width = validate_matrix(rows)
    local minimums = {}
    local maximums = {}
    for col = 1, width do
        minimums[col] = rows[1][col]
        maximums[col] = rows[1][col]
        for row_index = 2, #rows do
            minimums[col] = math.min(minimums[col], rows[row_index][col])
            maximums[col] = math.max(maximums[col], rows[row_index][col])
        end
    end
    return { minimums = minimums, maximums = maximums }
end

function M.transform_min_max(rows, scaler)
    local width = validate_matrix(rows)
    if width ~= #scaler.minimums then
        error("matrix width must match scaler width")
    end
    local out = {}
    for row_index, row in ipairs(rows) do
        out[row_index] = {}
        for col = 1, width do
            local span = scaler.maximums[col] - scaler.minimums[col]
            out[row_index][col] = span == 0.0 and 0.0 or (row[col] - scaler.minimums[col]) / span
        end
    end
    return out
end

return M
