local layout = require("coding_adventures.barcode_layout_1d")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local SIDE_GUARD = "101"
local CENTER_GUARD = "01010"

local DIGIT_PATTERNS = {
    L = {"0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"},
    R = {"1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"},
}

local function assert_digits(data, expected_a, expected_b)
    if not data:match("^%d+$") then
        error("UPC-A input must contain digits only")
    end
    local length = #data
    if length ~= expected_a and length ~= expected_b then
        error("UPC-A input must contain 11 digits or 12 digits")
    end
end

local function copy_metadata(metadata)
    local result = {}
    if metadata == nil then
        return result
    end
    for key, value in pairs(metadata) do
        result[key] = value
    end
    return result
end

local function retag_runs(runs, role)
    local result = {}
    for index, run in ipairs(runs) do
        result[index] = {
            color = run.color,
            modules = run.modules,
            source_char = run.source_char,
            source_index = run.source_index,
            role = role,
            metadata = copy_metadata(run.metadata),
        }
    end
    return result
end

function M.compute_upc_a_check_digit(payload11)
    assert_digits(payload11, 11, 11)
    local odd_sum = 0
    local even_sum = 0

    for index = 1, #payload11 do
        local digit = tonumber(payload11:sub(index, index))
        if (index % 2) == 1 then
            odd_sum = odd_sum + digit
        else
            even_sum = even_sum + digit
        end
    end

    return tostring((10 - (((odd_sum * 3) + even_sum) % 10)) % 10)
end

function M.normalize_upc_a(data)
    assert_digits(data, 11, 12)
    if #data == 11 then
        return data .. M.compute_upc_a_check_digit(data)
    end

    local expected = M.compute_upc_a_check_digit(data:sub(1, 11))
    local actual = data:sub(12, 12)
    if expected ~= actual then
        error(string.format("Invalid UPC-A check digit: expected %s but received %s", expected, actual))
    end
    return data
end

function M.encode_upc_a(data)
    local normalized = M.normalize_upc_a(data)
    local encoded = {}

    for index = 1, #normalized do
        local digit = normalized:sub(index, index)
        local encoding = index <= 6 and "L" or "R"
        encoded[#encoded + 1] = {
            digit = digit,
            encoding = encoding,
            pattern = DIGIT_PATTERNS[encoding][tonumber(digit) + 1],
            source_index = index - 1,
            role = index == #normalized and "check" or "data",
        }
    end

    return encoded
end

function M.expand_upc_a_runs(data)
    local encoded = M.encode_upc_a(data)
    local runs = {}

    local function append_runs(pattern, source_char, source_index, role)
        local pattern_runs = layout.runs_from_binary_pattern(
            pattern,
            {
                source_char = source_char,
                source_index = source_index,
            }
        )
        for _, run in ipairs(retag_runs(pattern_runs, role)) do
            runs[#runs + 1] = run
        end
    end

    append_runs(SIDE_GUARD, "start", -1, "guard")

    for index = 1, 6 do
        local entry = encoded[index]
        append_runs(entry.pattern, entry.digit, entry.source_index, entry.role)
    end

    append_runs(CENTER_GUARD, "center", -2, "guard")

    for index = 7, #encoded do
        local entry = encoded[index]
        append_runs(entry.pattern, entry.digit, entry.source_index, entry.role)
    end

    append_runs(SIDE_GUARD, "end", -3, "guard")
    return runs
end

function M.layout_upc_a(data, config)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_upc_a(data)
    return layout.layout_barcode_1d(
        M.expand_upc_a_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "upc-a",
                content_modules = 95,
            },
        }
    )
end

function M.draw_upc_a(data, config)
    return M.layout_upc_a(data, config)
end

return M
