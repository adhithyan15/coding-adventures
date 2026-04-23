local layout = require("coding_adventures.barcode_layout_1d")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

M.PATTERNS = {
    ["0"] = "bwbWBwBwb", ["1"] = "BwbWbwbwB", ["2"] = "bwBWbwbwB", ["3"] = "BwBWbwbwb",
    ["4"] = "bwbWBwbwB", ["5"] = "BwbWBwbwb", ["6"] = "bwBWBwbwb", ["7"] = "bwbWbwBwB",
    ["8"] = "BwbWbwBwb", ["9"] = "bwBWbwBwb", ["A"] = "BwbwbWbwB", ["B"] = "bwBwbWbwB",
    ["C"] = "BwBwbWbwb", ["D"] = "bwbwBWbwB", ["E"] = "BwbwBWbwb", ["F"] = "bwBwBWbwb",
    ["G"] = "bwbwbWBwB", ["H"] = "BwbwbWBwb", ["I"] = "bwBwbWBwb", ["J"] = "bwbwBWBwb",
    ["K"] = "BwbwbwbWB", ["L"] = "bwBwbwbWB", ["M"] = "BwBwbwbWb", ["N"] = "bwbwBwbWB",
    ["O"] = "BwbwBwbWb", ["P"] = "bwBwBwbWb", ["Q"] = "bwbwbwBWB", ["R"] = "BwbwbwBWb",
    ["S"] = "bwBwbwBWb", ["T"] = "bwbwBwBWb", ["U"] = "BWbwbwbwB", ["V"] = "bWBwbwbwB",
    ["W"] = "BWBwbwbwb", ["X"] = "bWbwBwbwB", ["Y"] = "BWbwBwbwb", ["Z"] = "bWBwBwbwb",
    ["-"] = "bWbwbwBwB", ["."] = "BWbwbwBwb", [" "] = "bWBwbwBwb", ["$"] = "bWbWbWbwb",
    ["/"] = "bWbWbwbWb", ["+"] = "bWbwbWbWb", ["%"] = "bwbWbWbWb", ["*"] = "bWbwBwBwb",
}

local BAR_SPACE_COLORS = {"bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"}

local function width_pattern(pattern)
    return pattern:gsub(".", function(token)
        if token == string.upper(token) then
            return "W"
        end
        return "N"
    end)
end

function M.normalize_code39(data)
    local normalized = string.upper(data)
    for index = 1, #normalized do
        local ch = normalized:sub(index, index)
        if ch == "*" then
            error('input must not contain "*" because it is reserved for start/stop')
        end
        if not M.PATTERNS[ch] then
            error(string.format('invalid character: "%s" is not supported by Code 39', ch))
        end
    end
    return normalized
end

function M.encode_code39_char(char)
    local pattern = M.PATTERNS[char]
    if not pattern then
        error(string.format('unknown Code 39 character: "%s"', char))
    end

    return {
        char = char,
        is_start_stop = char == "*",
        pattern = width_pattern(pattern),
    }
end

function M.encode_code39(data)
    local normalized = M.normalize_code39(data)
    local encoded = {}
    local wrapped = "*" .. normalized .. "*"
    for index = 1, #wrapped do
        encoded[#encoded + 1] = M.encode_code39_char(wrapped:sub(index, index))
    end
    return encoded
end

function M.expand_code39_runs(data)
    local encoded = M.encode_code39(data)
    local runs = {}

    for source_index, encoded_char in ipairs(encoded) do
        local char_runs = layout.runs_from_width_pattern(
            encoded_char.pattern,
            BAR_SPACE_COLORS,
            {
                source_char = encoded_char.char,
                source_index = source_index - 1,
            }
        )
        for _, run in ipairs(char_runs) do
            runs[#runs + 1] = run
        end
        if source_index < #encoded then
            runs[#runs + 1] = {
                color = "space",
                modules = 1,
                source_char = encoded_char.char,
                source_index = source_index - 1,
                role = "inter-character-gap",
                metadata = {},
            }
        end
    end

    return runs
end

function M.layout_code39(data, config)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_code39(data)
    return layout.layout_barcode_1d(
        M.expand_code39_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "code39",
                data = normalized,
            },
        }
    )
end

function M.draw_code39(data, config)
    return M.layout_code39(data, config)
end

return M
