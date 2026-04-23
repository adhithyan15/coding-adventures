local layout = require("coding_adventures.barcode_layout_1d")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local START_PATTERN = "1010"
local STOP_PATTERN = "11101"

local DIGIT_PATTERNS = {"00110", "10001", "01001", "11000", "00101", "10100", "01100", "00011", "10010", "01010"}

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

function M.normalize_itf(data)
    if not data:match("^%d+$") then
        error("ITF input must contain digits only")
    end
    if #data == 0 or (#data % 2) ~= 0 then
        error("ITF input must contain an even number of digits")
    end
    return data
end

function M.encode_itf(data)
    local normalized = M.normalize_itf(data)
    local encoded = {}

    for index = 1, #normalized, 2 do
        local pair = normalized:sub(index, index + 1)
        local bar_pattern = DIGIT_PATTERNS[tonumber(pair:sub(1, 1)) + 1]
        local space_pattern = DIGIT_PATTERNS[tonumber(pair:sub(2, 2)) + 1]
        local binary_pattern = {}

        for offset = 1, #bar_pattern do
            binary_pattern[#binary_pattern + 1] = (bar_pattern:sub(offset, offset) == "1") and "111" or "1"
            binary_pattern[#binary_pattern + 1] = (space_pattern:sub(offset, offset) == "1") and "000" or "0"
        end

        encoded[#encoded + 1] = {
            pair = pair,
            bar_pattern = bar_pattern,
            space_pattern = space_pattern,
            binary_pattern = table.concat(binary_pattern),
            source_index = (#encoded),
        }
    end

    return encoded
end

function M.expand_itf_runs(data)
    local encoded = M.encode_itf(data)
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

    append_runs(START_PATTERN, "start", -1, "start")

    for _, entry in ipairs(encoded) do
        append_runs(entry.binary_pattern, entry.pair, entry.source_index, "data")
    end

    append_runs(STOP_PATTERN, "stop", -2, "stop")
    return runs
end

function M.layout_itf(data, config)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_itf(data)
    return layout.layout_barcode_1d(
        M.expand_itf_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "itf",
                pair_count = #normalized / 2,
            },
        }
    )
end

function M.draw_itf(data, config)
    return M.layout_itf(data, config)
end

return M
