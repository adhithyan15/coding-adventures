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
    G = {"0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"},
    R = {"1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"},
}

local LEFT_PARITY_PATTERNS = {"LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"}

local function assert_digits(data, expected_a, expected_b)
    if not data:match("^%d+$") then
        error("EAN-13 input must contain digits only")
    end
    local length = #data
    if length ~= expected_a and length ~= expected_b then
        error("EAN-13 input must contain 12 digits or 13 digits")
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

function M.compute_ean_13_check_digit(payload12)
    assert_digits(payload12, 12, 12)
    local total = 0
    local position = 0
    for index = #payload12, 1, -1 do
        local digit = tonumber(payload12:sub(index, index))
        local multiplier = (position % 2 == 0) and 3 or 1
        total = total + digit * multiplier
        position = position + 1
    end
    return tostring((10 - (total % 10)) % 10)
end

function M.normalize_ean_13(data)
    assert_digits(data, 12, 13)
    if #data == 12 then
        return data .. M.compute_ean_13_check_digit(data)
    end

    local expected = M.compute_ean_13_check_digit(data:sub(1, 12))
    local actual = data:sub(13, 13)
    if expected ~= actual then
        error(string.format("Invalid EAN-13 check digit: expected %s but received %s", expected, actual))
    end
    return data
end

function M.left_parity_pattern(data)
    local normalized = M.normalize_ean_13(data)
    return LEFT_PARITY_PATTERNS[tonumber(normalized:sub(1, 1)) + 1]
end

function M.encode_ean_13(data)
    local normalized = M.normalize_ean_13(data)
    local parity = M.left_parity_pattern(normalized)
    local encoded = {}

    for offset = 1, 6 do
        local digit = normalized:sub(offset + 1, offset + 1)
        local encoding = parity:sub(offset, offset)
        encoded[#encoded + 1] = {
            digit = digit,
            encoding = encoding,
            pattern = DIGIT_PATTERNS[encoding][tonumber(digit) + 1],
            source_index = offset,
            role = "data",
        }
    end

    for offset = 1, 6 do
        local digit = normalized:sub(offset + 7, offset + 7)
        encoded[#encoded + 1] = {
            digit = digit,
            encoding = "R",
            pattern = DIGIT_PATTERNS.R[tonumber(digit) + 1],
            source_index = offset + 6,
            role = offset == 6 and "check" or "data",
        }
    end

    return encoded
end

function M.expand_ean_13_runs(data)
    local encoded = M.encode_ean_13(data)
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

function M.layout_ean_13(data, config)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_ean_13(data)
    return layout.layout_barcode_1d(
        M.expand_ean_13_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "ean-13",
                leading_digit = normalized:sub(1, 1),
                left_parity = M.left_parity_pattern(normalized),
                content_modules = 95,
            },
        }
    )
end

function M.draw_ean_13(data, config)
    return M.layout_ean_13(data, config)
end

return M
