local layout = require("coding_adventures.barcode_layout_1d")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local START_B = 104
local STOP = 106

local PATTERNS = {
    "11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
    "10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
    "11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
    "10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
    "11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
    "11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
    "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
    "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
    "11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
    "10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
    "11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
    "11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
    "11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
    "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
    "10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
    "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
    "10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
    "10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
    "11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
    "10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
    "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
    "11010011100", "1100011101011",
}

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

function M.normalize_code128_b(data)
    for index = 1, #data do
        local code = string.byte(data, index)
        if code < 32 or code > 126 then
            error("Code 128 Code Set B supports printable ASCII characters only")
        end
    end
    return data
end

function M.value_for_code128_b_char(char)
    return string.byte(char) - 32
end

function M.compute_code128_checksum(values)
    local sum = START_B
    for index, value in ipairs(values) do
        sum = sum + value * index
    end
    return sum % 103
end

function M.encode_code128_b(data)
    local normalized = M.normalize_code128_b(data)
    local encoded = {
        {
            label = "Start B",
            value = START_B,
            pattern = PATTERNS[START_B + 1],
            source_index = -1,
            role = "start",
        },
    }
    local values = {}

    for index = 1, #normalized do
        local char = normalized:sub(index, index)
        local value = M.value_for_code128_b_char(char)
        values[#values + 1] = value
        encoded[#encoded + 1] = {
            label = char,
            value = value,
            pattern = PATTERNS[value + 1],
            source_index = index - 1,
            role = "data",
        }
    end

    local checksum = M.compute_code128_checksum(values)
    encoded[#encoded + 1] = {
        label = "Checksum " .. tostring(checksum),
        value = checksum,
        pattern = PATTERNS[checksum + 1],
        source_index = #normalized,
        role = "check",
    }
    encoded[#encoded + 1] = {
        label = "Stop",
        value = STOP,
        pattern = PATTERNS[STOP + 1],
        source_index = #normalized + 1,
        role = "stop",
    }

    return encoded
end

function M.expand_code128_runs(data)
    local encoded = M.encode_code128_b(data)
    local runs = {}

    for _, symbol in ipairs(encoded) do
        local segment_runs = layout.runs_from_binary_pattern(
            symbol.pattern,
            {
                source_char = symbol.label,
                source_index = symbol.source_index,
            }
        )
        for _, run in ipairs(retag_runs(segment_runs, symbol.role)) do
            runs[#runs + 1] = run
        end
    end

    return runs
end

function M.layout_code128(data, config)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_code128_b(data)
    local encoded = M.encode_code128_b(normalized)
    local checksum = encoded[#encoded - 1].value

    return layout.layout_barcode_1d(
        M.expand_code128_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "code128",
                code_set = "B",
                checksum = checksum,
            },
        }
    )
end

function M.draw_code128(data, config)
    return M.layout_code128(data, config)
end

return M
