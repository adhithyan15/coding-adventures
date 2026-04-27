local layout = require("coding_adventures.barcode_layout_1d")

local M = {}

M.VERSION = "0.1.0"

M.DEFAULT_LAYOUT_CONFIG = {
    module_unit = 4,
    bar_height = 120,
    quiet_zone_modules = 10,
}

M.DEFAULT_RENDER_CONFIG = M.DEFAULT_LAYOUT_CONFIG

local GUARDS = {
    A = true,
    B = true,
    C = true,
    D = true,
}

M.PATTERNS = {
    ["0"] = "101010011",
    ["1"] = "101011001",
    ["2"] = "101001011",
    ["3"] = "110010101",
    ["4"] = "101101001",
    ["5"] = "110101001",
    ["6"] = "100101011",
    ["7"] = "100101101",
    ["8"] = "100110101",
    ["9"] = "110100101",
    ["-"] = "101001101",
    ["$"] = "101100101",
    [":"] = "1101011011",
    ["/"] = "1101101011",
    ["."] = "1101101101",
    ["+"] = "1011011011",
    ["A"] = "1011001001",
    ["B"] = "1001001011",
    ["C"] = "1010010011",
    ["D"] = "1010011001",
}

local function is_guard(char)
    return GUARDS[char] == true
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

local function assert_body_chars(body)
    for index = 1, #body do
        local char = body:sub(index, index)
        if not M.PATTERNS[char] or is_guard(char) then
            error(string.format('Invalid Codabar body character "%s"', char))
        end
    end
end

function M.normalize_codabar(data, start, stop)
    local normalized = string.upper(data)
    if #normalized >= 2 and is_guard(normalized:sub(1, 1)) and is_guard(normalized:sub(#normalized, #normalized)) then
        assert_body_chars(normalized:sub(2, #normalized - 1))
        return normalized
    end

    start = start or "A"
    stop = stop or "A"
    if not is_guard(start) or not is_guard(stop) then
        error("Codabar guards must be one of A, B, C, or D")
    end

    assert_body_chars(normalized)
    return start .. normalized .. stop
end

function M.encode_codabar(data, start, stop)
    local normalized = M.normalize_codabar(data, start, stop)
    local encoded = {}

    for index = 1, #normalized do
        local char = normalized:sub(index, index)
        local role = "data"
        if index == 1 then
            role = "start"
        elseif index == #normalized then
            role = "stop"
        end

        encoded[#encoded + 1] = {
            char = char,
            pattern = M.PATTERNS[char],
            source_index = index - 1,
            role = role,
        }
    end

    return encoded
end

function M.expand_codabar_runs(data, start, stop)
    local encoded = M.encode_codabar(data, start, stop)
    local runs = {}

    for index, symbol in ipairs(encoded) do
        local symbol_runs = layout.runs_from_binary_pattern(
            symbol.pattern,
            {
                source_char = symbol.char,
                source_index = symbol.source_index,
            }
        )

        for _, run in ipairs(retag_runs(symbol_runs, symbol.role)) do
            runs[#runs + 1] = run
        end

        if index < #encoded then
            runs[#runs + 1] = {
                color = "space",
                modules = 1,
                source_char = symbol.char,
                source_index = symbol.source_index,
                role = "inter-character-gap",
                metadata = {},
            }
        end
    end

    return runs
end

function M.layout_codabar(data, config, start, stop)
    config = config or M.DEFAULT_LAYOUT_CONFIG
    local normalized = M.normalize_codabar(data, start, stop)
    return layout.layout_barcode_1d(
        M.expand_codabar_runs(normalized),
        config,
        {
            fill = "#000000",
            background = "#ffffff",
            metadata = {
                symbology = "codabar",
                start = normalized:sub(1, 1),
                stop = normalized:sub(#normalized, #normalized),
            },
        }
    )
end

function M.draw_codabar(data, config, start, stop)
    return M.layout_codabar(data, config, start, stop)
end

return M
